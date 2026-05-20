use super::engine::{ExecutionStrategy, WorkflowEngine, WorkflowExecutionContext, WorkflowStatus};
use super::nodes::NodeType;
use crate::core::models::WorkflowRecord;
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::message_bus::routing::TeamBusRouter;
use std::collections::HashMap;
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
                    let parsed = WorkflowEngine::parse_workflow(&wf.definition);
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

        // 2. Worker result listener loop
        let engine_listener = engine.clone();
        let instance_id = team_instance_id.unwrap_or_else(|| "sdg-instance-123".to_string());
        runtime.spawn(async move {
            let mut rx = team_bus
                .register_member(&instance_id, "iflow_engine", "System")
                .await;

            while let Some(msg) = rx.recv().await {
                if let Some(metadata) = msg.metadata {
                    if metadata.starts_with("iflow_result:") {
                        let parts: Vec<&str> = metadata.split(':').collect();
                        if parts.len() >= 3 {
                            let execution_id = parts[1];
                            let node_id = parts[2];
                            if let Ok(_state) = engine_listener.resolve_agent_task(
                                execution_id,
                                node_id,
                                msg.content,
                            ) {
                                // Resume execution
                                let _ = Self::run_execution(&engine_listener, execution_id).await;
                            }
                        }
                    }
                }
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

    pub fn upsert_workflow(db: Arc<dyn DatabasePort>, wf: &WorkflowRecord) -> anyhow::Result<()> {
        db.upsert_workflow(wf)
    }
}
