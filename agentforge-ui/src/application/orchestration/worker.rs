use crate::core::traits::database::DatabasePort;
use crate::infrastructure::message_bus::routing::{MessageType, TeamBusRouter, TeamMessage};
use crate::providers::BaseProviderAdapter;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

fn provider_kind(p: &crate::db::Provider) -> &str {
    match p.provider_name.as_str() {
        "openrouter" | "claude" | "gemini" | "codex" | "opencode" => p.provider_name.as_str(),
        _ => match p.adapter_type.as_str() {
            "AnthropicAdapter" => "claude",
            "OpenAIAdapter" => "codex",
            "GeminiAdapter" => "gemini",
            "OpenCodeAdapter" => "opencode",
            _ => p.provider_name.as_str(),
        },
    }
}

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
        if !self.is_agent_online().await {
            println!(
                "AgentWorker {} not started because agent is offline",
                self.agent_id
            );
            return;
        }

        let agent_role = self
            .get_agent_role()
            .await
            .unwrap_or_else(|| "Agent".to_string());

        let mut rx = self
            .team_bus
            .register_member(&self.team_instance_id, &self.agent_id, &agent_role)
            .await;
        let mut bc_rx = self
            .team_bus
            .subscribe_broadcast(&self.team_instance_id)
            .await;
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(2));

        println!(
            "AgentWorker {} (Role: {}) started for instance {}",
            self.agent_id, agent_role, self.team_instance_id
        );

        loop {
            tokio::select! {
                _ = tick.tick() => {
                    if !self.is_agent_online().await {
                        break;
                    }
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

        self.team_bus
            .unregister_member(&self.team_instance_id, &self.agent_id, &agent_role)
            .await;
        println!("AgentWorker {} stopped", self.agent_id);
    }

    async fn get_agent_role(&self) -> Option<String> {
        let agent = self.db.get_agent(&self.agent_id).ok()??;
        Some(agent.routing_role())
    }

    async fn is_agent_online(&self) -> bool {
        self.db
            .get_agent(&self.agent_id)
            .ok()
            .flatten()
            .map(|agent| agent.status.to_lowercase() != "offline")
            .unwrap_or(false)
    }

    async fn select_review_handler_agent_id(&self, case_id: Option<&str>) -> Option<String> {
        let agent_ids = self.db.get_instance_agents(&self.team_instance_id).ok()?;
        if agent_ids.is_empty() {
            return None;
        }
        if let Some(case_id) = case_id {
            let service =
                crate::application::orchestration::collaboration::CollaborationService::new(
                    self.db.clone(),
                );
            if let Ok(Some(agent_id)) =
                service.route_by_competency(case_id, "delivery_quality", &agent_ids)
            {
                return Some(agent_id);
            }
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

    async fn select_message_handler_agent_id(&self, case_id: Option<&str>) -> Option<String> {
        let agent_ids = self.db.get_instance_agents(&self.team_instance_id).ok()?;
        if let Some(case_id) = case_id {
            let service =
                crate::application::orchestration::collaboration::CollaborationService::new(
                    self.db.clone(),
                );
            if let Ok(Some(agent_id)) =
                service.route_by_competency(case_id, "delivery_quality", &agent_ids)
            {
                return Some(agent_id);
            }
        }
        agent_ids.into_iter().next()
    }

    async fn handle_message(&self, msg: TeamMessage) {
        if msg.sender_member_id == self.agent_id {
            return;
        }

        println!(
            "AgentWorker {} received message: {}",
            self.agent_id, msg.content
        );

        // Process iFlow task dispatch
        if let Some(metadata) = &msg.metadata {
            if metadata.starts_with("iflow_dispatch:") {
                self.execute_iflow_task(msg).await;
                return;
            }
        }

        if let Some(handoff) = Self::parse_cross_team_handoff(&msg) {
            self.persist_cross_team_case_event(&handoff, &msg);
            let case_id = self.ensure_governed_case(&handoff);
            if handoff.handoff_type == "review_request" && !handoff.reply_to_team.is_empty() {
                let handler = self.select_review_handler_agent_id(case_id.as_deref()).await;
                if handler.as_deref() == Some(self.agent_id.as_str()) {
                    self.execute_cross_team_review(&msg, handoff.clone()).await;
                }
            }

            if matches!(handoff.handoff_type.as_str(), "message" | "handoff")
                && !handoff.reply_to_team.is_empty()
            {
                let handler = self.select_message_handler_agent_id(case_id.as_deref()).await;
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
            (
                handoff.handoff_type.clone(),
                handoff.briefing_package.clone(),
            )
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

        let _ =
            self.db
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
        if handoff.handoff_type != "status_event" {
            let _ = self.ensure_governed_case(handoff);
        }
    }

    fn ensure_governed_case(&self, handoff: &CrossTeamHandoff) -> Option<String> {
        if !handoff.case_id.trim().is_empty()
            && self
                .db
                .get_collaboration_case(&handoff.case_id)
                .ok()
                .flatten()
                .is_some()
        {
            return Some(handoff.case_id.clone());
        }
        if let Some(case) = self
            .db
            .get_collaboration_case_by_correlation_id(&handoff.correlation_id)
            .ok()
            .flatten()
        {
            return Some(case.id);
        }
        let service =
            crate::application::orchestration::collaboration::CollaborationService::new(
                self.db.clone(),
            );
        service
            .create_handoff(crate::application::orchestration::collaboration::HandoffInput {
                run_id: None,
                correlation_id: Some(&handoff.correlation_id),
                from_instance_id: &handoff.from_team,
                to_instance_id: &self.team_instance_id,
                from_agent_id: None,
                objective: &handoff.briefing_package,
                acceptance_json: "[]",
                constraints_json: "{}",
                context_refs_json: "[]",
                priority: "medium",
                risk_level: "medium",
            })
            .ok()
            .map(|(case_id, _, _)| case_id)
    }

    async fn try_execute_next_task(&self) {
        let Ok(_guard) = self.task_exec_lock.try_lock() else {
            return;
        };

        let Ok(Some(agent)) = self.db.get_agent(&self.agent_id) else {
            return;
        };
        let provider_config = self
            .db
            .get_provider_by_name(&agent.provider)
            .ok()
            .flatten()
            .or_else(|| {
                self.db.list_providers().ok().and_then(|providers| {
                    providers
                        .into_iter()
                        .find(|p| provider_kind(p) == agent.provider.as_str())
                })
            });
        let Some(provider_config) = provider_config else {
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
            if self.db.is_task_unblocked(&task.id).unwrap_or(false) {
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

        let chat_service = crate::application::services::chat_service::ChatService::new(
            self.db.clone(),
            self.team_bus.clone(),
        );
        let sys_prompt = chat_service
            .build_dynamic_system_prompt(&team_id, &self.team_instance_id, &self.agent_id)
            .unwrap_or_default();

        let task_text = task.payload.clone().unwrap_or_else(|| task.id.clone());
        let instructions = if let Some(ref ws) = workspace_dir {
            format!("Execute the following task. You are working in the directory: {}. To create or change files, call the write_file or edit_file tool with a relative path; do not represent file operations as markdown. Sensitive tools may pause for governance approval. Task:\n", ws)
        } else {
            "Execute the following task. File operations require a configured workspace and must be performed through write_file or edit_file tools; do not represent file operations as markdown. Sensitive tools may pause for governance approval. Task:\n".to_string()
        };

        let history = vec![
            crate::providers::ChatMessage {
                role: "system".into(),
                content: sys_prompt.into(),
                parts: vec![],
                agent_name: Some(agent.name.clone().into()),
                thought_duration_secs: None,
            },
            crate::providers::ChatMessage {
                role: "user".into(),
                content: format!("{}{}", instructions, task_text).into(),
                parts: vec![],
                agent_name: None,
                thought_duration_secs: None,
            },
        ];

        let adapter: Option<Arc<dyn BaseProviderAdapter>> = match provider_kind(&provider_config) {
            "openrouter" => {
                let mut a = crate::providers::openrouter::OpenRouterAdapter::new();
                if a.initialize(&provider_config).is_ok() {
                    Some(Arc::new(a))
                } else {
                    None
                }
            }
            "claude" => {
                let mut a = crate::providers::claude::ClaudeAdapter::new();
                if a.initialize(&provider_config).is_ok() {
                    Some(Arc::new(a))
                } else {
                    None
                }
            }
            "gemini" => {
                let mut a = crate::providers::gemini::GeminiAdapter::new();
                if a.initialize(&provider_config).is_ok() {
                    Some(Arc::new(a))
                } else {
                    None
                }
            }
            "codex" => {
                let mut a = crate::providers::codex::CodexAdapter::new();
                if a.initialize(&provider_config).is_ok() {
                    Some(Arc::new(a))
                } else {
                    None
                }
            }
            "opencode" => {
                let mut a = crate::providers::opencode::OpenCodeAdapter::new();
                if a.initialize(&provider_config).is_ok() {
                    Some(Arc::new(a))
                } else {
                    None
                }
            }
            _ => None,
        };

        let Some(adapter) = adapter else {
            let _ = self.db.mark_task_failed(&task.id);
            return;
        };

        let session_id = task.run_id.as_deref().and_then(|run_id| {
            self.db
                .get_orchestration_run(run_id)
                .ok()
                .flatten()
                .map(|run| run.session_id)
        });
        let mcp_registry = Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(
            self.db.clone(),
        ));
        let executor = crate::application::orchestration::executor::AgentExecutor::new(
            adapter,
            mcp_registry,
            self.db.clone(),
            self.team_bus.clone(),
            self.team_instance_id.clone(),
            self.agent_id.clone(),
            session_id,
            task.run_id.clone(),
            None,
            None,
        );
        let result = executor.execute_task(history).await;
        let (status_text, status) = match result {
            Ok(text) if text.starts_with("Approval required before executing") => (
                format!("[Task Waiting Approval] {}:\n{}", task.id, text),
                "waiting_approval",
            ),
            Ok(text) => (
                format!("[Task Completed] {}:\n{}", task.id, text),
                "completed",
            ),
            Err(e) => (format!("[Task Failed] {}:\n{}", task.id, e), "failed"),
        };

        let _ = match status {
            "completed" => self.db.mark_task_completed(&task.id),
            "waiting_approval" => self.db.mark_task_waiting_approval(&task.id),
            _ => self.db.mark_task_failed(&task.id),
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
            let _ = self
                .db
                .create_session_for_instance(&self.team_instance_id, &self.agent_id);
            session = self
                .db
                .get_latest_session_for_instance(&self.team_instance_id)
                .ok()
                .flatten();
        }
        if let Some(session) = session {
            let _ =
                self.db
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

    fn start_cross_team_run(&self, correlation_id: &str, goal: &str) -> Option<(String, String)> {
        let mut session = self
            .db
            .get_latest_session_for_instance(&self.team_instance_id)
            .ok()
            .flatten();
        if session.is_none() {
            let session_id = self
                .db
                .create_session_for_instance(&self.team_instance_id, &self.agent_id)
                .ok()?;
            session = self
                .db
                .list_sessions_for_instance(&self.team_instance_id)
                .ok()?
                .into_iter()
                .find(|candidate| candidate.id == session_id);
        }
        let session_id = session?.id;
        let actor_id =
            crate::application::orchestration::tool_gateway::LOCAL_DESKTOP_ACTOR_ID.to_string();
        self.db.ensure_local_security_owner(&actor_id).ok()?;
        let mode = self
            .db
            .get_setting("orchestration_mode")
            .ok()
            .flatten()
            .and_then(|value| {
                crate::application::orchestration::modes::OperatingMode::from_storage(&value)
            })
            .unwrap_or(crate::application::orchestration::modes::OperatingMode::HumanInteraction)
            .storage_value()
            .to_string();
        let run_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.db
            .create_orchestration_run(&crate::core::models::OrchestrationRunRecord {
                id: run_id.clone(),
                session_id: session_id.clone(),
                instance_id: self.team_instance_id.clone(),
                initiated_by: Some(actor_id),
                goal: goal.to_string(),
                mode,
                status: "running".to_string(),
                workflow_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            })
            .ok()?;
        let _ = self
            .db
            .insert_run_event(&crate::core::models::RunEventRecord {
                id: Uuid::new_v4().to_string(),
                run_id: run_id.clone(),
                event_type: "cross_team_run_created".to_string(),
                actor_type: "system".to_string(),
                actor_id: None,
                task_id: None,
                payload: Some(format!("correlation_id={}", correlation_id)),
                created_at: now,
            });
        Some((run_id, session_id))
    }

    fn governed_handoff_succeeded(&self, run_id: &str) -> bool {
        self.db
            .list_recent_run_events(Some(run_id), 100)
            .unwrap_or_default()
            .iter()
            .any(|event| {
                event.event_type == "tool_call_succeeded"
                    && event
                        .payload
                        .as_deref()
                        .unwrap_or_default()
                        .contains("handoff_to_team")
            })
    }

    fn finish_cross_team_run(&self, run_id: &str, result: &str, handoff_succeeded: bool) {
        let status = if result.starts_with("Approval required before executing") {
            "waiting_approval"
        } else if result.trim().is_empty() || !handoff_succeeded {
            "failed"
        } else {
            "completed"
        };
        let _ = self
            .db
            .update_orchestration_run_status(run_id, status, None);
        let _ = self
            .db
            .insert_run_event(&crate::core::models::RunEventRecord {
                id: Uuid::new_v4().to_string(),
                run_id: run_id.to_string(),
                event_type: format!("cross_team_run_{}", status),
                actor_type: "system".to_string(),
                actor_id: None,
                task_id: None,
                payload: None,
                created_at: chrono::Utc::now().to_rfc3339(),
            });
    }

    async fn execute_cross_team_review(
        &self,
        original_msg: &TeamMessage,
        handoff: CrossTeamHandoff,
    ) {
        let agent = match self.db.get_agent(&self.agent_id) {
            Ok(Some(a)) => a,
            _ => return,
        };

        let provider_config = self
            .db
            .get_provider_by_name(&agent.provider)
            .ok()
            .flatten()
            .or_else(|| {
                self.db.list_providers().ok().and_then(|providers| {
                    providers
                        .into_iter()
                        .find(|p| provider_kind(p) == agent.provider.as_str())
                })
            });
        let Some(provider_config) = provider_config else {
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

        let chat_service = crate::application::services::chat_service::ChatService::new(
            self.db.clone(),
            self.team_bus.clone(),
        );
        let mut sys = chat_service
            .build_dynamic_system_prompt(&team_id, &self.team_instance_id, &self.agent_id)
            .unwrap_or_default();

        sys.push_str("\n\nROLE: CRITIC\nYou are performing a cross-team review.\nYou MUST output a structured critique (numbered issues + concrete fixes).\nAfter writing the critique, you MUST respond to the requester by calling the tool handoff_to_team with handoff_type='review_response', correlation_id preserved, target_team=reply_to_team.\n");

        let correlation_id = if handoff.correlation_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            handoff.correlation_id.clone()
        };
        let Some((run_id, session_id)) =
            self.start_cross_team_run(&correlation_id, "Cross-team review request")
        else {
            return;
        };
        if let Some(case_id) = self.ensure_governed_case(&handoff) {
            let service =
                crate::application::orchestration::collaboration::CollaborationService::new(
                    self.db.clone(),
                );
            let _ = service.acknowledge_and_readback(
                &case_id,
                &self.agent_id,
                "I received the review request and will review only the submitted scope.",
                "[]",
                "[]",
                None,
            );
            if service.grant_readback_only(
                &case_id,
                &run_id,
                crate::application::orchestration::tool_gateway::LOCAL_DESKTOP_ACTOR_ID,
                &self.agent_id,
            ).is_err() {
                self.finish_cross_team_run(
                    &run_id,
                    "Delegated review denied: readback scope could not be persisted.",
                    false,
                );
                return;
            }
        }

        let user_text = format!(
            "Cross-team review request\ncorrelation_id: {}\nfrom_team: {}\noriginal_message: {}\n\nArtifact to review:\n{}{}",
            correlation_id,
            handoff.from_team,
            original_msg.content,
            handoff.briefing_package,
            handoff.context
                .as_ref()
                .map(|c| format!("\n\nAdditional Context:\n{}",
                    serde_json::to_string_pretty(c).unwrap_or_default()))
                .unwrap_or_default()
        );

        let history = vec![
            crate::providers::ChatMessage {
                role: "system".into(),
                content: sys.into(),
                parts: vec![],
                agent_name: Some(agent.name.clone().into()),
                thought_duration_secs: None,
            },
            crate::providers::ChatMessage {
                role: "user".into(),
                content: user_text.into(),
                parts: vec![],
                agent_name: None,
                thought_duration_secs: None,
            },
        ];

        let adapter: Option<Arc<dyn crate::providers::BaseProviderAdapter>> =
            match provider_kind(&provider_config) {
                "openrouter" => {
                    let mut a = crate::providers::openrouter::OpenRouterAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                "claude" => {
                    let mut a = crate::providers::claude::ClaudeAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                "gemini" => {
                    let mut a = crate::providers::gemini::GeminiAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                "codex" => {
                    let mut a = crate::providers::codex::CodexAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                "opencode" => {
                    let mut a = crate::providers::opencode::OpenCodeAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                _ => None,
            };

        let Some(adapter) = adapter else {
            return;
        };

        let mcp_registry = Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(
            self.db.clone(),
        ));
        let executor = crate::application::orchestration::executor::AgentExecutor::new(
            adapter,
            mcp_registry,
            self.db.clone(),
            self.team_bus.clone(),
            self.team_instance_id.clone(),
            agent.id.clone(),
            Some(session_id),
            Some(run_id.clone()),
            None,
            None,
        );

        let result = executor
            .execute_task(history)
            .await
            .ok()
            .unwrap_or_default();
        let handoff_succeeded = self.governed_handoff_succeeded(&run_id);
        self.finish_cross_team_run(&run_id, &result, handoff_succeeded);
        if handoff.reply_to_team.is_empty() {
            return;
        }

        if handoff_succeeded {
            self.emit_status_event(
                &correlation_id,
                &handoff.reply_to_team,
                "COMPLETED",
                "Cross-team review completed.",
            )
            .await;
        }
    }

    async fn execute_cross_team_message(
        &self,
        original_msg: &TeamMessage,
        handoff: CrossTeamHandoff,
    ) {
        let correlation_id = if handoff.correlation_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            handoff.correlation_id.clone()
        };
        let Some((run_id, session_id)) =
            self.start_cross_team_run(&correlation_id, "Cross-team execution request")
        else {
            return;
        };
        self.emit_status_event(
            &correlation_id,
            &handoff.reply_to_team,
            "ACK_RECEIVED",
            "Received cross-team handoff.",
        )
        .await;
        if let Some(case_id) = self.ensure_governed_case(&handoff) {
            let service =
                crate::application::orchestration::collaboration::CollaborationService::new(
                    self.db.clone(),
                );
            let _ = service.acknowledge_and_readback(
                &case_id,
                &self.agent_id,
                &format!("I understand the request: {}", handoff.briefing_package),
                "[]",
                "[]",
                None,
            );
            if service.grant_readback_only(
                &case_id,
                &run_id,
                crate::application::orchestration::tool_gateway::LOCAL_DESKTOP_ACTOR_ID,
                &self.agent_id,
            ).is_err() {
                self.finish_cross_team_run(
                    &run_id,
                    "Delegated execution denied: readback scope could not be persisted.",
                    false,
                );
                self.emit_status_event(
                    &correlation_id,
                    &handoff.reply_to_team,
                    "FAILED",
                    "Delegated scope could not be persisted; execution was blocked.",
                )
                .await;
                return;
            }
            self.emit_status_event(
                &correlation_id,
                &handoff.reply_to_team,
                "READBACK_SUBMITTED",
                "Readback submitted and awaiting acceptance before delegated execution.",
            )
            .await;
        }

        let agent = match self.db.get_agent(&self.agent_id) {
            Ok(Some(a)) => a,
            _ => return,
        };

        let provider_config = self
            .db
            .get_provider_by_name(&agent.provider)
            .ok()
            .flatten()
            .or_else(|| {
                self.db.list_providers().ok().and_then(|providers| {
                    providers
                        .into_iter()
                        .find(|p| provider_kind(p) == agent.provider.as_str())
                })
            });
        let Some(provider_config) = provider_config else {
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

        let chat_service = crate::application::services::chat_service::ChatService::new(
            self.db.clone(),
            self.team_bus.clone(),
        );
        let mut sys = chat_service
            .build_dynamic_system_prompt(&team_id, &self.team_instance_id, &self.agent_id)
            .unwrap_or_default();

        sys.push_str("\n\nCROSS-TEAM HANDOFF\nYou received a governed cross-team handoff. A readback has been submitted for human or policy acceptance. You MUST NOT delegate, modify artifacts, or send a completion handoff until the case context reports an accepted readback. If acceptance is absent, report that work is awaiting readback acceptance.\n");
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
            "Cross-team message\ncorrelation_id: {}\nfrom_instance: {}\noriginal_message: {}\n\nMessage:\n{}{}",
            correlation_id, handoff.from_team, original_msg.content, handoff.briefing_package,
            handoff.context
                .as_ref()
                .map(|c| format!("\n\nAdditional Context (deadline, constraints, related_files, etc.):\n{}",
                    serde_json::to_string_pretty(c).unwrap_or_default()))
                .unwrap_or_default()
        );

        let history = vec![
            crate::providers::ChatMessage {
                role: "system".into(),
                content: sys.into(),
                parts: vec![],
                agent_name: Some(agent.name.clone().into()),
                thought_duration_secs: None,
            },
            crate::providers::ChatMessage {
                role: "user".into(),
                content: user_text.into(),
                parts: vec![],
                agent_name: None,
                thought_duration_secs: None,
            },
        ];

        let adapter: Option<Arc<dyn crate::providers::BaseProviderAdapter>> =
            match provider_kind(&provider_config) {
                "openrouter" => {
                    let mut a = crate::providers::openrouter::OpenRouterAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                "claude" => {
                    let mut a = crate::providers::claude::ClaudeAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                "gemini" => {
                    let mut a = crate::providers::gemini::GeminiAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                "codex" => {
                    let mut a = crate::providers::codex::CodexAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                "opencode" => {
                    let mut a = crate::providers::opencode::OpenCodeAdapter::new();
                    if a.initialize(&provider_config).is_ok() {
                        Some(Arc::new(a))
                    } else {
                        None
                    }
                }
                _ => None,
            };

        let Some(adapter) = adapter else {
            return;
        };

        let mcp_registry = Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(
            self.db.clone(),
        ));
        let executor = crate::application::orchestration::executor::AgentExecutor::new(
            adapter,
            mcp_registry,
            self.db.clone(),
            self.team_bus.clone(),
            self.team_instance_id.clone(),
            agent.id.clone(),
            Some(session_id),
            Some(run_id.clone()),
            None,
            None,
        );

        let response_text = executor
            .execute_task(history)
            .await
            .ok()
            .unwrap_or_default();
        let handoff_succeeded = self.governed_handoff_succeeded(&run_id);
        self.finish_cross_team_run(&run_id, &response_text, handoff_succeeded);
        if handoff.reply_to_team.is_empty() || response_text.is_empty() {
            return;
        }

        if handoff_succeeded {
            self.emit_status_event(
                &correlation_id,
                &handoff.reply_to_team,
                "COMPLETED",
                "Cross-team execution completed.",
            )
            .await;
        }
    }

    async fn execute_iflow_task(&self, msg: TeamMessage) {
        let parts: Vec<&str> = msg.metadata.as_ref().unwrap().split(':').collect();
        if parts.len() < 3 {
            return;
        }

        let execution_id = parts[1];
        let node_id = parts[2];
        let instruction = &msg.content;
        let run_id = self
            .db
            .get_workflow_execution(execution_id)
            .ok()
            .flatten()
            .map(|execution| execution.run_id);
        let session_id = run_id.as_deref().and_then(|run_id| {
            self.db
                .get_orchestration_run(run_id)
                .ok()
                .flatten()
                .map(|run| run.session_id)
        });

        let agent = match self.db.get_agent(&self.agent_id) {
            Ok(Some(a)) => a,
            _ => return,
        };

        let provider_config = self
            .db
            .get_provider_by_name(&agent.provider)
            .ok()
            .flatten()
            .or_else(|| {
                self.db.list_providers().ok().and_then(|providers| {
                    providers
                        .into_iter()
                        .find(|p| provider_kind(p) == agent.provider.as_str())
                })
            });
        let Some(provider_config) = provider_config else {
            return;
        };

        let mut history = Vec::new();
        if let Some(system_prompt) = agent.system_prompt.clone() {
            history.push(crate::providers::ChatMessage {
                role: "system".into(),
                content: system_prompt.into(),
                parts: vec![],
                agent_name: Some(agent.name.clone().into()),
                thought_duration_secs: None,
            });
        }
        history.push(crate::providers::ChatMessage {
            role: "user".into(),
            content: instruction.clone().into(),
            parts: vec![],
            agent_name: Some(agent.name.clone().into()),
            thought_duration_secs: None,
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
            metadata: Some(format!(
                "{{\"workflow_execution_id\":\"{}\",\"node_id\":\"{}\"}}",
                execution_id, node_id
            )),
            delivery_status: "delivered".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let _ = self.db.insert_team_message(&stream_msg);
        let _ = self.team_bus.route_message(stream_msg.clone()).await;

        let output_text = if run_id.is_none() {
            "iFlow task denied: workflow execution is not linked to a traceable run.".to_string()
        } else {
            let adapter: Option<Arc<dyn crate::providers::BaseProviderAdapter>> =
                match provider_kind(&provider_config) {
                    "openrouter" => {
                        let mut adapter = crate::providers::openrouter::OpenRouterAdapter::new();
                        adapter.initialize(&provider_config).ok().map(|_| {
                            Arc::new(adapter) as Arc<dyn crate::providers::BaseProviderAdapter>
                        })
                    }
                    "claude" => {
                        let mut adapter = crate::providers::claude::ClaudeAdapter::new();
                        adapter.initialize(&provider_config).ok().map(|_| {
                            Arc::new(adapter) as Arc<dyn crate::providers::BaseProviderAdapter>
                        })
                    }
                    "gemini" => {
                        let mut adapter = crate::providers::gemini::GeminiAdapter::new();
                        adapter.initialize(&provider_config).ok().map(|_| {
                            Arc::new(adapter) as Arc<dyn crate::providers::BaseProviderAdapter>
                        })
                    }
                    "codex" => {
                        let mut adapter = crate::providers::codex::CodexAdapter::new();
                        adapter.initialize(&provider_config).ok().map(|_| {
                            Arc::new(adapter) as Arc<dyn crate::providers::BaseProviderAdapter>
                        })
                    }
                    "opencode" => {
                        let mut adapter = crate::providers::opencode::OpenCodeAdapter::new();
                        adapter.initialize(&provider_config).ok().map(|_| {
                            Arc::new(adapter) as Arc<dyn crate::providers::BaseProviderAdapter>
                        })
                    }
                    _ => None,
                };
            if let Some(adapter) = adapter {
                let db_for_stream = self.db.clone();
                let message_id_for_stream = message_id.clone();
                let agent_name_for_stream = agent.name.clone();
                let callback = Arc::new(move |text: String| {
                    let _ = db_for_stream.update_team_message_content(
                        &message_id_for_stream,
                        &format!("[{}]: {}", agent_name_for_stream, text),
                    );
                });
                let executor = crate::application::orchestration::executor::AgentExecutor::new(
                    adapter,
                    Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(
                        self.db.clone(),
                    )),
                    self.db.clone(),
                    self.team_bus.clone(),
                    self.team_instance_id.clone(),
                    agent.id.clone(),
                    session_id.clone(),
                    run_id.clone(),
                    None,
                    Some(callback),
                );
                match executor.execute_task(history).await {
                    Ok(text) => text,
                    Err(error) => format!("iFlow task failed: {}", error),
                }
            } else {
                "iFlow task failed: provider could not be initialized.".to_string()
            }
        };
        let _ = self.db.update_team_message_content(
            &message_id,
            &format!("[{}]: {}", agent.name, output_text),
        );
        if let Some(session_id) = session_id.as_deref() {
            let metadata = serde_json::json!({
                "agent_name": agent.name,
                "workflow_execution_id": execution_id,
                "node_id": node_id
            })
            .to_string();
            let _ = self.db.ensure_session(session_id, &agent.id, Some(&self.team_instance_id));
            let _ = self
                .db
                .append_conversation_turn(session_id, "assistant", &output_text, Some(&metadata));
            let _ = self.db.touch_session(session_id);
        }
        if output_text.starts_with("Approval required before executing") {
            if let Some(run_id) = run_id.as_deref() {
                let _ = self.db.update_orchestration_run_status(
                    run_id,
                    "waiting_approval",
                    None,
                );
                let _ = self.db.insert_run_event(&crate::core::models::RunEventRecord {
                    id: Uuid::new_v4().to_string(),
                    run_id: run_id.to_string(),
                    event_type: "workflow_node_waiting_approval".to_string(),
                    actor_type: "policy".to_string(),
                    actor_id: None,
                    task_id: None,
                    payload: Some(format!("execution_id={} node_id={}", execution_id, node_id)),
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
            }
            return;
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
    #[serde(default)]
    case_id: String,
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
            workers.retain(|_, handle| !handle.is_finished());
            for agent_id in agent_ids {
                let is_online = self
                    .db
                    .get_agent(&agent_id)
                    .ok()
                    .flatten()
                    .map(|agent| agent.status.to_lowercase() != "offline")
                    .unwrap_or(false);
                if !is_online {
                    continue;
                }

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
