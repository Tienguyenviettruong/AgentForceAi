use std::sync::Arc;
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::message_bus::routing::{TeamBusRouter, TeamMessage, MessageType};
use crate::providers::BaseProviderAdapter;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct AgentWorker {
    pub agent_id: String,
    pub team_instance_id: String,
    db: Arc<dyn DatabasePort>,
    team_bus: Arc<TeamBusRouter>,
    task_exec_lock: Mutex<()>,
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
            task_exec_lock: Mutex::new(()),
        }
    }

    pub async fn start(self: Arc<Self>) {
        let agent_role = self.get_agent_role().await.unwrap_or_else(|| "Agent".to_string());
        
        let mut rx = self.team_bus
            .register_member(&self.team_instance_id, &self.agent_id, &agent_role)
            .await;
        let mut bc_rx = self.team_bus.subscribe_broadcast(&self.team_instance_id).await;
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(2));

        println!("AgentWorker {} (Role: {}) started for instance {}", self.agent_id, agent_role, self.team_instance_id);

        loop {
            tokio::select! {
                _ = tick.tick() => {
                    self.try_execute_next_task().await;
                }
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

    async fn select_message_handler_agent_id(&self) -> Option<String> {
        self.db
            .get_instance_agents(&self.team_instance_id)
            .ok()
            .and_then(|ids| ids.into_iter().next())
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
            self.persist_cross_team_case_event(&handoff, &msg);
            if handoff.handoff_type == "review_request" && !handoff.reply_to_team.is_empty() {
                let handler = self.select_review_handler_agent_id().await;
                if handler.as_deref() == Some(self.agent_id.as_str()) {
                    self.execute_cross_team_review(&msg, handoff.clone()).await;
                }
            }

            if handoff.handoff_type == "message" && !handoff.reply_to_team.is_empty() {
                let handler = self.select_message_handler_agent_id().await;
                if handler.as_deref() == Some(self.agent_id.as_str()) {
                    self.execute_cross_team_message(&msg, handoff).await;
                }
            }
        }
    }

    fn persist_cross_team_case_event(&self, handoff: &CrossTeamHandoff, msg: &TeamMessage) {
        let owner_instance_id = if !handoff.reply_to_team.is_empty() {
            handoff.reply_to_team.clone()
        } else {
            handoff.from_team.clone()
        };
        let target_instance_id = self.team_instance_id.clone();

        let (event_type, summary) = if handoff.handoff_type == "status_event" {
            let t = handoff
                .event
                .as_ref()
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("STATUS_EVENT")
                .to_string();
            let s = handoff
                .event
                .as_ref()
                .and_then(|v| v.get("summary"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (t, s)
        } else {
            (handoff.handoff_type.clone(), handoff.briefing_package.clone())
        };

        let summary = if summary.len() > 180 {
            summary.chars().take(180).collect::<String>()
        } else {
            summary
        };

        let _ = self.db.upsert_cross_team_case(
            &handoff.correlation_id,
            &owner_instance_id,
            &target_instance_id,
            &event_type,
            &summary,
        );

        let _ = self
            .db
            .insert_cross_team_case_event(&crate::core::models::CrossTeamCaseEventRecord {
                id: Uuid::new_v4().to_string(),
                correlation_id: handoff.correlation_id.clone(),
                from_instance_id: handoff.from_team.clone(),
                reply_to_instance_id: handoff.reply_to_team.clone(),
                event_type,
                summary,
                payload: msg.metadata.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            });
    }

    async fn try_execute_next_task(&self) {
        let Ok(_guard) = self.task_exec_lock.try_lock() else {
            return;
        };

        let Ok(Some(agent)) = self.db.get_agent(&self.agent_id) else {
            return;
        };
        let Ok(Some(provider_config)) = self.db.get_provider_by_name(&agent.provider) else {
            return;
        };

        let team_id = self
            .db
            .list_instances()
            .ok()
            .and_then(|instances| {
                instances
                    .into_iter()
                    .find(|i| i.id == self.team_instance_id)
                    .map(|i| i.team_id)
            })
            .unwrap_or_default();

        let tasks = self
            .db
            .list_tasks_for_instance(&self.team_instance_id)
            .unwrap_or_default();

        let mut next_task: Option<crate::tasks::shared_task_list::Task> = None;
        for task in &tasks {
            if task.status != "pending" || task.assignee_id.as_ref() != Some(&self.agent_id) {
                continue;
            }
            if let Some(payload) = &task.payload {
                if let Ok(dag_task) =
                    serde_json::from_str::<crate::application::orchestration::core::DagTask>(payload)
                {
                    let mut all_deps_met = true;
                    for dep_id in dag_task.dependencies {
                        let full_dep_id = format!("{}:{}", self.team_instance_id, dep_id);
                        if let Some(dt) = tasks.iter().find(|t| t.id == full_dep_id) {
                            if dt.status != "completed" {
                                all_deps_met = false;
                                break;
                            }
                        }
                    }
                    if all_deps_met {
                        next_task = Some(task.clone());
                        break;
                    }
                } else {
                    next_task = Some(task.clone());
                    break;
                }
            } else {
                next_task = Some(task.clone());
                break;
            }
        }

        let Some(task) = next_task else {
            return;
        };

        let claimed = self
            .db
            .claim_task_for_instance(&task.id, &self.agent_id, &self.team_instance_id)
            .unwrap_or(false);
        if !claimed {
            return;
        }

        let workspace_dir = self
            .db
            .get_setting(&format!("workspace_{}", self.team_instance_id))
            .ok()
            .flatten();

        let chat_service =
            crate::application::services::chat_service::ChatService::new(self.db.clone(), self.team_bus.clone());
        let sys_prompt = chat_service
            .build_dynamic_system_prompt(&team_id, &self.team_instance_id, &self.agent_id)
            .unwrap_or_default();

        let task_text = task.payload.clone().unwrap_or_else(|| task.id.clone());
        let mut instructions = if let Some(ref ws) = workspace_dir {
            format!("Execute the following task. You are working in the directory: {}. If you generate or modify any files, use a markdown code block starting with ```file:<filepath> and ending with ```. Please output absolute file paths within this directory. For example:\n```file:{}/example.txt\nFile contents here\n```\nTask:\n", ws, ws)
        } else {
            "Execute the following task. If you generate or modify any files, use a markdown code block starting with ```file:<filepath> and ending with ```. For example:\n```file:/workspace/example.txt\nFile contents here\n```\nTask:\n".to_string()
        };

        if let Ok(query_vec) = crate::providers::embeddings::EmbeddingProvider::new()
            .get_embedding(&task_text)
            .await
        {
            if let Ok(similar) = self.db.search_similar_chunks(&query_vec, 3) {
                if !similar.is_empty() {
                    instructions.push_str("\n\n[SYSTEM KNOWLEDGE RETRIEVAL]\nHere is context retrieved from the user's Obsidian Vault that might be relevant to your task:\n");
                    for (title, chunk_content, sim) in similar {
                        if sim > 0.6 {
                            instructions.push_str(&format!(
                                "\n--- Document: {} (Similarity: {:.2}) ---\n{}\n",
                                title, sim, chunk_content
                            ));
                        }
                    }
                    instructions.push_str("\n[END KNOWLEDGE RETRIEVAL]\n\n");
                }
            }
        }

        let history = vec![
            crate::providers::ChatMessage {
                role: "system".into(),
                content: sys_prompt.into(),
                agent_name: Some(agent.name.clone().into()),
            },
            crate::providers::ChatMessage {
                role: "user".into(),
                content: format!("{}{}", instructions, task_text).into(),
                agent_name: None,
            },
        ];

        let adapter: Option<Arc<dyn BaseProviderAdapter>> = match provider_config.provider_name.as_str() {
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

        let Some(adapter) = adapter else {
            let _ = self.db.mark_task_failed(&task.id);
            return;
        };

        let result = adapter.send_message(history).await;
        let (status_text, ok) = match result {
            Ok(resp) => {
                let text = resp.content.to_string();
                let _ = self.db.insert_token_usage(
                    Some(&self.team_instance_id),
                    &self.agent_id,
                    resp.token_usage.input_tokens,
                    resp.token_usage.output_tokens,
                    resp.token_usage.total_tokens,
                );

                let chat_service = crate::application::services::chat_service::ChatService::new(
                    self.db.clone(),
                    self.team_bus.clone(),
                );
                let (files_written, _) = chat_service.parse_and_write_files(&text, workspace_dir.as_ref());
                let mut final_text = format!("[Task Completed] {}:\n{}", task.id, text);
                if !files_written.is_empty() {
                    final_text.push_str("\n\n**Files Generated/Modified:**\n");
                    for f in files_written {
                        let display_path = if let Some(ws) = workspace_dir.as_ref() {
                            f.replace(ws, "")
                        } else {
                            std::path::Path::new(&f)
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or(f)
                        };
                        let display_path = display_path
                            .trim_start_matches('/')
                            .trim_start_matches('\\');
                        final_text.push_str(&format!("- `{}`\n", display_path));
                    }
                }
                (final_text, true)
            }
            Err(e) => (format!("[Task Failed] {}:\n{}", task.id, e), false),
        };

        let _ = if ok {
            self.db.mark_task_completed(&task.id)
        } else {
            self.db.mark_task_failed(&task.id)
        };

        let metadata = serde_json::json!({"agent_name": agent.name}).to_string();
        let mut msg = TeamMessage::new_broadcast(
            self.team_instance_id.clone(),
            "assistant".to_string(),
            status_text.clone(),
        );
        msg.metadata = Some(metadata.clone());
        let _ = self.db.insert_team_message(&msg);
        let _ = self.team_bus.route_message(msg).await;

        let mut session = self
            .db
            .get_latest_session_for_instance(&self.team_instance_id)
            .ok()
            .flatten();
        if session.is_none() {
            let _ = self.db.create_session_for_instance(&self.team_instance_id, &self.agent_id);
            session = self
                .db
                .get_latest_session_for_instance(&self.team_instance_id)
                .ok()
                .flatten();
        }
        if let Some(session) = session {
            let _ = self
                .db
                .ensure_session(&session.id, &self.agent_id, Some(&self.team_instance_id));
            let _ = self.db.append_conversation_turn(
                &session.id,
                "assistant",
                &status_text,
                Some(&metadata),
            );
            let _ = self.db.touch_session(&session.id);
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
        serde_json::from_str::<CrossTeamHandoff>(&payload_str).ok()
    }

    async fn emit_status_event(
        &self,
        correlation_id: &str,
        reply_to_team: &str,
        event_type: &str,
        summary: &str,
    ) {
        if reply_to_team.trim().is_empty() {
            return;
        }
        let payload = serde_json::json!({
            "handoff_type": "status_event",
            "correlation_id": correlation_id,
            "from_team": self.team_instance_id.clone(),
            "reply_to_team": reply_to_team,
            "briefing_package": "",
            "event": {
                "type": event_type,
                "summary": summary,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });
        let payload_str = payload.to_string();
        let mut msg = TeamMessage::new_broadcast(
            reply_to_team.to_string(),
            self.agent_id.clone(),
            format!("[CROSS_TEAM_HANDOFF] {}", payload_str.clone()),
        );
        msg.metadata = Some(payload_str);
        let _ = self.db.insert_team_message(&msg);
        let _ = self.team_bus.route_message(msg).await;
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
            None,
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

    async fn execute_cross_team_message(&self, original_msg: &TeamMessage, handoff: CrossTeamHandoff) {
        let correlation_id = if handoff.correlation_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            handoff.correlation_id.clone()
        };
        self.emit_status_event(
            &correlation_id,
            &handoff.reply_to_team,
            "ACK_RECEIVED",
            "Received cross-team handoff.",
        )
        .await;
        self.emit_status_event(
            &correlation_id,
            &handoff.reply_to_team,
            "READBACK_CONFIRMED",
            &format!("I understand the request: {}", handoff.briefing_package),
        )
        .await;

        let agent = match self.db.get_agent(&self.agent_id) {
            Ok(Some(a)) => a,
            _ => return,
        };

        let provider_config = match self.db.get_provider_by_name(&agent.provider) {
            Ok(Some(p)) => p,
            _ => return,
        };

        let team_id = self
            .db
            .list_instances()
            .ok()
            .and_then(|instances| {
                instances
                    .into_iter()
                    .find(|i| i.id == self.team_instance_id)
                    .map(|i| i.team_id)
            })
            .unwrap_or_default();

        let chat_service = crate::application::services::chat_service::ChatService::new(self.db.clone(), self.team_bus.clone());
        let mut sys = chat_service
            .build_dynamic_system_prompt(&team_id, &self.team_instance_id, &self.agent_id)
            .unwrap_or_default();

        sys.push_str("\n\nCROSS-TEAM HANDOFF\nYou received a cross-team handoff from another instance.\nYou MUST (1) acknowledge the request, (2) create an execution plan for your team, and (3) delegate via create_subtasks if needed.\nAfter you have a plan, output a short summary update for the requester.\n");
        let role_mapping = self
            .db
            .get_instance_agent_name_mapping(&self.team_instance_id)
            .unwrap_or_default();
        if !role_mapping.is_empty() {
            let mut roles: Vec<String> = role_mapping.keys().cloned().collect();
            roles.sort();
            sys.push_str(&format!(
                "\nAvailable roles in this instance: {}. When calling create_subtasks, each task.role MUST match one of these exactly.\n",
                roles.join(", ")
            ));
        }

        let user_text = format!(
            "Cross-team message\ncorrelation_id: {}\nfrom_instance: {}\noriginal_message: {}\n\nMessage:\n{}",
            correlation_id, handoff.from_team, original_msg.content, handoff.briefing_package
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
            None,
        );

        let response_text = executor.execute_task(history).await.ok().unwrap_or_default();
        if handoff.reply_to_team.is_empty() || response_text.is_empty() {
            return;
        }

        let payload = serde_json::json!({
            "handoff_type": "message_response",
            "correlation_id": correlation_id,
            "from_team": self.team_instance_id,
            "reply_to_team": handoff.reply_to_team,
            "briefing_package": response_text
        })
        .to_string();

        let content = format!("[CROSS_TEAM_HANDOFF] {}", payload.clone());
        let mut msg = TeamMessage::new_broadcast(
            handoff.reply_to_team.clone(),
            self.agent_id.clone(),
            content.clone(),
        );
        msg.metadata = Some(payload);
        let _ = self.db.insert_team_message(&msg);
        let _ = self.team_bus.route_message(msg).await;

        let origin_agent_id = self
            .db
            .get_instance_agents(&handoff.reply_to_team)
            .ok()
            .and_then(|ids| ids.first().cloned());
        if let Some(origin_agent_id) = origin_agent_id {
            let mut session = self
                .db
                .get_latest_session_for_instance(&handoff.reply_to_team)
                .ok()
                .flatten();
            if session.is_none() {
                let _ = self.db.create_session_for_instance(&handoff.reply_to_team, &origin_agent_id);
                session = self
                    .db
                    .get_latest_session_for_instance(&handoff.reply_to_team)
                    .ok()
                    .flatten();
            }
            if let Some(session) = session {
                let meta = serde_json::json!({"agent_name": agent.name}).to_string();
                let _ = self
                    .db
                    .ensure_session(&session.id, &origin_agent_id, Some(&handoff.reply_to_team));
                let _ = self.db.append_conversation_turn(&session.id, "assistant", &content, Some(&meta));
                let _ = self.db.touch_session(&session.id);
            }
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

#[derive(Clone, Debug, Deserialize)]
struct CrossTeamHandoff {
    handoff_type: String,
    correlation_id: String,
    from_team: String,
    reply_to_team: String,
    #[serde(default)]
    briefing_package: String,
    #[serde(default)]
    context: Option<serde_json::Value>,
    #[serde(default)]
    event: Option<serde_json::Value>,
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
