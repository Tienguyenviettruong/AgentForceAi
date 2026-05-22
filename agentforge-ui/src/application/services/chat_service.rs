use crate::core::traits::database::DatabasePort;
use crate::infrastructure::message_bus::routing::TeamBusRouter;
use std::sync::Arc;

pub struct ChatService {
    db: Arc<dyn DatabasePort>,
    team_bus: Arc<TeamBusRouter>,
}

impl ChatService {
    pub fn new(db: Arc<dyn DatabasePort>, team_bus: Arc<TeamBusRouter>) -> Self {
        Self { db, team_bus }
    }

    pub fn build_dynamic_system_prompt(
        &self,
        team_id: &str,
        instance_id: &str,
        agent_id: &str,
    ) -> Option<String> {
        let mut team_name = "Unknown Team".to_string();
        if let Ok(teams) = self.db.list_teams() {
            if let Some(t) = teams.iter().find(|t| t.id == team_id) {
                team_name = t.name.clone();
            }
        }

        let mut instance_name = "Unknown Instance".to_string();
        if let Ok(instances) = self.db.list_instances() {
            if let Some(i) = instances.iter().find(|i| i.id == instance_id) {
                instance_name = i.name.clone();
            }
        }

        let mut members_str = String::new();
        let mut my_name = "Agent".to_string();
        let mut my_role = "Agent".to_string();
        let mut my_position = "Agent".to_string();
        let mut my_details = "No additional profile details configured.".to_string();

        let conn = self.db.clone();
        if let Ok(agent_ids) = conn.get_instance_agents(instance_id) {
            for aid in agent_ids {
                if let Ok(Some(a)) = conn.get_agent(&aid) {
                    let member_role = a.routing_role();
                    let member_position = a.profile_position();
                    if aid == agent_id {
                        my_name = a.name.clone();
                        my_role = member_role.clone();
                        my_position = member_position.clone();
                        if let Some(details) = a.profile_details() {
                            my_details = details;
                        }
                        members_str.push_str(&format!(
                            "- {} | routing_role={} | position={} (THIS IS YOU)\n",
                            a.name, member_role, member_position
                        ));
                    } else {
                        members_str.push_str(&format!(
                            "- {} | routing_role={} | position={}\n",
                            a.name, member_role, member_position
                        ));
                    }
                }
            }
        }

        let agent = self.db.get_agent(agent_id).ok().flatten()?;
        let base_prompt = agent.system_prompt.unwrap_or_default();

        let clean_prompt = base_prompt
            .replace("SDG team", &format!("{} team", team_name))
            .replace("SDG", &team_name);

        let dynamic_prompt = format!(
            "--- SYSTEM CONTEXT OVERRIDE ---\n\
             Current Team: {}\n\
             Current Instance: {}\n\
             Your Agent Name: {}\n\
             Your Routing Role: {}\n\
             Your Professional Position: {}\n\
             Your Profile Details: {}\n\
             Team Members Available:\n{}\n\
             IMPORTANT ROLE RULES:\n\
             - You are agent '{}' with routing role '{}'. This routing role is the exact TeamBus/task-assignment key for your work.\n\
             - You MUST act only within your professional position and profile details. Do not claim to be another member.\n\
             - When delegating with create_subtasks or sending role-targeted messages, use the exact routing_role values listed above.\n\
             - Adapt your answers to the '{}' team and current instance. Ignore hardcoded or hallucinated team names in the base prompt.\n\
             - If a request is outside your role, state the boundary, then hand off or recommend the correct role.\n\
             CROSS-TEAM REVIEW PROTOCOL:\n\
             - If you receive a message that starts with '[CROSS_TEAM_HANDOFF]', treat it as a cross-team request.\n\
             - If the payload indicates handoff_type='review_request', you must write a structured critique and respond by calling the tool handoff_to_team with:\n\
               target_team = reply_to_team, handoff_type='review_response', correlation_id = the same id, briefing_package = your critique.\n\
             -------------------------------\n\n\
             Base Prompt:\n{}",
            team_name,
            instance_name,
            my_name,
            my_role,
            my_position,
            my_details,
            members_str,
            my_name,
            my_role,
            team_name,
            clean_prompt
        );

        Some(dynamic_prompt)
    }

