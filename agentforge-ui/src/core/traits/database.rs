
pub trait DatabasePort: Send + Sync {
    fn seed_provider_templates(&self) -> anyhow::Result<()>;
    fn list_provider_templates(&self) -> anyhow::Result<Vec<crate::core::models::ProviderTemplate>>;
    fn insert_provider(&self, p: &crate::core::models::Provider) -> anyhow::Result<()>;
    fn list_providers(&self) -> anyhow::Result<Vec<crate::core::models::Provider>>;
    fn get_provider_by_name(&self, provider_name: &str) -> anyhow::Result<Option<crate::core::models::Provider>>;
    fn insert_team(&self, team: &crate::core::models::Team) -> anyhow::Result<()>;
    fn create_instance(&self, id: &str, team_id: &str, config: Option<&str>, state: Option<&str>,) -> anyhow::Result<()>;
    fn list_instances(&self) -> anyhow::Result<Vec<crate::core::models::Instance>>;
    fn list_teams(&self) -> anyhow::Result<Vec<crate::core::models::Team>>;
    fn insert_agent(&self, agent: &crate::core::models::Agent) -> anyhow::Result<()>;
    fn list_agents(&self) -> anyhow::Result<Vec<crate::core::models::Agent>>;
    fn assign_agent_to_team(&self, team_id: &str, agent_id: &str) -> anyhow::Result<()>;
    fn remove_agent_from_team(&self, team_id: &str, agent_id: &str) -> anyhow::Result<()>;
    fn get_team_agents(&self, team_id: &str) -> anyhow::Result<Vec<String>>;
    fn get_instance_agents(&self, instance_id: &str) -> anyhow::Result<Vec<String>>;
    fn get_instance_agent_name_mapping(&self, instance_id: &str) -> anyhow::Result<std::collections::HashMap<String, String>>;
    fn upsert_task(&self, id: &str, team_id: &str, instance_id: Option<&str>, priority: &str, payload: Option<&str>,) -> anyhow::Result<()>;
    fn get_total_tokens_per_agent(&self) -> anyhow::Result<Vec<(String, usize)>>;
    fn get_total_tokens_per_instance(&self) -> anyhow::Result<Vec<(String, usize)>>;
    fn get_agent_instance_count(&self) -> anyhow::Result<Vec<(String, usize)>>;
    fn get_total_daily_tokens(&self) -> anyhow::Result<usize>;
    fn get_total_tasks_completed(&self) -> anyhow::Result<usize>;
    fn get_active_agents_count(&self) -> anyhow::Result<usize>;
    fn insert_token_usage(&self, instance_id: Option<&str>, agent_id: &str, input_tokens: usize, output_tokens: usize, total_tokens: usize) -> anyhow::Result<()>;
    fn assign_task_to_agent(&self, task_id: &str, agent_id: &str) -> anyhow::Result<()>;
    fn list_tasks_for_instance(&self, instance_id: &str,) -> anyhow::Result<Vec<crate::application::tasks::shared_task_list::Task>>;
    fn list_pending_tasks_for_instance(&self, instance_id: &str, limit: u32,) -> anyhow::Result<Vec<crate::application::tasks::shared_task_list::Task>>;
    fn claim_task_for_instance(&self, task_id: &str, agent_id: &str, instance_id: &str) -> anyhow::Result<bool>;
    fn mark_task_completed(&self, task_id: &str) -> anyhow::Result<()>;
    fn mark_task_failed(&self, task_id: &str) -> anyhow::Result<()>;
    fn seed_sdg_team(&self) -> anyhow::Result<()>;
    fn get_agent(&self, agent_id: &str) -> anyhow::Result<Option<crate::core::models::Agent>>;
    fn insert_team_message(&self, msg: &crate::infrastructure::message_bus::routing::TeamMessage) -> anyhow::Result<()>;
    fn get_team_messages_for_instance(&self, team_instance_id: &str, limit: u32,) -> anyhow::Result<Vec<crate::infrastructure::message_bus::routing::TeamMessage>>;
    fn get_team_messages_for_instance_by_type(&self, team_instance_id: &str, message_type: crate::infrastructure::message_bus::routing::MessageType, limit: u32,) -> anyhow::Result<Vec<crate::infrastructure::message_bus::routing::TeamMessage>>;
    fn update_team_message_delivery_status(&self, message_id: &str, status: &str) -> anyhow::Result<()>;
    fn update_team_message_content(&self, message_id: &str, content: &str) -> anyhow::Result<()>;
    fn append_conversation_turn(&self, session_id: &str, role: &str, content: &str, metadata: Option<&str>,) -> anyhow::Result<()>;
    fn ensure_session(&self, session_id: &str, agent_id: &str, team_instance_id: Option<&str>,) -> anyhow::Result<()>;
    fn create_session_for_instance(&self, instance_id: &str, agent_id: &str) -> anyhow::Result<String>;
    fn list_sessions_for_instance(&self, instance_id: &str) -> anyhow::Result<Vec<crate::core::models::SessionRecord>>;
    fn get_latest_session_for_instance(&self, instance_id: &str) -> anyhow::Result<Option<crate::core::models::SessionRecord>>;
    fn touch_session(&self, session_id: &str) -> anyhow::Result<()>;
    fn get_conversation_turns(&self, session_id: &str,) -> anyhow::Result<Vec<crate::core::models::ChatMessage>>;
    fn save_message(&self, team_id: &str, instance_id: Option<&str>, role: &str, content: &str) -> anyhow::Result<()>;
    fn get_messages(&self, team_id: &str, instance_id: Option<&str>) -> anyhow::Result<Vec<crate::core::models::ChatMessage>>;
    fn upsert_knowledge_item(&self, item: &crate::core::models::KnowledgeItem) -> anyhow::Result<()>;
    fn get_all_knowledge_items(&self) -> anyhow::Result<Vec<crate::core::models::KnowledgeItem>>;
    fn set_setting(&self, key: &str, value: &str) -> anyhow::Result<()>;
    fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>>;
    fn get_recent_workspaces(&self) -> anyhow::Result<Vec<String>>;
    fn search_knowledge(&self, query: &str) -> anyhow::Result<Vec<crate::core::models::KnowledgeItem>>;
    fn search_knowledge_fts(&self, query: &str, limit: u32) -> anyhow::Result<Vec<crate::core::models::KnowledgeItem>>;
    fn upsert_knowledge_chunks(&self, document_id: &str, chunks: Vec<(usize, String, Vec<f32>)>,) -> anyhow::Result<()>;
    fn search_similar_chunks(&self, query_embedding: &[f32], limit: usize) -> anyhow::Result<Vec<(String, String, f32)>>;

