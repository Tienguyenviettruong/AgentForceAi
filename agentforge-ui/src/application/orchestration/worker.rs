use std::sync::Arc;
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::message_bus::routing::{TeamBusRouter, TeamMessage, MessageType};
use crate::providers::BaseProviderAdapter;
use std::collections::HashMap;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct AgentWorker {
    pub agent_id: String,
    pub team_instance_id: String,
    db: Arc<dyn DatabasePort>,
    team_bus: Arc<TeamBusRouter>,
}

impl AgentWorker {
    pub fn new(
        agent_id: String,
        team_instance_id: String,
        db: Arc<dyn DatabasePort>,
        team_bus: Arc<TeamBusRouter>,
    ) -> Self {
        Self {
            agent_id,
            team_instance_id,
            db,
            team_bus,
        }
    }

    pub async fn start(self: Arc<Self>) {
        let agent_role = self.get_agent_role().await.unwrap_or_else(|| "Agent".to_string());
        
        let mut rx = self.team_bus
            .register_member(&self.team_instance_id, &self.agent_id, &agent_role)
            .await;
        let mut bc_rx = self.team_bus.subscribe_broadcast(&self.team_instance_id).await;

        println!("AgentWorker {} (Role: {}) started for instance {}", self.agent_id, agent_role, self.team_instance_id);

        loop {
            tokio::select! {
                msg = rx.recv() => {
                    let Some(msg) = msg else { break; };
                    self.handle_message(msg).await;
                }
                msg = bc_rx.recv() => {
                    let Ok(msg) = msg else { break; };
                    self.handle_message(msg).await;
                }
            }
        }

        self.team_bus.unregister_member(&self.team_instance_id, &self.agent_id, &agent_role).await;
        println!("AgentWorker {} stopped", self.agent_id);
    }

    async fn get_agent_role(&self) -> Option<String> {
        let agent = self.db.get_agent(&self.agent_id).ok()??;
        Some(agent.name)
    }

    async fn select_review_handler_agent_id(&self) -> Option<String> {
        let agent_ids = self.db.get_instance_agents(&self.team_instance_id).ok()?;
        if agent_ids.is_empty() {
            return None;
        }
        let mut resolved = Vec::new();
        for id in agent_ids.iter() {
            let name = self
                .db
                .get_agent(id)
                .ok()
                .flatten()
                .map(|a| a.name)
                .unwrap_or_else(|| id.clone());
            resolved.push((id.clone(), name.to_lowercase()));
        }
        for key in ["critic", "qa", "reviewer"] {
            if let Some((id, _)) = resolved.iter().find(|(_, name)| name.contains(key)) {
                return Some(id.clone());
            }
        }
        Some(resolved[0].0.clone())
    }

    async fn handle_message(&self, msg: TeamMessage) {
        if msg.sender_member_id == self.agent_id {
            return;
        }

        println!("AgentWorker {} received message: {}", self.agent_id, msg.content);

        // Process iFlow task dispatch
        if let Some(metadata) = &msg.metadata {
            if metadata.starts_with("iflow_dispatch:") {
                self.execute_iflow_task(msg).await;
                return;
            }
        }

        if let Some(handoff) = Self::parse_cross_team_handoff(&msg) {
            if handoff.handoff_type == "review_request" && !handoff.reply_to_team.is_empty() {
                let handler = self.select_review_handler_agent_id().await;
                if handler.as_deref() == Some(self.agent_id.as_str()) {
                    self.execute_cross_team_review(&msg, handoff).await;
                }
            }
        }
    }