    pub fn parse_and_write_files(
        &self,
        text: &str,
        workspace_dir: Option<&String>,
    ) -> (Vec<String>, String) {
        let mut files_written = Vec::new();
        let mut current_text = text.to_string();

        loop {
            // Handle ```file:<path> — write entire file
            if let Some(start_idx) = current_text.find("```file:") {
                let rest = &current_text[start_idx + 8..];
                if let Some(newline_idx) = rest.find('\n') {
                    let filepath = rest[..newline_idx].trim().to_string();
                    let file_content_start = &rest[newline_idx + 1..];
                    if let Some(end_idx) = file_content_start.find("```") {
                        let file_content = &file_content_start[..end_idx];

                        let resolved_path = self.resolve_path(&filepath, workspace_dir);

                        if let Some(parent) = resolved_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }

                        if std::fs::write(&resolved_path, file_content).is_ok() {
                            let path_str = resolved_path.to_string_lossy().to_string();
                            files_written.push(path_str.clone());
                            // Broadcast FILE_WRITTEN event so other agents/panels can react
                            let team_bus = self.team_bus.clone();
                            tokio::spawn(async move {
                                use crate::infrastructure::message_bus::routing::TeamMessage;
                                let mut event = TeamMessage::new_broadcast(
                                    "global".to_string(),
                                    "system".to_string(),
                                    format!("[FILE_WRITTEN] {}", path_str),
                                );
                                event.metadata = Some(
                                    serde_json::json!({"event": "file_written", "path": path_str}).to_string()
                                );
                                let _ = team_bus.route_message(event).await;
                            });
                        }

                        let block_end = start_idx + 8 + newline_idx + 1 + end_idx + 3;
                        current_text = current_text[block_end..].to_string();
                        continue;
                    }
                }
            }
            // Handle ```edit:<path> — find-replace in existing file
            // Format: ```edit:<path>\n<<<FIND>>>\nold text\n<<<REPLACE>>>\nnew text\n```
            else if let Some(start_idx) = current_text.find("```edit:") {
                let rest = &current_text[start_idx + 8..];
                if let Some(newline_idx) = rest.find('\n') {
                    let filepath = rest[..newline_idx].trim().to_string();
                    let edit_content_start = &rest[newline_idx + 1..];
                    if let Some(end_idx) = edit_content_start.find("```") {
                        let edit_block = &edit_content_start[..end_idx];

                        let resolved_path = self.resolve_path(&filepath, workspace_dir);

                        // Parse <<<FIND>>> and <<<REPLACE>>> markers
                        if let (Some(find_start), Some(replace_marker)) = (
                            edit_block.find("<<<FIND>>>"),
                            edit_block.find("<<<REPLACE>>>"),
                        ) {
                            let find_text = edit_block[find_start + 10..replace_marker].trim();
                            let replace_text = edit_block[replace_marker + 13..].trim();

                            if let Ok(existing_content) = std::fs::read_to_string(&resolved_path) {
                                if existing_content.contains(find_text) {
                                    let new_content =
                                        existing_content.replacen(find_text, replace_text, 1);
                                    if std::fs::write(&resolved_path, &new_content).is_ok() {
                                        let path_str = format!("(edited) {}", resolved_path.display());
                                        files_written.push(path_str.clone());
                                        // Broadcast FILE_EDITED event
                                        let team_bus = self.team_bus.clone();
                                        let abs_path = resolved_path.to_string_lossy().to_string();
                                        tokio::spawn(async move {
                                            use crate::infrastructure::message_bus::routing::TeamMessage;
                                            let mut event = TeamMessage::new_broadcast(
                                                "global".to_string(),
                                                "system".to_string(),
                                                format!("[FILE_EDITED] {}", abs_path),
                                            );
                                            event.metadata = Some(
                                                serde_json::json!({"event": "file_edited", "path": abs_path}).to_string()
                                            );
                                            let _ = team_bus.route_message(event).await;
                                        });
                                    }
                                }
                            }
                        }

                        let block_end = start_idx + 8 + newline_idx + 1 + end_idx + 3;
                        current_text = current_text[block_end..].to_string();
                        continue;
                    }
                }
            }
            break;
        }

        (files_written, current_text)
    }

    fn resolve_path(&self, filepath: &str, workspace_dir: Option<&String>) -> std::path::PathBuf {
        let path = std::path::Path::new(filepath);
        if path.is_relative() {
            if let Some(ws) = workspace_dir {
                let mut full_path = std::path::PathBuf::from(ws);
                full_path.push(path);
                full_path
            } else {
                path.to_path_buf()
            }
        } else {
            path.to_path_buf()
        }
    }
}
