use std::sync::Arc;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use anyhow::Result;
use crate::core::traits::database::DatabasePort;
use crate::core::models::{WorkflowRecord, ChatMessage};
use crate::providers::BaseProviderAdapter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpActionLog {
    pub timestamp: DateTime<Utc>,
    pub tool_name: String,
    pub args: String,
    pub result: String,
}

pub struct ActionRecorder {
    logs: std::sync::RwLock<Vec<McpActionLog>>,
    db: Arc<dyn DatabasePort>,
}

impl ActionRecorder {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self {
            logs: std::sync::RwLock::new(Vec::new()),
            db,
        }
    }

    pub fn record_action(&self, tool_name: String, args: String, result: String) {
        let mut logs = self.logs.write().unwrap();
        logs.push(McpActionLog {
            timestamp: Utc::now(),
            tool_name,
            args,
            result,
        });
    }

    pub fn get_logs(&self) -> Vec<McpActionLog> {
        self.logs.read().unwrap().clone()
    }

    pub async fn generate_iflow_and_save(&self, llm_provider: Option<Arc<dyn BaseProviderAdapter>>) -> Result<String> {
        let logs = self.get_logs();
        if logs.is_empty() {
            return Ok("No logs to generate iFlow from".to_string());
        }

        let logs_json = serde_json::to_string_pretty(&logs)?;
        
        let prompt = format!("Convert the following MCP tool usage logs into an iFlow DAG JSON. Only output valid JSON with nodes and edges representing the workflow steps.\nLogs:\n{}", logs_json);
        
        let workflow_id = uuid::Uuid::new_v4().to_string();
        let workflow_name = format!("Learned Workflow {}", Utc::now().timestamp());
        
        let Some(llm) = llm_provider else {
            return Ok("No provider configured to generate iFlow. Configure an AI provider first.".to_string());
        };
        let messages = vec![ChatMessage {
            role: "user".to_string().into(),
            content: prompt.into(),
            agent_name: None,
        }];
        let definition = match llm.send_message(messages).await {
            Ok(response) => response.content.to_string(),
            Err(e) => return Ok(format!("Failed to generate iFlow via provider: {}", e)),
        };

        let wf = WorkflowRecord {
            id: workflow_id.clone(),
            name: workflow_name.clone(),
            definition,
            version: "1.0".to_string(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        self.db.upsert_workflow(&wf)?;
        
        self.logs.write().unwrap().clear();

        Ok(format!("Generated and saved workflow: {}", workflow_name))
    }

    fn generate_mock_dag(&self, workflow_id: &str, workflow_name: &str, logs: &[McpActionLog]) -> String {
        let mut nodes_json = String::new();
        for (i, log) in logs.iter().enumerate() {
            let node = format!(r#"
            "{node_id}": {{
                "id": "{node_id}",
                "node_type": {{"AgentTask": {{"agent_id": "auto", "task_description": "Execute {tool} with {args}"}}}},
                "name": "Step {i}",
                "position": {{"x": 100.0, "y": {y}}}
            }}"#, node_id = format!("node_{}", i), tool = log.tool_name, args = log.args.replace("\"", "\\\"").replace("\n", " "), y = 100.0 + (i as f32 * 100.0));
            if i > 0 {
                nodes_json.push_str(",\n");
            }
            nodes_json.push_str(&node);
        }

        format!(r#"{{
            "id": "{workflow_id}",
            "name": "{workflow_name}",
            "start_node_id": "node_0",
            "nodes": {{{nodes_json}}},
            "edges": []
        }}"#)
    }
}
