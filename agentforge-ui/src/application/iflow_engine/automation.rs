use super::engine::{ExecutionStrategy, WorkflowEngine, WorkflowExecutionContext, WorkflowStatus};
use super::nodes::NodeType;
use crate::core::models::WorkflowRecord;
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::message_bus::routing::TeamBusRouter;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::time::{Duration, Instant};

pub struct IFlowAutomation;

impl IFlowAutomation {
    pub fn start(
        db: Arc<dyn DatabasePort>,
        team_bus: Arc<TeamBusRouter>,
        runtime: Arc<tokio::runtime::Runtime>,
    ) {
        let team_instance_id = db
            .list_instances()
            .ok()
            .and_then(|mut v| v.pop())
            .map(|i| i.id);

        let ctx = Arc::new(WorkflowExecutionContext {
            db: db.clone(),
            team_bus: team_bus.clone(),
            team_instance_id: team_instance_id.clone(),
        });
        let engine = Arc::new(WorkflowEngine::new_with_context(ctx));

        // 1. Cron background loop
        let engine_cron = engine.clone();
        let db_cron = db.clone();
        runtime.spawn(async move {
            let mut next_runs: HashMap<String, Instant> = HashMap::new();
            loop {
                let workflows = db_cron.list_workflows().unwrap_or_default();
                for wf in workflows {
                    if wf.run_id.is_none() || wf.activation_status != "active" {
                        continue;
                    }
                    let parsed = WorkflowEngine::parse_validated_workflow(&wf.definition);
                    let Ok(workflow) = parsed else { continue };

                    let start_node = workflow.nodes.get(&workflow.start_node_id);
                    let Some(start_node) = start_node else {
                        continue;
                    };

                    let interval_ms = match &start_node.node_type {
                        NodeType::CronTrigger { interval_ms } => Some(*interval_ms),
                        _ => None,
                    };
                    let Some(interval_ms) = interval_ms else {
                        continue;
                    };

                    engine_cron.register_workflow(workflow.clone());
                    let entry = next_runs
                        .entry(workflow.id.clone())
                        .or_insert_with(|| Instant::now() + Duration::from_millis(interval_ms));
                    if Instant::now() >= *entry {
                        if let Ok(execution_id) = engine_cron.start_workflow(
                            &workflow.id,
                            ExecutionStrategy::Parallel,
                            Default::default(),
                        ) {
                            let _ = Self::run_execution(&engine_cron, &execution_id).await;
                        }
                        *entry = Instant::now() + Duration::from_millis(interval_ms);
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });

        // 2. Worker result listeners follow every persisted instance. Workers publish
        // iFlow completion as system broadcasts, so direct-member registration cannot receive it.
        let engine_listener = engine.clone();
        let db_listener = db.clone();
        let bus_listener = team_bus.clone();
        runtime.spawn(async move {
            let mut listening = HashSet::new();
            loop {
                for instance in db_listener.list_instances().unwrap_or_default() {
                    if listening.insert(instance.id.clone()) {
                        let mut rx = bus_listener.subscribe_broadcast(&instance.id).await;
                        let engine_for_instance = engine_listener.clone();
                        tokio::spawn(async move {
                            while let Ok(msg) = rx.recv().await {
                                if let Some(metadata) = msg.metadata {
                                    if metadata.starts_with("iflow_result:") {
                                        let parts: Vec<&str> = metadata.split(':').collect();
                                        if parts.len() >= 3 {
                                            let execution_id = parts[1];
                                            let node_id = parts[2];
                                            if engine_for_instance
                                                .resolve_agent_task(
                                                    execution_id,
                                                    node_id,
                                                    msg.content,
                                                )
                                                .is_ok()
                                            {
                                                let _ = Self::run_execution(
                                                    &engine_for_instance,
                                                    execution_id,
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }

    async fn run_execution(engine: &WorkflowEngine, execution_id: &str) -> Result<(), String> {
        loop {
            let state = engine.step_execution(execution_id).await?;
            match state.status {
                WorkflowStatus::Running => {
                    if state.current_nodes.is_empty() && !state.pending_delays.is_empty() {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    continue;
                }
                WorkflowStatus::Pending => continue,
                WorkflowStatus::Paused => return Ok(()),
                WorkflowStatus::Completed => return Ok(()),
                WorkflowStatus::Failed(e) => return Err(e),
            }
        }
    }

    pub fn dispatch_persisted_workflow(
        db: Arc<dyn DatabasePort>,
        team_bus: Arc<TeamBusRouter>,
        runtime: Arc<tokio::runtime::Runtime>,
        workflow_id: &str,
    ) -> Result<String, String> {
        let record = db
            .get_workflow(workflow_id)
            .map_err(|error| error.to_string())?
            .ok_or("Workflow was not persisted")?;
        let workflow = WorkflowEngine::parse_validated_workflow(&record.definition)?;
        let ctx = Arc::new(WorkflowExecutionContext {
            db,
            team_bus,
            team_instance_id: workflow.instance_id.clone(),
        });
        let engine = Arc::new(WorkflowEngine::new_with_context(ctx));
        engine.register_workflow(workflow);
        let execution_id =
            engine.start_workflow(workflow_id, ExecutionStrategy::Serial, Default::default())?;
        let engine_for_run = engine.clone();
        let execution_for_run = execution_id.clone();
        runtime.spawn(async move {
            let _ = Self::run_execution(&engine_for_run, &execution_for_run).await;
        });
        Ok(execution_id)
    }

    pub fn resume_approved_run(
        db: Arc<dyn DatabasePort>,
        team_bus: Arc<TeamBusRouter>,
        runtime: Arc<tokio::runtime::Runtime>,
        run_id: &str,
    ) -> Result<(), String> {
        let version = db
            .get_latest_workflow_version_for_run(run_id)
            .map_err(|error| error.to_string())?
            .ok_or("Approved run has no workflow version")?;
        let execution = db
            .get_latest_workflow_execution_for_version(&version.id)
            .map_err(|error| error.to_string())?
            .ok_or("Approved run has no workflow execution")?;
        let workflow = WorkflowEngine::parse_validated_workflow(&version.definition_json)?;
        let ctx = Arc::new(WorkflowExecutionContext {
            db,
            team_bus,
            team_instance_id: workflow.instance_id.clone(),
        });
        let engine = Arc::new(WorkflowEngine::new_with_context(ctx));
        engine.register_workflow(workflow);
        runtime.spawn(async move {
            let _ = engine.redispatch_pending_agent_tasks(&execution.id).await;
        });
        Ok(())
    }

    pub fn reject_waiting_run(
        db: Arc<dyn DatabasePort>,
        team_bus: Arc<TeamBusRouter>,
        run_id: &str,
    ) -> Result<(), String> {
        let version = db
            .get_latest_workflow_version_for_run(run_id)
            .map_err(|error| error.to_string())?
            .ok_or("Rejected run has no workflow version")?;
        let execution = db
            .get_latest_workflow_execution_for_version(&version.id)
            .map_err(|error| error.to_string())?
            .ok_or("Rejected run has no workflow execution")?;
        let workflow = WorkflowEngine::parse_validated_workflow(&version.definition_json)?;
        let ctx = Arc::new(WorkflowExecutionContext {
            db,
            team_bus,
            team_instance_id: workflow.instance_id.clone(),
        });
        let engine = WorkflowEngine::new_with_context(ctx);
        engine.register_workflow(workflow);
        let mut state = engine
            .get_state(&execution.id)
            .ok_or("Rejected run execution state was not found")?;
        state.pending_agent_tasks.clear();
        state.status = WorkflowStatus::Failed("Tool approval rejected by operator.".to_string());
        engine.persist_state(&state)
    }

    pub fn upsert_workflow(db: Arc<dyn DatabasePort>, wf: &WorkflowRecord) -> anyhow::Result<()> {
        db.upsert_workflow(wf)
    }
}
