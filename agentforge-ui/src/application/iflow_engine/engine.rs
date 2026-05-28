use super::nodes::{Node, NodeType, WorkflowData};
use crate::core::traits::database::DatabasePort;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub nodes: HashMap<String, Node>,
    pub start_node_id: String,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    Serial,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub execution_id: String,
    pub workflow_id: String,
    pub current_nodes: Vec<String>,
    pub completed_nodes: HashSet<String>,
    pub data: WorkflowData,
    pub status: WorkflowStatus,
    pub strategy: ExecutionStrategy,
    pub pending_review: Option<String>,
    pub pending_agent_tasks: HashSet<String>,
    #[serde(default)]
    pub pending_delays: HashMap<String, u64>,
}

#[derive(Clone)]
pub struct WorkflowExecutionContext {
    pub db: Arc<dyn DatabasePort>,
    pub team_bus: Arc<crate::infrastructure::message_bus::routing::TeamBusRouter>,
    pub team_instance_id: Option<String>,
}

#[derive(Clone)]
pub struct WorkflowEngine {
    state_store: Arc<RwLock<HashMap<String, WorkflowState>>>,
    workflow_store: Arc<RwLock<HashMap<String, Workflow>>>,
    execution_context: Option<Arc<WorkflowExecutionContext>>,
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            state_store: Arc::new(RwLock::new(HashMap::new())),
            workflow_store: Arc::new(RwLock::new(HashMap::new())),
            execution_context: None,
        }
    }

    pub fn new_with_context(ctx: Arc<WorkflowExecutionContext>) -> Self {
        Self {
            state_store: Arc::new(RwLock::new(HashMap::new())),
            workflow_store: Arc::new(RwLock::new(HashMap::new())),
            execution_context: Some(ctx),
        }
    }

    pub fn parse_workflow(json: &str) -> Result<Workflow, serde_json::Error> {
        let mut wf: Workflow = serde_json::from_str(json)?;
        for node in wf.nodes.values_mut() {
            node.sanitize_command();
        }
        Ok(wf)
    }

    pub fn validate_workflow(workflow: &Workflow) -> Result<(), String> {
        Self::validate_workflow_contract(workflow, true)
    }

    pub fn validate_workflow_draft(workflow: &Workflow) -> Result<(), String> {
        Self::validate_workflow_contract(workflow, false)
    }

    fn validate_workflow_contract(
        workflow: &Workflow,
        require_instance_scope: bool,
    ) -> Result<(), String> {
        if workflow.nodes.is_empty() {
            return Err("Workflow must contain at least one node.".to_string());
        }
        let start = workflow
            .nodes
            .get(&workflow.start_node_id)
            .ok_or_else(|| "Workflow start node does not exist.".to_string())?;
        if !matches!(
            start.node_type,
            NodeType::Start | NodeType::CronTrigger { .. }
        ) {
            return Err(
                "Workflow start_node_id must reference a Start or CronTrigger node.".to_string(),
            );
        }
        if require_instance_scope
            && workflow
                .instance_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            return Err("Workflow must be scoped to an instance before execution.".to_string());
        }

        let mut has_end = false;
        for (node_id, node) in &workflow.nodes {
            if matches!(
                node.node_type,
                NodeType::SystemCommand { .. } | NodeType::HttpRequest { .. }
            ) {
                return Err(format!(
                    "Node {} uses a direct side effect that is disabled until the governed execution gateway is implemented.",
                    node_id
                ));
            }
            if matches!(node.node_type, NodeType::End) {
                has_end = true;
            }
            if let NodeType::AgentTask {
                agent_id,
                instruction,
                ..
            } = &node.node_type
            {
                if agent_id.trim().is_empty() || instruction.trim().is_empty() {
                    return Err(format!(
                        "Agent task node {} requires an agent and instruction.",
                        node_id
                    ));
                }
            }

            let mut destinations = node.next_nodes.clone();
            match &node.node_type {
                NodeType::Decision {
                    true_next,
                    false_next,
                    ..
                } => {
                    destinations.push(true_next.clone());
                    destinations.push(false_next.clone());
                }
                NodeType::HumanReview {
                    approved_next,
                    rejected_next,
                    ..
                } => {
                    destinations.push(approved_next.clone());
                    destinations.push(rejected_next.clone());
                }
                _ => {}
            }
            for destination in destinations {
                if destination.trim().is_empty() || !workflow.nodes.contains_key(&destination) {
                    return Err(format!(
                        "Node {} references a missing destination node.",
                        node_id
                    ));
                }
            }
        }
        if !has_end {
            return Err("Workflow must contain an End node.".to_string());
        }
        Ok(())
    }

    pub fn parse_validated_workflow(json: &str) -> Result<Workflow, String> {
        let workflow = Self::parse_workflow(json).map_err(|error| error.to_string())?;
        Self::validate_workflow(&workflow)?;
        Ok(workflow)
    }

    pub fn parse_validated_draft(json: &str) -> Result<Workflow, String> {
        let workflow = Self::parse_workflow(json).map_err(|error| error.to_string())?;
        Self::validate_workflow_draft(&workflow)?;
        Ok(workflow)
    }

    pub fn register_workflow(&self, workflow: Workflow) {
        let mut store = self.workflow_store.write().unwrap();
        store.insert(workflow.id.clone(), workflow);
    }

    pub fn get_workflow(&self, workflow_id: &str) -> Option<Workflow> {
        let store = self.workflow_store.read().unwrap();
        store.get(workflow_id).cloned()
    }

    pub fn start_workflow(
        &self,
        workflow_id: &str,
        strategy: ExecutionStrategy,
        initial_data: WorkflowData,
    ) -> Result<String, String> {
        let workflow = {
            let store = self.workflow_store.read().unwrap();
            store
                .get(workflow_id)
                .cloned()
                .ok_or_else(|| "Workflow not found".to_string())?
        };
        Self::validate_workflow(&workflow)?;

        let execution_id = Uuid::new_v4().to_string();
        let mut data = initial_data;
        if let Some(team_id) = &workflow.team_id {
            data.set("team_id", serde_json::Value::String(team_id.clone()));
        }
        if let Some(instance_id) = &workflow.instance_id {
            data.set(
                "instance_id",
                serde_json::Value::String(instance_id.clone()),
            );
        }

        let state = WorkflowState {
            execution_id: execution_id.clone(),
            workflow_id: workflow.id.clone(),
            current_nodes: vec![workflow.start_node_id.clone()],
            completed_nodes: HashSet::new(),
            data,
            status: WorkflowStatus::Running,
            strategy,
            pending_review: None,
            pending_agent_tasks: HashSet::new(),
            pending_delays: HashMap::new(),
        };

        self.persist_state(&state)?;
        Ok(execution_id)
    }

    pub fn persist_state(&self, state: &WorkflowState) -> Result<(), String> {
        let mut store = self.state_store.write().unwrap();
        store.insert(state.execution_id.clone(), state.clone());
        if let Some(ctx) = &self.execution_context {
            ctx.db
                .save_workflow_state(state)
                .map_err(|error| format!("Unable to persist workflow state: {}", error))?;
            if let Ok(Some(version)) = ctx
                .db
                .get_latest_workflow_version_for_workflow(&state.workflow_id)
            {
                if let Some(run_id) = version.run_id {
                    let status = match &state.status {
                        WorkflowStatus::Pending => "pending".to_string(),
                        WorkflowStatus::Running => "running".to_string(),
                        WorkflowStatus::Paused => "paused".to_string(),
                        WorkflowStatus::Completed => "completed".to_string(),
                        WorkflowStatus::Failed(reason) => format!("failed: {}", reason),
                    };
                    let state_json =
                        serde_json::to_string(state).map_err(|error| error.to_string())?;
                    ctx.db
                        .save_workflow_execution(&crate::core::models::WorkflowExecutionRecord {
                            id: state.execution_id.clone(),
                            workflow_version_id: version.id,
                            run_id: run_id.clone(),
                            status: status.clone(),
                            state_json,
                            updated_at: chrono::Utc::now().to_rfc3339(),
                        })
                        .map_err(|error| {
                            format!("Unable to persist workflow execution: {}", error)
                        })?;
                    let run_status = match &state.status {
                        WorkflowStatus::Pending | WorkflowStatus::Running => "running",
                        WorkflowStatus::Paused => "paused",
                        WorkflowStatus::Completed => "completed",
                        WorkflowStatus::Failed(_) => "failed",
                    };
                    ctx.db
                        .update_orchestration_run_status(&run_id, run_status, None)
                        .map_err(|error| {
                            format!("Unable to update run from workflow execution: {}", error)
                        })?;
                    let _ = ctx
                        .db
                        .insert_run_event(&crate::core::models::RunEventRecord {
                            id: uuid::Uuid::new_v4().to_string(),
                            run_id,
                            event_type: "workflow_execution_state".to_string(),
                            actor_type: "system".to_string(),
                            actor_id: None,
                            task_id: None,
                            payload: Some(format!(
                                "Execution {} status {}",
                                state.execution_id, status
                            )),
                            created_at: chrono::Utc::now().to_rfc3339(),
                        });
                }
            }
        }
        Ok(())
    }

    pub fn get_state(&self, execution_id: &str) -> Option<WorkflowState> {
        let store = self.state_store.read().unwrap();
        if let Some(state) = store.get(execution_id).cloned() {
            return Some(state);
        }
        // Fallback: try to restore from database
        drop(store);
        if let Some(ctx) = &self.execution_context {
            if let Ok(Some(state)) = ctx.db.load_workflow_state(execution_id) {
                if self.get_workflow(&state.workflow_id).is_none() {
                    if let Ok(Some(record)) = ctx.db.get_workflow(&state.workflow_id) {
                        if let Ok(workflow) = Self::parse_validated_workflow(&record.definition) {
                            self.register_workflow(workflow);
                        }
                    }
                }
                let mut store = self.state_store.write().unwrap();
                store.insert(execution_id.to_string(), state.clone());
                return Some(state);
            }
        }
        None
    }

    pub async fn step_execution(&self, execution_id: &str) -> Result<WorkflowState, String> {
        let mut state = self.get_state(execution_id).ok_or("Execution not found")?;

        if state.status != WorkflowStatus::Running {
            return Ok(state);
        }

        let workflow = {
            let store = self.workflow_store.read().unwrap();
            store
                .get(&state.workflow_id)
                .cloned()
                .ok_or("Workflow not found")?
        };

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut ready_delays = Vec::new();
        for (node_id, wake_time) in &state.pending_delays {
            if now_ms >= *wake_time {
                ready_delays.push(node_id.clone());
            }
        }
        for id in ready_delays {
            state.pending_delays.remove(&id);
            if let Some(node) = workflow.nodes.get(&id) {
                state.current_nodes.extend(node.next_nodes.clone());
            }
        }

        if state.current_nodes.is_empty() {
            if state.pending_delays.is_empty() && state.pending_agent_tasks.is_empty() {
                state.status = WorkflowStatus::Completed;
            }
            self.persist_state(&state)?;
            return Ok(state);
        }

        match state.strategy {
            ExecutionStrategy::Serial => {
                let node_id = state.current_nodes.remove(0);
                self.execute_node(&workflow, &mut state, &node_id).await?;
            }
            ExecutionStrategy::Parallel => {
                let nodes_to_execute = std::mem::take(&mut state.current_nodes);
                for node_id in nodes_to_execute {
                    self.execute_node(&workflow, &mut state, &node_id).await?;
                }
            }
        }

        if state.current_nodes.is_empty() && state.status == WorkflowStatus::Running {
            if state.pending_delays.is_empty() && state.pending_agent_tasks.is_empty() {
                state.status = WorkflowStatus::Completed;
            }
        }

        self.persist_state(&state)?;
        Ok(state)
    }

    async fn execute_node(
        &self,
        workflow: &Workflow,
        state: &mut WorkflowState,
        node_id: &str,
    ) -> Result<(), String> {
        let node = workflow
            .nodes
            .get(node_id)
            .ok_or(format!("Node {} not found", node_id))?;

        match &node.node_type {
            NodeType::Start => {
                state.current_nodes.extend(node.next_nodes.clone());
            }
            NodeType::CronTrigger { .. } => {
                state.current_nodes.extend(node.next_nodes.clone());
            }
            NodeType::End => {
                state.status = WorkflowStatus::Completed;
            }
            NodeType::AgentTask {
                agent_id,
                instruction,
                input_vars,
                output_var: _,
            } => {
                let mut prompt = format!("Task Instruction:\n{}\n\nInputs:\n", instruction);
                for var in input_vars {
                    if let Some(val) = state.data.get(var) {
                        prompt.push_str(&format!("{}: {}\n", var, val));
                    }
                }

                if let Some(ctx) = &self.execution_context {
                    let instance_id = workflow
                        .instance_id
                        .clone()
                        .or_else(|| ctx.team_instance_id.clone())
                        .ok_or_else(|| {
                            "Cannot dispatch an iFlow agent task without instance context."
                                .to_string()
                        })?;

                    let msg = crate::infrastructure::message_bus::routing::TeamMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        team_instance_id: instance_id,
                        sender_member_id: "system".to_string(),
                        recipient_member_id: Some(agent_id.clone()),
                        recipient_role: None,
                        message_type:
                            crate::infrastructure::message_bus::routing::MessageType::Direct,
                        content: prompt,
                        metadata: Some(format!(
                            "iflow_dispatch:{}:{}",
                            state.execution_id, node_id
                        )),
                        delivery_status: "delivered".to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                    };

                    let _ = ctx.team_bus.route_message(msg).await;

                    state.status = WorkflowStatus::Paused;
                    state.pending_agent_tasks.insert(node_id.to_string());
                } else {
                    state.current_nodes.extend(node.next_nodes.clone());
                }
            }
            NodeType::SystemCommand { .. } | NodeType::HttpRequest { .. } => {
                return Err(format!(
                    "Node {} uses a disabled direct side effect. Execute mutations through governed agent tools.",
                    node_id
                ));
            }
            NodeType::Transform {
                input_var,
                output_var,
                mode,
            } => {
                let value = state
                    .data
                    .get(input_var)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let out = match mode {
                    super::nodes::TransformMode::Identity => value,
                    super::nodes::TransformMode::ToString => {
                        serde_json::Value::String(value.to_string())
                    }
                };
                state.data.set(output_var.clone(), out);
                state.current_nodes.extend(node.next_nodes.clone());
            }
            NodeType::Decision {
                condition_var,
                true_next,
                false_next,
            } => {
                let condition_met = match state.data.get(condition_var) {
                    Some(serde_json::Value::Bool(b)) => *b,
                    _ => false,
                };

                if condition_met {
                    state.current_nodes.push(true_next.clone());
                } else {
                    state.current_nodes.push(false_next.clone());
                }
            }
            NodeType::HumanReview {
                prompt: _,
                approved_next: _,
                rejected_next: _,
                output_var: _,
            } => {
                state.status = WorkflowStatus::Paused;
                state.pending_review = Some(node_id.to_string());
            }
            NodeType::Merge => {
                state.current_nodes.extend(node.next_nodes.clone());
            }
            NodeType::Delay { duration_ms } => {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                state
                    .pending_delays
                    .insert(node_id.to_string(), now_ms + *duration_ms);
            }
        }

        state.completed_nodes.insert(node_id.to_string());
        Ok(())
    }

    pub fn resolve_agent_task(
        &self,
        execution_id: &str,
        node_id: &str,
        output_text: String,
    ) -> Result<WorkflowState, String> {
        let mut state = self
            .get_state(execution_id)
            .ok_or("Execution not found".to_string())?;

        if state.status != WorkflowStatus::Paused {
            return Err("Execution is not paused".to_string());
        }

        if !state.pending_agent_tasks.contains(node_id) {
            return Err(format!("Node {} is not a pending agent task", node_id));
        }
        state.pending_agent_tasks.remove(node_id);

        let workflow = {
            let store = self.workflow_store.read().unwrap();
            store
                .get(&state.workflow_id)
                .cloned()
                .ok_or("Workflow not found".to_string())?
        };

        let node = workflow
            .nodes
            .get(node_id)
            .ok_or("Node not found".to_string())?;

        if let NodeType::AgentTask { output_var, .. } = &node.node_type {
            if let Some(out_var) = output_var {
                state
                    .data
                    .set(out_var.clone(), serde_json::Value::String(output_text));
            }
            state.completed_nodes.insert(node_id.to_string());
            state.current_nodes.extend(node.next_nodes.clone());

            // If there are still pending agent tasks, we should remain paused
            if state.pending_agent_tasks.is_empty() {
                state.status = WorkflowStatus::Running;
            }

            self.persist_state(&state)?;
            Ok(state)
        } else {
            Err("Pending node is not AgentTask".to_string())
        }
    }

    pub async fn redispatch_pending_agent_tasks(&self, execution_id: &str) -> Result<(), String> {
        let state = self.get_state(execution_id).ok_or("Execution not found")?;
        let workflow = self
            .get_workflow(&state.workflow_id)
            .ok_or("Workflow not found")?;
        let ctx = self
            .execution_context
            .as_ref()
            .ok_or("Execution context is not configured")?;
        let instance_id = workflow
            .instance_id
            .clone()
            .or_else(|| ctx.team_instance_id.clone())
            .ok_or("Workflow has no instance scope")?;
        for node_id in &state.pending_agent_tasks {
            let node = workflow
                .nodes
                .get(node_id)
                .ok_or_else(|| format!("Pending node {} is missing", node_id))?;
            if let NodeType::AgentTask {
                agent_id,
                instruction,
                input_vars,
                ..
            } = &node.node_type
            {
                let mut prompt = format!("Task Instruction:\n{}\n\nInputs:\n", instruction);
                for var in input_vars {
                    if let Some(value) = state.data.get(var) {
                        prompt.push_str(&format!("{}: {}\n", var, value));
                    }
                }
                ctx.team_bus
                    .route_message(crate::infrastructure::message_bus::routing::TeamMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        team_instance_id: instance_id.clone(),
                        sender_member_id: "system".to_string(),
                        recipient_member_id: Some(agent_id.clone()),
                        recipient_role: None,
                        message_type:
                            crate::infrastructure::message_bus::routing::MessageType::Direct,
                        content: prompt,
                        metadata: Some(format!("iflow_dispatch:{}:{}", execution_id, node_id)),
                        delivery_status: "delivered".to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                    })
                    .await?;
            }
        }
        Ok(())
    }

    pub fn resolve_review(
        &self,
        execution_id: &str,
        approved: bool,
    ) -> Result<WorkflowState, String> {
        let mut state = self.get_state(execution_id).ok_or("Execution not found")?;
        let workflow = {
            let store = self.workflow_store.read().unwrap();
            store
                .get(&state.workflow_id)
                .cloned()
                .ok_or("Workflow not found")?
        };

        let node_id = state.pending_review.clone().ok_or("No pending review")?;
        let node = workflow
            .nodes
            .get(&node_id)
            .ok_or(format!("Node {} not found", node_id))?;

        let (approved_next, rejected_next, output_var) = match &node.node_type {
            NodeType::HumanReview {
                approved_next,
                rejected_next,
                output_var,
                ..
            } => (
                approved_next.clone(),
                rejected_next.clone(),
                output_var.clone(),
            ),
            _ => return Err("Pending node is not HumanReview".to_string()),
        };

        state
            .data
            .set(output_var, serde_json::Value::Bool(approved));
        state.pending_review = None;
        state.status = WorkflowStatus::Running;
        state.current_nodes.clear();
        state.current_nodes.push(if approved {
            approved_next
        } else {
            rejected_next
        });
        self.persist_state(&state)?;
        Ok(state)
    }
}
