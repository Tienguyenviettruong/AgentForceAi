use crate::core::models::WorkflowRecord;
use crate::core::traits::database::DatabasePort;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpActionLog {
    pub timestamp: DateTime<Utc>,
    pub tool_name: String,
    pub args: String,
    pub result: String,
}

pub struct ActionRecorder {
    logs: std::sync::RwLock<VecDeque<McpActionLog>>,
    db: Arc<dyn DatabasePort>,
}

const MAX_ACTION_LOGS: usize = 1_000;

impl ActionRecorder {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self {
            logs: std::sync::RwLock::new(VecDeque::new()),
            db,
        }
    }

    pub fn record_action(&self, tool_name: String, args: String, result: String) {
        let mut logs = self.logs.write().unwrap();
        if logs.len() == MAX_ACTION_LOGS {
            logs.pop_front();
        }
        logs.push_back(McpActionLog {
            timestamp: Utc::now(),
            tool_name,
            args,
            result,
        });
    }

    pub fn get_logs(&self) -> Vec<McpActionLog> {
        self.logs.read().unwrap().iter().cloned().collect()
    }

    pub async fn generate_iflow_and_save(&self) -> Result<String> {
        let logs = self.get_logs();
        if logs.is_empty() {
            return Ok("No logs to generate iFlow from".to_string());
        }

        let workflow_id = uuid::Uuid::new_v4().to_string();
        let workflow_name = format!("Learned Workflow {}", Utc::now().timestamp());
        let instruction = logs
            .iter()
            .map(|log| {
                format!(
                    "Repeat governed action '{}' with input {} after review.",
                    log.tool_name, log.args
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut nodes = HashMap::new();
        nodes.insert(
            "start".to_string(),
            crate::application::iflow_engine::nodes::Node {
                id: "start".to_string(),
                name: "Start".to_string(),
                node_type: crate::application::iflow_engine::nodes::NodeType::Start,
                next_nodes: vec!["reviewed-action".to_string()],
            },
        );
        nodes.insert(
            "reviewed-action".to_string(),
            crate::application::iflow_engine::nodes::Node {
                id: "reviewed-action".to_string(),
                name: "Reviewed Recorded Actions".to_string(),
                node_type: crate::application::iflow_engine::nodes::NodeType::AgentTask {
                    agent_id: "unassigned".to_string(),
                    instruction,
                    input_vars: Vec::new(),
                    output_var: Some("result".to_string()),
                },
                next_nodes: vec!["end".to_string()],
            },
        );
        nodes.insert(
            "end".to_string(),
            crate::application::iflow_engine::nodes::Node {
                id: "end".to_string(),
                name: "End".to_string(),
                node_type: crate::application::iflow_engine::nodes::NodeType::End,
                next_nodes: Vec::new(),
            },
        );
        let workflow = crate::application::iflow_engine::engine::Workflow {
            id: workflow_id,
            name: workflow_name.clone(),
            version: "1.0".to_string(),
            nodes,
            start_node_id: "start".to_string(),
            team_id: None,
            instance_id: None,
        };
        crate::application::iflow_engine::engine::WorkflowEngine::validate_workflow(&workflow)
            .map_err(anyhow::Error::msg)?;
        let definition = serde_json::to_string(&workflow)?;

        let wf = WorkflowRecord {
            id: workflow.id.clone(),
            run_id: None,
            origin_kind: "learned".to_string(),
            activation_status: "review_required".to_string(),
            name: workflow.name.clone(),
            definition: definition.clone(),
            version: workflow.version.clone(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        self.db.upsert_workflow(&wf)?;

        self.logs.write().unwrap().clear();

        Ok(format!(
            "Generated learned workflow draft for review: {}",
            workflow_name
        ))
    }
}
