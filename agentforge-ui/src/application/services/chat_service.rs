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
                instance_name = i.id.clone();
            }
        }

        let mut members_str = String::new();
        let mut my_role = "Agent".to_string();
        
        let conn = self.db.clone();
        if let Ok(agent_ids) = conn.get_instance_agents(instance_id) {
            for aid in agent_ids {
                if let Ok(Some(a)) = conn.get_agent(&aid) {
                    if aid == agent_id {
                        my_role = a.name.clone();
                        members_str.push_str(&format!("- {} (THIS IS YOU)\n", a.name));
                    } else {
                        members_str.push_str(&format!("- {}\n", a.name));
                    }
                }
            }
        }

        let agent = self.db.get_agent(agent_id).ok().flatten()?;
        let base_prompt = agent.system_prompt.unwrap_or_default();

        let clean_prompt = base_prompt.replace("SDG team", &format!("{} team", team_name))
                                      .replace("SDG", &team_name);

        let dynamic_prompt = format!(
            "--- SYSTEM CONTEXT OVERRIDE ---\n\
             Current Team: {}\n\
             Current Instance: {}\n\
             Your Name/Role in this team: {}\n\
             Team Members Available:\n{}\n\
             IMPORTANT: You are '{}'. You MUST NOT act as or claim to be any other member (e.g. do not act as the Coordinator if you are a BA). You must adapt your role to the '{}' team. Ignore any hardcoded team names (like 'SDG') or hallucinated roles in your base prompt. Only collaborate with the members listed above.\n\
             CROSS-TEAM REVIEW PROTOCOL:\n\
             - If you receive a message that starts with '[CROSS_TEAM_HANDOFF]', treat it as a cross-team request.\n\
             - If the payload indicates handoff_type='review_request', you must write a structured critique and respond by calling the tool handoff_to_team with:\n\
               target_team = reply_to_team, handoff_type='review_response', correlation_id = the same id, briefing_package = your critique.\n\
             -------------------------------\n\n\
             Base Prompt:\n{}",
            team_name, instance_name, my_role, members_str, my_role, team_name, clean_prompt
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
            if let Some(start_idx) = current_text.find("```file:") {
                let rest = &current_text[start_idx + 8..];
                if let Some(newline_idx) = rest.find('\n') {
                    let mut filepath = rest[..newline_idx].trim().to_string();
                    let file_content_start = &rest[newline_idx + 1..];
                    if let Some(end_idx) = file_content_start.find("```") {
                        let file_content = &file_content_start[..end_idx];

                        let path = std::path::Path::new(&filepath);
                        let resolved_path = if path.is_relative() {
                            if let Some(ws) = workspace_dir {
                                let mut full_path = std::path::PathBuf::from(ws);
                                full_path.push(path);
                                full_path
                            } else {
                                path.to_path_buf()
                            }
                        } else {
                            path.to_path_buf()
                        };
                        filepath = resolved_path.to_string_lossy().to_string();

                        if let Some(parent) = resolved_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }

                        if std::fs::write(&resolved_path, file_content).is_ok() {
                            files_written.push(filepath);
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
}