    fn upsert_workflow(&self, wf: &crate::core::models::WorkflowRecord) -> anyhow::Result<()>;
    fn list_workflows(&self) -> anyhow::Result<Vec<crate::core::models::WorkflowRecord>>;
    fn get_workflow(&self, workflow_id: &str) -> anyhow::Result<Option<crate::core::models::WorkflowRecord>>;

    fn save_workflow_state(&self, state: &crate::application::iflow_engine::engine::WorkflowState) -> anyhow::Result<()>;
    fn load_workflow_state(&self, execution_id: &str) -> anyhow::Result<Option<crate::application::iflow_engine::engine::WorkflowState>>;
    fn insert_audit_log(&self, event: &crate::infrastructure::security::audit::AuditEvent) -> anyhow::Result<()>;
    fn create_role(&self, role: &crate::application::teams::role::Role) -> anyhow::Result<()>;
    fn update_role_permissions(&self, role_id: &str, permissions: &str) -> anyhow::Result<()>;
    fn check_role_permission(&self, role_id: &str, required_permission: &str) -> anyhow::Result<bool>;

    // MCP Tools
    fn upsert_mcp_tool(&self, tool: &crate::infrastructure::mcp::registry::McpTool) -> anyhow::Result<()>;
    fn get_mcp_tool(&self, id: &str) -> anyhow::Result<Option<crate::infrastructure::mcp::registry::McpTool>>;
    fn list_mcp_tools(&self) -> anyhow::Result<Vec<crate::infrastructure::mcp::registry::McpTool>>;
    fn delete_mcp_tool(&self, id: &str) -> anyhow::Result<()>;

    // Knowledge Entries (Long-term memory)
    fn upsert_knowledge_entry(&self, entry: &crate::core::models::knowledge::KnowledgeEntry) -> anyhow::Result<()>;
    fn get_knowledge_entry(&self, id: &str) -> anyhow::Result<Option<crate::core::models::knowledge::KnowledgeEntry>>;
    fn search_knowledge_entries_fts(&self, query: &str, limit: u32) -> anyhow::Result<Vec<crate::core::models::knowledge::KnowledgeEntry>>;
}
