use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use crate::application::orchestration::tool_gateway::ToolExecutionGateway;
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::security::audit::AuditLogger;

/// MCP Tool definition
#[derive(Clone, Debug)]
pub struct McpTool {
    pub id: String,
    pub server_id: Option<String>,
    pub name: String,
    pub description: String,
    pub version: String,
    pub command: String,
    pub args: Vec<String>,
    pub input_schema: String,
    pub is_active: bool,
}

#[derive(Clone, Debug)]
pub struct McpServerRecord {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub endpoint: Option<String>,
    pub env_secret_refs: String,
    pub header_secret_refs: String,
    pub source_kind: String,
    pub is_enabled: bool,
    pub health_status: String,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP Middleware for RBAC interception
pub struct McpAuthMiddleware {
    gateway: ToolExecutionGateway,
    audit_logger: tokio::sync::Mutex<AuditLogger>,
}

impl McpAuthMiddleware {
    pub async fn new(db: Arc<dyn DatabasePort>) -> Result<Self> {
        let logger = AuditLogger::new(db.clone()).await?;
        Ok(Self {
            gateway: ToolExecutionGateway::new(db),
            audit_logger: tokio::sync::Mutex::new(logger),
        })
    }

    /// Intercepts a tool call. Returns `Ok(true)` if allowed, `Ok(false)` if denied.
    pub async fn intercept_call(
        &self,
        _legacy_role_id: &str,
        user_id: Option<&str>,
        tool_name: &str,
    ) -> Result<bool> {
        let allowed = user_id
            .map(|actor_id| self.gateway.can_execute_interactively(actor_id, tool_name))
            .unwrap_or(false);

        let action = if allowed {
            "mcp_tool_execute_allowed"
        } else {
            "mcp_tool_execute_denied"
        };
        let details = format!(
            "Security actor {} attempted to execute tool {}",
            user_id.unwrap_or("missing"),
            tool_name
        );

        let mut logger = self.audit_logger.lock().await;
        let _ = logger.log(action, user_id, "mcp_tool", &details).await;

        Ok(allowed)
    }
}

/// MCP Tool Registration and Discovery
pub struct McpToolRegistry {
    db: Arc<dyn DatabasePort>,
}

impl McpToolRegistry {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self { db }
    }

    pub fn register_tool(&self, tool: McpTool) -> Result<()> {
        self.db.upsert_mcp_tool(&tool)
    }

    pub fn register_server(&self, server: &McpServerRecord) -> Result<()> {
        self.db.upsert_mcp_server(server)
    }

    pub fn unregister_tool(&self, tool_id: &str) -> Result<()> {
        self.db.delete_mcp_tool(tool_id)
    }

    pub fn list_tools(&self) -> Vec<McpTool> {
        self.db.list_mcp_tools().unwrap_or_default()
    }

    pub fn list_selected_tools(&self, run_id: Option<&str>) -> Vec<McpTool> {
        let mut selected_by_id = HashMap::new();
        for entry in self
            .db
            .list_capability_selections("default", "")
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| entry.capability_kind == "mcp_tool")
        {
            selected_by_id.insert(entry.capability_id.clone(), entry);
        }
        if let Some(run_id) = run_id {
            for entry in self
                .db
                .list_capability_selections("run", run_id)
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| entry.capability_kind == "mcp_tool")
            {
                selected_by_id.insert(entry.capability_id.clone(), entry);
            }
        }
        let selected_ids = selected_by_id
            .into_iter()
            .filter(|(_, entry)| entry.enabled)
            .map(|(capability_id, _)| capability_id)
            .collect::<std::collections::HashSet<_>>();
        let enabled_server_ids = self
            .list_servers()
            .into_iter()
            .filter(|server| server.is_enabled)
            .map(|server| server.id)
            .collect::<std::collections::HashSet<_>>();
        self.list_tools()
            .into_iter()
            .filter(|tool| {
                tool.is_active
                    && selected_ids.contains(&tool.id)
                    && tool
                        .server_id
                        .as_ref()
                        .is_some_and(|server_id| enabled_server_ids.contains(server_id))
            })
            .collect()
    }

    pub fn list_servers(&self) -> Vec<McpServerRecord> {
        self.db.list_mcp_servers().unwrap_or_default()
    }

    pub fn get_tool(&self, tool_id: &str) -> Option<McpTool> {
        self.db.get_mcp_tool(tool_id).unwrap_or(None)
    }
}
