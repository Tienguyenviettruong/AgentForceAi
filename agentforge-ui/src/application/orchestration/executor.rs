use std::sync::Arc;
use crate::core::traits::database::DatabasePort;
use crate::providers::BaseProviderAdapter;
use crate::core::models::chat::ChatMessage;
use crate::infrastructure::mcp::registry::McpToolRegistry;
use crate::infrastructure::message_bus::routing::{TeamBusRouter, TeamMessage};
use anyhow::Result;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

pub struct AgentExecutor {
    provider: Arc<dyn BaseProviderAdapter>,
    mcp_registry: Arc<McpToolRegistry>,
    db: Arc<dyn DatabasePort>,
    team_bus: Arc<TeamBusRouter>,
    team_instance_id: String,
    agent_id: String,
}

impl AgentExecutor {
    pub fn new(
        provider: Arc<dyn BaseProviderAdapter>,
        mcp_registry: Arc<McpToolRegistry>,
        db: Arc<dyn DatabasePort>,
        team_bus: Arc<TeamBusRouter>,
        team_instance_id: String,
        agent_id: String,
    ) -> Self {
        Self { provider, mcp_registry, db, team_bus, team_instance_id, agent_id }
    }

    pub async fn execute_task(&self, mut history: Vec<ChatMessage>) -> Result<String> {
        let mut iteration = 0;
        let max_iterations = 5;

        // Apply Sliding Window context pruning
        history = self.prune_history(&history, 20);

        while iteration < max_iterations {
            let mut response_text = String::new();
            let mut tool_calls = Vec::<ToolCall>::new();

            let mut stream = self.provider.send_message_stream(history.clone()).await?;
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(crate::core::models::chat::StreamChunk::Text(t)) => response_text.push_str(&t),
                    Ok(crate::core::models::chat::StreamChunk::Done(_)) => break,
                    Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
                }
            }

            // Simulate tool call extraction via regex for now, since we haven't updated LlmProviderPort
            // We can look for `<tool_call>...</tool_call>` in response_text
            if let Some(start) = response_text.find("<tool_call>") {
                if let Some(end) = response_text.find("</tool_call>") {
                    let tool_json = &response_text[start+11..end];
                    if let Ok(tc) = serde_json::from_str::<ToolCall>(tool_json) {
                        tool_calls.push(tc);
                    }
                }
            }

            if tool_calls.is_empty() {
                return Ok(response_text);
            }

            // Execute tools
            for tc in tool_calls.clone() {
                let result = self.execute_tool(&tc.name, &tc.arguments).await;
                history.push(ChatMessage {
                    role: "user".into(), // Hack: send tool result as user since no tool role in ChatMessage
                    content: format!("Tool result ({}):\n{}", tc.name, result).into(),
                    agent_name: Some(tc.name.clone().into()),
                });
            }

            history.push(ChatMessage {
                role: "assistant".into(),
                content: response_text.into(),
                agent_name: None,
            });

            iteration += 1;
        }
        Ok("Max tool iterations reached".to_string())
    }

    fn prune_history(&self, history: &[ChatMessage], max_messages: usize) -> Vec<ChatMessage> {
        let mut pruned = Vec::new();
        // Keep System Prompt (Index 0)
        if let Some(sys) = history.first().filter(|m| m.role == "system") {
            pruned.push(sys.clone());
        }
        
        let tail_start = history.len().saturating_sub(max_messages);
        let start_idx = std::cmp::max(1, tail_start); // Skip index 0 as it's system prompt
        
        if start_idx < history.len() {
            pruned.extend_from_slice(&history[start_idx..]);
        }
        pruned
    }

    async fn execute_tool(&self, name: &str, args: &str) -> String {
        if name == "save_to_knowledge" {
            // Simple JSON parsing for demonstration
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(args) {
                let title = v["title"].as_str().unwrap_or("Untitled").to_string();
                let content = v["content"].as_str().unwrap_or("").to_string();
                let entry = crate::core::models::knowledge::KnowledgeEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    agent_id: self.agent_id.clone(),
                    session_id: Some(self.team_instance_id.clone()),
                    title,
                    content,
                    tags: vec![],
                    created_at: chrono::Utc::now(),
                };
                if let Err(e) = self.db.upsert_knowledge_entry(&entry) {
                    return format!("Failed to save knowledge: {}", e);
                }
                return "Knowledge saved successfully.".to_string();
            }
        }
        
        if name == "declare_consensus" {
            let msg = crate::infrastructure::message_bus::routing::TeamMessage::new_broadcast(
                self.team_instance_id.clone(),
                self.agent_id.clone(),
                format!("[CONSENSUS_REACHED] {}", args)
            );
            let _ = self.team_bus.route_message(msg).await;
            return "Consensus declared and broadcasted.".to_string();
        }

        if name == "handoff_to_team" {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(args) {
                let target_team = v["target_team"].as_str().unwrap_or("UnknownTeam");
                let package = v["briefing_package"].as_str().unwrap_or("");
                let msg = crate::infrastructure::message_bus::routing::TeamMessage::new_broadcast(
                    target_team.to_string(), // Cross-team route
                    self.agent_id.clone(),
                    format!("[CROSS_TEAM_HANDOFF] {}", package)
                );
                let _ = self.team_bus.route_message(msg).await;
                return format!("Handoff package sent to {}.", target_team);
            }
        }

        if name == "create_subtasks" {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(args) {
                if let Some(tasks) = v["tasks"].as_array() {
                    for t in tasks {
                        let desc = t["description"].as_str().unwrap_or("");
                        let role = t["role"].as_str().unwrap_or("");
                        let msg = crate::infrastructure::message_bus::routing::TeamMessage::new_role_group(
                            self.team_instance_id.clone(),
                            self.agent_id.clone(),
                            role.to_string(),
                            format!("[NEW_TASK] {}", desc)
                        );
                        let _ = self.team_bus.route_message(msg).await;
                    }
                    return format!("Created {} subtasks and dispatched.", tasks.len());
                }
            }
        }
        
        format!("Tool {} executed successfully with args: {}", name, args)
    }
}
