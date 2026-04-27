use crate::mcp::registry::{McpTool, McpToolRegistry};

pub fn register_team_tools(registry: &McpToolRegistry) -> anyhow::Result<()> {
    // 2.25 Implement team_message_role MCP tool
    registry.register_tool(McpTool {
        id: "mcp-team-msg-role".to_string(),
        name: "team_message_role".to_string(),
        description: "Send a message to a specific team role".to_string(),
        version: "1.0.0".to_string(),
        command: "agentforge-cli".to_string(),
        args: vec!["team".to_string(), "message-role".to_string()],
        input_schema: "{}".to_string(),
        is_active: true,
    })?;

    // 2.26 Implement team_broadcast MCP tool
    registry.register_tool(McpTool {
        id: "mcp-team-broadcast".to_string(),
        name: "team_broadcast".to_string(),
        description: "Broadcast a message to all team members".to_string(),
        version: "1.0.0".to_string(),
        command: "agentforge-cli".to_string(),
        args: vec!["team".to_string(), "broadcast".to_string()],
        input_schema: "{}".to_string(),
        is_active: true,
    })?;

    // 2.27 Implement team_claim_task MCP tool
    registry.register_tool(McpTool {
        id: "mcp-team-claim-task".to_string(),
        name: "team_claim_task".to_string(),
        description: "Atomically claim a task from the shared task list".to_string(),
        version: "1.0.0".to_string(),
        command: "agentforge-cli".to_string(),
        args: vec!["team".to_string(), "claim-task".to_string()],
        input_schema: "{}".to_string(),
        is_active: true,
    })?;

    // 2.28 Implement team_complete_task MCP tool
    registry.register_tool(McpTool {
        id: "mcp-team-complete-task".to_string(),
        name: "team_complete_task".to_string(),
        description: "Mark a claimed task as done".to_string(),
        version: "1.0.0".to_string(),
        command: "agentforge-cli".to_string(),
        args: vec!["team".to_string(), "complete-task".to_string()],
        input_schema: "{}".to_string(),
        is_active: true,
    })?;

    // 2.29 Implement team_get_tasks MCP tool
    registry.register_tool(McpTool {
        id: "mcp-team-get-tasks".to_string(),
        name: "team_get_tasks".to_string(),
        description: "Query available tasks in the team workspace".to_string(),
        version: "1.0.0".to_string(),
        command: "agentforge-cli".to_string(),
        args: vec!["team".to_string(), "get-tasks".to_string()],
        input_schema: "{}".to_string(),
        is_active: true,
    })?;

    Ok(())
}
