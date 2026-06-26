use crate::application::orchestration::role_policy;
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::message_bus::routing::TeamBusRouter;
use std::sync::Arc;

pub struct ChatService {
    db: Arc<dyn DatabasePort>,
}

impl ChatService {
    pub fn new(db: Arc<dyn DatabasePort>, _team_bus: Arc<TeamBusRouter>) -> Self {
        Self { db }
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
        let mut instance_context = "No instance-specific context configured.".to_string();
        if let Ok(instances) = self.db.list_instances() {
            if let Some(i) = instances.iter().find(|i| i.id == instance_id) {
                instance_name = i.name.clone();
                if let Some(config) = i
                    .config
                    .as_deref()
                    .map(str::trim)
                    .filter(|config| !config.is_empty())
                {
                    instance_context = config.to_string();
                }
            }
        }

        let mut members_str = String::new();
        let mut peer_instances_str = String::new();
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
                            "- {} (THIS IS YOU)\n",
                            role_policy::agent_profile_for_prompt(&a)
                        ));
                    } else {
                        members_str.push_str(&format!(
                            "- {}\n",
                            role_policy::agent_profile_for_prompt(&a)
                        ));
                    }
                }
            }
        }

        if let Ok(instances) = self.db.list_instances() {
            for instance in instances
                .into_iter()
                .filter(|instance| instance.team_id == team_id && instance.id != instance_id)
            {
                let mut roster = Vec::new();
                if let Ok(agent_ids) = conn.get_instance_agents(&instance.id) {
                    for aid in agent_ids {
                        if let Ok(Some(agent)) = conn.get_agent(&aid) {
                            roster.push(role_policy::agent_profile_for_prompt(&agent));
                        }
                    }
                }
                if !roster.is_empty() {
                    peer_instances_str.push_str(&format!(
                        "- {} ({})\n  agents: {}\n",
                        instance.name,
                        instance.id,
                        roster.join(" || ")
                    ));
                }
            }
        }
        if peer_instances_str.trim().is_empty() {
            peer_instances_str.push_str("No peer instances with visible agents.\n");
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
             Instance Context: {}\n\
             Your Agent Name: {}\n\
             Your Routing Role: {}\n\
             Your Professional Position: {}\n\
             Your Profile Details: {}\n\
             Team Members Available:\n{}\n\
             Peer Instances Available For Handoff:\n{}\n\
             IMPORTANT ROLE RULES:\n\
             - You are agent '{}' with routing role '{}'. This routing role is the exact TeamBus/task-assignment key for your work.\n\
             - You MUST act only within your professional position, responsibilities, competencies, allowed_task_types, disallowed_task_types, and profile details. Do not claim to be another member.\n\
             - When delegating with create_subtasks or sending role-targeted messages, use the exact routing_role values listed above.\n\
             - Coordinator is mandatory for every instance and may coordinate, read, search, review, and delegate, but must not create, edit, delete, or run side-effectful artifact operations.\n\
             - Cross-team communication is owned by the Coordinator of each instance. Non-Coordinator agents contribute through internal subtasks or role-targeted messages from their Coordinator; they must not independently represent the instance in cross-team handoff responses.\n\
             - Assign work by task competency, not by generic non-Coordinator status. Examples: implementation/build/testing work needs an implementation-capable agent; design work needs a design-capable agent; content work needs a content-capable agent; marketing work needs a marketing-capable agent; analysis/documentation work needs matching analyst/documentation competency.\n\
             - If this instance lacks a matching competency, do not force the task onto PM/BA/designer/marketer/content/other unrelated agents. Use handoff_to_team with a clear briefing package to a peer instance that has a matching agent, or raise escalation if none exists.\n\
             - Adapt your answers to the '{}' team and current instance. Ignore hardcoded or hallucinated team names in the base prompt.\n\
             - If a request is outside your role, state the boundary, then hand off or recommend the correct role.\n\
             CROSS-TEAM REVIEW PROTOCOL:\n\
             - If you receive a message that starts with '[CROSS_TEAM_HANDOFF]', treat it as a cross-team request owned by this instance's Coordinator.\n\
             - If you are Coordinator and the payload indicates handoff_type='review_request', collect or synthesize structured critique, then respond by calling the tool handoff_to_team with:\n\
               target_team = reply_to_team, handoff_type='review_response', correlation_id = the same id, briefing_package = the coordinated critique.\n\
             -------------------------------\n\n\
             Base Prompt:\n{}",
            team_name,
            instance_name,
            instance_context,
            my_name,
            my_role,
            my_position,
            my_details,
            members_str,
            peer_instances_str,
            my_name,
            my_role,
            team_name,
            clean_prompt
        );

        Some(dynamic_prompt)
    }

    pub fn parse_generated_response(
        &self,
        text: &str,
        _workspace_dir: Option<&String>,
    ) -> (Vec<String>, String) {
        // Markdown is display content only. File side effects must use governed tools.
        (Vec::new(), text.to_string())
    }
}