    fn parse_cross_team_handoff(msg: &TeamMessage) -> Option<CrossTeamHandoff> {
        let mut payload_str = msg.metadata.clone();
        if payload_str.is_none() {
            let prefix = "[CROSS_TEAM_HANDOFF]";
            if msg.content.starts_with(prefix) {
                payload_str = Some(msg.content[prefix.len()..].trim().to_string());
            }
        }
        let payload_str = payload_str?;
        let v: serde_json::Value = serde_json::from_str(&payload_str).ok()?;
        Some(CrossTeamHandoff {
            handoff_type: v.get("handoff_type").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            correlation_id: v.get("correlation_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            from_team: v.get("from_team").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            reply_to_team: v.get("reply_to_team").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            briefing_package: v.get("briefing_package").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        })
    }

    async fn execute_cross_team_review(&self, original_msg: &TeamMessage, handoff: CrossTeamHandoff) {
        let agent = match self.db.get_agent(&self.agent_id) {
            Ok(Some(a)) => a,
            _ => return,
        };

        let provider_config = match self.db.get_provider_by_name(&agent.provider) {
            Ok(Some(p)) => p,
            _ => return,
        };

        let team_id = self.db
            .list_instances()
            .ok()
            .and_then(|instances| instances.into_iter().find(|i| i.id == self.team_instance_id).map(|i| i.team_id))
            .unwrap_or_default();

        let chat_service = crate::application::services::chat_service::ChatService::new(self.db.clone(), self.team_bus.clone());
        let mut sys = chat_service
            .build_dynamic_system_prompt(&team_id, &self.team_instance_id, &self.agent_id)
            .unwrap_or_default();

        sys.push_str("\n\nROLE: CRITIC\nYou are performing a cross-team review.\nYou MUST output a structured critique (numbered issues + concrete fixes).\nAfter writing the critique, you MUST respond to the requester by calling the tool handoff_to_team with handoff_type='review_response', correlation_id preserved, target_team=reply_to_team.\n");

        let correlation_id = if handoff.correlation_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            handoff.correlation_id.clone()
        };

        let user_text = format!(
            "Cross-team review request\ncorrelation_id: {}\nfrom_team: {}\noriginal_message: {}\n\nArtifact to review:\n{}",
            correlation_id,
            handoff.from_team,
            original_msg.content,
            handoff.briefing_package
        );

        let history = vec![
            crate::providers::ChatMessage {
                role: "system".into(),
                content: sys.into(),
                agent_name: Some(agent.name.clone().into()),
            },
            crate::providers::ChatMessage {
                role: "user".into(),
                content: user_text.into(),
                agent_name: None,
            },
        ];

        let adapter: Option<Arc<dyn crate::providers::BaseProviderAdapter>> = match provider_config.provider_name.as_str() {
            "openrouter" => {
                let mut a = crate::providers::openrouter::OpenRouterAdapter::new();
                if a.initialize(&provider_config).is_ok() { Some(Arc::new(a)) } else { None }
            }
            "claude" => {
                let mut a = crate::providers::claude::ClaudeAdapter::new();
                if a.initialize(&provider_config).is_ok() { Some(Arc::new(a)) } else { None }
            }
            "gemini" => {
                let mut a = crate::providers::gemini::GeminiAdapter::new();
                if a.initialize(&provider_config).is_ok() { Some(Arc::new(a)) } else { None }
            }
            "codex" => {
                let mut a = crate::providers::codex::CodexAdapter::new();
                if a.initialize(&provider_config).is_ok() { Some(Arc::new(a)) } else { None }
            }
            "opencode" => {
                let mut a = crate::providers::opencode::OpenCodeAdapter::new();
                if a.initialize(&provider_config).is_ok() { Some(Arc::new(a)) } else { None }
            }
            _ => None,
        };

        let Some(adapter) = adapter else { return; };

        let mcp_registry = Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(self.db.clone()));
        let executor = crate::application::orchestration::executor::AgentExecutor::new(
            adapter,
            mcp_registry,
            self.db.clone(),
            self.team_bus.clone(),
            self.team_instance_id.clone(),
            agent.id.clone(),
        );

        let result = executor.execute_task(history).await.ok().unwrap_or_default();
        if handoff.reply_to_team.is_empty() {
            return;
        }

        if !result.is_empty() {
            let payload = serde_json::json!({
                "handoff_type": "review_response",
                "correlation_id": correlation_id,
                "from_team": self.team_instance_id,
                "reply_to_team": handoff.reply_to_team,
                "briefing_package": result
            })
            .to_string();

            let mut msg = TeamMessage::new_broadcast(
                handoff.reply_to_team,
                self.agent_id.clone(),
                format!("[CROSS_TEAM_HANDOFF] {}", payload),
            );
            msg.metadata = Some(payload);
            let _ = self.db.insert_team_message(&msg);
            let _ = self.team_bus.route_message(msg).await;
        }
    }

    async fn execute_iflow_task(&self, msg: TeamMessage) {
        let parts: Vec<&str> = msg.metadata.as_ref().unwrap().split(':').collect();
        if parts.len() < 3 { return; }
        
        let execution_id = parts[1];
        let node_id = parts[2];
        let instruction = &msg.content;

        let agent = match self.db.get_agent(&self.agent_id) {
            Ok(Some(a)) => a,
            _ => return,
        };

        let provider_config = match self.db.get_provider_by_name(&agent.provider) {
            Ok(Some(p)) => p,
            _ => return,
        };

        let mut history = Vec::new();
        if let Some(system_prompt) = agent.system_prompt.clone() {
            history.push(crate::providers::ChatMessage {
                role: "system".into(),
                content: system_prompt.into(),
                agent_name: Some(agent.name.clone().into()),
            });
        }
        history.push(crate::providers::ChatMessage {
            role: "user".into(),
            content: instruction.clone().into(),
            agent_name: Some(agent.name.clone().into()),
        });

        // We will send a stream message back
        let message_id = uuid::Uuid::new_v4().to_string();
        let content = format!("[{}]: ", agent.name);
        
        let stream_msg = TeamMessage {
            id: message_id.clone(),
            team_instance_id: self.team_instance_id.clone(),
            sender_member_id: self.agent_id.clone(),
            recipient_member_id: None,
            recipient_role: None,
            message_type: MessageType::Broadcast,
            content: content.clone(),
            metadata: Some(format!("{{\"workflow_execution_id\":\"{}\",\"node_id\":\"{}\"}}", execution_id, node_id)),
            delivery_status: "delivered".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let _ = self.db.insert_team_message(&stream_msg);
        let _ = self.team_bus.route_message(stream_msg.clone()).await;

        let mut output_text = String::new();
        
        // Using a macro or similar block to handle adapters
        // For brevity, we instantiate openrouter adapter directly here
        // In real app, we should use a factory
        use crate::providers::BaseProviderAdapter;
        use futures::stream::StreamExt;
        
        let mut adapter = crate::providers::openrouter::OpenRouterAdapter::new();
        if adapter.initialize(&provider_config).is_ok() {
            if let Ok(mut stream) = adapter.send_message_stream(history).await {
                while let Some(chunk) = stream.next().await {
                    if let Ok(chunk) = chunk {
                        match chunk {
                            crate::providers::StreamChunk::Text(t) => {
                                output_text.push_str(&t);
                                let _ = self.db.update_team_message_content(&message_id, &format!("[{}]: {}", agent.name, output_text));
                            }
                            crate::providers::StreamChunk::Done(_) => break,
                        }
                    }
                }
            }
        }

        // Send a completion message so the engine can resume
        let completion_msg = TeamMessage {
            id: Uuid::new_v4().to_string(),
            team_instance_id: self.team_instance_id.clone(),
            sender_member_id: self.agent_id.clone(),
            recipient_member_id: None,
            recipient_role: None,
            message_type: MessageType::System,
            content: output_text,
            metadata: Some(format!("iflow_result:{}:{}", execution_id, node_id)),
            delivery_status: "delivered".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let _ = self.team_bus.route_message(completion_msg).await;
    }
}

#[derive(Clone, Debug)]
struct CrossTeamHandoff {
    handoff_type: String,
    correlation_id: String,
    from_team: String,
    reply_to_team: String,
    briefing_package: String,
}

pub struct WorkerManager {
    pub db: Arc<dyn DatabasePort>,
    team_bus: Arc<TeamBusRouter>,
    workers: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl WorkerManager {
    pub fn new(db: Arc<dyn DatabasePort>, team_bus: Arc<TeamBusRouter>) -> Self {
        Self {
            db,
            team_bus,
            workers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_workers_for_instance(&self, instance_id: &str) {
        if let Ok(agent_ids) = self.db.get_instance_agents(instance_id) {
            let mut workers = self.workers.lock().await;
            for agent_id in agent_ids {
                let worker_key = format!("{}_{}", instance_id, agent_id);
                if !workers.contains_key(&worker_key) {
                    let worker = Arc::new(AgentWorker::new(
                        agent_id.clone(),
                        instance_id.to_string(),
                        self.db.clone(),
                        self.team_bus.clone(),
                    ));
                    
                    let handle = tokio::spawn(async move {
                        worker.start().await;
                    });
                    
                    workers.insert(worker_key, handle);
                }
            }
        }
    }
}
