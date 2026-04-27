use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::application::teams::role::RoleManager;
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::security::audit::AuditLogger;

/// MCP Tool definition
#[derive(Clone, Debug)]
pub struct McpTool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub command: String,
    pub args: Vec<String>,
    pub input_schema: String,
    pub is_active: bool,
}

/// MCP Middleware for RBAC interception
pub struct McpAuthMiddleware {
    role_manager: RoleManager,
    audit_logger: tokio::sync::Mutex<AuditLogger>,
}

impl McpAuthMiddleware {
    pub async fn new(db: Arc<dyn DatabasePort>) -> Result<Self> {
        let logger = AuditLogger::new(db.clone()).await?;
        Ok(Self {
            role_manager: RoleManager::new(db),
            audit_logger: tokio::sync::Mutex::new(logger),
        })
    }

    /// Intercepts a tool call. Returns `Ok(true)` if allowed, `Ok(false)` if denied.
    pub async fn intercept_call(&self, role_id: &str, user_id: Option<&str>, tool_name: &str) -> Result<bool> {
        let required_permission = format!("mcp:execute:{}", tool_name);
        
        let has_all = self.role_manager.check_permission(role_id, "all").unwrap_or(false);
        let has_mcp_all = self.role_manager.check_permission(role_id, "mcp:execute:all").unwrap_or(false);
        let has_specific = self.role_manager.check_permission(role_id, &required_permission).unwrap_or(false);
        
        let allowed = has_all || has_mcp_all || has_specific;
        
        let action = if allowed { "mcp_tool_execute_allowed" } else { "mcp_tool_execute_denied" };
        let details = format!("Role {} attempted to execute tool {}", role_id, tool_name);
        
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

    pub fn unregister_tool(&self, tool_id: &str) -> Result<()> {
        self.db.delete_mcp_tool(tool_id)
    }

    pub fn list_tools(&self) -> Vec<McpTool> {
        self.db.list_mcp_tools().unwrap_or_default()
    }

    pub fn get_tool(&self, tool_id: &str) -> Option<McpTool> {
        self.db.get_mcp_tool(tool_id).unwrap_or(None)
    }
}
