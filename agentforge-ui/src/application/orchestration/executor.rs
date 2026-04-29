use std::sync::Arc;
use crate::core::traits::database::DatabasePort;
use crate::providers::BaseProviderAdapter;
use crate::core::models::chat::ChatMessage;
use crate::infrastructure::mcp::registry::McpToolRegistry;
use crate::infrastructure::message_bus::routing::TeamBusRouter;
use anyhow::Result;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

pub struct AgentExecutor {
    provider: Arc<dyn BaseProviderAdapter>,
    mcp_registry: Arc<McpToolRegistry>,
    db: Arc<dyn DatabasePort>,
    team_bus: Arc<TeamBusRouter>,
    team_instance_id: String,
    agent_id: String,
}

impl AgentExecutor {
    pub fn new(
        provider: Arc<dyn BaseProviderAdapter>,
        mcp_registry: Arc<McpToolRegistry>,
        db: Arc<dyn DatabasePort>,
        team_bus: Arc<TeamBusRouter>,
        team_instance_id: String,
        agent_id: String,
    ) -> Self {
        Self { provider, mcp_registry, db, team_bus, team_instance_id, agent_id }
    }

    pub async fn execute_task(&self, mut history: Vec<ChatMessage>) -> Result<String> {
        let mut iteration = 0;
        let max_iterations = 5;

        // Apply Sliding Window context pruning
        history = self.prune_history(&history, 20);

        // 1. Tool Injection
        let mut tools_json = serde_json::json!({
            "tools": [
                {
                    "name": "save_to_knowledge",
                    "description": "Save important long-term information to the knowledge base.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" },
                            "content": { "type": "string" }
                        },
                        "required": ["title", "content"]
                    }
                },
                {
                    "name": "declare_consensus",
                    "description": "Declare that a consensus has been reached and broadcast it to the team.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "message": { "type": "string" }
                        },
                        "required": ["message"]
                    }
                },
                {
                    "name": "handoff_to_team",
                    "description": "Handoff the task to another team.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target_team": { "type": "string" },
                            "briefing_package": { "type": "string" },
                            "handoff_type": { "type": "string", "description": "e.g. review_request, review_response, handoff" },
                            "correlation_id": { "type": "string", "description": "optional id to link request/response" },
                            "reply_to_team": { "type": "string", "description": "if set, receiver should reply to this team instance id" }
                        },
                        "required": ["target_team", "briefing_package"]
                    }
                },
                {
                    "name": "create_subtasks",
                    "description": "Create subtasks and dispatch them to agents with specific roles.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "tasks": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "description": { "type": "string" },
                                        "role": { "type": "string" }
                                    },
                                    "required": ["description", "role"]
                                }
                            }
                        },
                        "required": ["tasks"]
                    }
                },
                {
                    "name": "web_search",
                    "description": "Search the web for information.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string" }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "run_cli",
                    "description": "Run a shell command on the host machine.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string" },
                            "args": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        },
                        "required": ["command", "args"]
                    }
                }
            ]
        });

        // Add MCP tools
        let mcp_tools = self.mcp_registry.list_tools();
        if let Some(tools_arr) = tools_json["tools"].as_array_mut() {
            for mcp_tool in mcp_tools {
                if let Ok(schema) = serde_json::from_str::<serde_json::Value>(&mcp_tool.input_schema) {
                    tools_arr.push(serde_json::json!({
                        "name": mcp_tool.name,
                        "description": mcp_tool.description,
                        "parameters": schema
                    }));
                }
            }
        }

        let tools_schema_str = serde_json::to_string_pretty(&tools_json).unwrap();

        // 2. Semantic Memory (RAG)
        let mut rag_context = String::new();
        if let Some(last_msg) = history.last() {
            if last_msg.role == "user" {
                let query = last_msg.content.to_string();
                let mut any = false;
                if let Ok(entries) = self.db.search_knowledge_entries_fts(&query, 3) {
                    if !entries.is_empty() {
                        any = true;
                        rag_context.push_str("\n\n--- RELEVANT KNOWLEDGE (RAG) ---\n");
                        for entry in entries {
                            rag_context.push_str(&format!("Title: {}\nContent: {}\n\n", entry.title, entry.content));
                        }
                    }
                }
                if !any {
                    if let Ok(items) = self.db.search_knowledge_fts(&query, 3) {
                        if !items.is_empty() {
                            rag_context.push_str("\n\n--- RELEVANT KNOWLEDGE (RAG) ---\n");
                            for item in items {
                                rag_context.push_str(&format!("Title: {}\nContent: {}\n\n", item.title, item.content));
                            }
                        }
                    }
                }
                if !rag_context.is_empty() {
                    rag_context.push_str("----------------------------------\n");
                }
            }
        }

        let injection = format!(
            "\n\n[SYSTEM INJECTION]\nYou have access to the following tools:\n{}\n\nTo use a tool, you MUST return ONLY a JSON object wrapped in `<tool_call>` tags like this:\n<tool_call>{{\"id\":\"call_1\",\"name\":\"tool_name\",\"arguments\":{{\"key\":\"value\"}}}}</tool_call>\nDo not output any other text when making a tool call.{}",
            tools_schema_str, rag_context
        );

        if let Some(sys_msg) = history.first_mut().filter(|m| m.role == "system") {
            sys_msg.content = format!("{}{}", sys_msg.content, injection).into();
        } else {
            history.insert(0, ChatMessage {
                role: "system".into(),
                content: injection.into(),
                agent_name: None,

            });
        }


        while iteration < max_iterations {
            let mut response_text = String::new();
            let mut tool_calls = Vec::<ToolCall>::new();

            let mut stream = self.provider.send_message_stream(history.clone()).await?;
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(crate::core::models::chat::StreamChunk::Text(t)) => response_text.push_str(&t),
                    Ok(crate::core::models::chat::StreamChunk::Done(_)) => break,
                    Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
                }
            }

            let mut search_idx = 0usize;
            while let Some(start) = response_text[search_idx..].find("<tool_call>") {
                let start = search_idx + start;
                let after_start = start + "<tool_call>".len();
                if let Some(end_rel) = response_text[after_start..].find("</tool_call>") {
                    let end = after_start + end_rel;
                    let tool_json = response_text[after_start..end].trim();
                    if let Ok(mut tc) = serde_json::from_str::<ToolCall>(tool_json) {
                        if tc.id.is_empty() {
                            tc.id = format!("call_{}", uuid::Uuid::new_v4());
                        }
                        if tc.arguments.is_string() {
                            if let Some(s) = tc.arguments.as_str() {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(s) {
                                    tc.arguments = v;
                                }
                            }
                        }
                        tool_calls.push(tc);
                    }
                    search_idx = end + "</tool_call>".len();
                    continue;
                }
                break;
            }

            if tool_calls.is_empty() {
                return Ok(response_text);
            }

            // Execute tools
            history.push(ChatMessage {
                role: "assistant".into(),
                content: response_text.into(),
                agent_name: None,
            });

            for tc in tool_calls.clone() {
                let result = self.execute_tool(&tc.name, &tc.arguments).await;
                history.push(ChatMessage {
                    role: "user".into(),
                    content: format!("Tool result (id={}, name={}):\n{}", tc.id, tc.name, result).into(),
                    agent_name: Some(tc.name.clone().into()),
                });
            }

            iteration += 1;
        }
        Ok("Max tool iterations reached".to_string())
    }

    fn prune_history(&self, history: &[ChatMessage], max_messages: usize) -> Vec<ChatMessage> {
        let mut pruned = Vec::new();
        // Keep System Prompt (Index 0)
        if let Some(sys) = history.first().filter(|m| m.role == "system") {
            pruned.push(sys.clone());
        }
        
        let tail_start = history.len().saturating_sub(max_messages);
        let start_idx = std::cmp::max(1, tail_start); // Skip index 0 as it's system prompt
        
        if start_idx < history.len() {
            pruned.extend_from_slice(&history[start_idx..]);
        }
        pruned
    }

    async fn execute_tool(&self, name: &str, args: &serde_json::Value) -> String {
        if name == "save_to_knowledge" {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let entry = crate::core::models::knowledge::KnowledgeEntry {
                id: uuid::Uuid::new_v4().to_string(),
                agent_id: self.agent_id.clone(),
                session_id: Some(self.team_instance_id.clone()),
                title,
                content,
                tags: vec![],
                created_at: chrono::Utc::now(),
            };
            if let Err(e) = self.db.upsert_knowledge_entry(&entry) {
                return format!("Failed to save knowledge: {}", e);
            }
            return "Knowledge saved successfully.".to_string();
        }
        
        if name == "declare_consensus" {
            let msg_text = args.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let msg = crate::infrastructure::message_bus::routing::TeamMessage::new_broadcast(
                self.team_instance_id.clone(),
                self.agent_id.clone(),
                format!("[CONSENSUS_REACHED] {}", msg_text)
            );
            let _ = self.team_bus.route_message(msg).await;
            return "Consensus declared and broadcasted.".to_string();
        }

        if name == "handoff_to_team" {
            let target_team = args.get("target_team").and_then(|v| v.as_str()).unwrap_or("UnknownTeam");
            let package = args.get("briefing_package").and_then(|v| v.as_str()).unwrap_or("");
            let handoff_type = args.get("handoff_type").and_then(|v| v.as_str()).unwrap_or("handoff");
            let mut correlation_id = args.get("correlation_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let reply_to_team = args.get("reply_to_team").and_then(|v| v.as_str()).unwrap_or("");
            if correlation_id.is_empty() {
                correlation_id = uuid::Uuid::new_v4().to_string();
            }

            let payload = serde_json::json!({
                "handoff_type": handoff_type,
                "correlation_id": correlation_id,
                "from_team": self.team_instance_id.clone(),
                "reply_to_team": reply_to_team,
                "briefing_package": package
            });
            let payload_str = payload.to_string();

            let mut msg = crate::infrastructure::message_bus::routing::TeamMessage::new_broadcast(
                target_team.to_string(),
                self.agent_id.clone(),
                format!("[CROSS_TEAM_HANDOFF] {}", payload_str.clone()),
            );
            msg.metadata = Some(payload_str);
            let _ = self.db.insert_team_message(&msg);
            let _ = self.team_bus.route_message(msg).await;
            return format!("Handoff package sent to {}.", target_team);
        }

        if name == "create_subtasks" {
            if let Some(tasks) = args.get("tasks").and_then(|v| v.as_array()) {
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
                let name_map = self
                    .db
                    .get_instance_agent_name_mapping(&self.team_instance_id)
                    .unwrap_or_default();
                for t in tasks {
                    let desc = t.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    let role = t.get("role").and_then(|v| v.as_str()).unwrap_or("");
                    let assignee_id = name_map.get(role).map(|s| s.as_str());
                    let task_id = format!("{}:{}", self.team_instance_id, uuid::Uuid::new_v4());
                    let payload = serde_json::json!({
                        "type": "subtask",
                        "description": desc,
                        "role": role
                    })
                    .to_string();
                    if !team_id.is_empty() {
                        let _ = self.db.upsert_task(
                            &task_id,
                            &team_id,
                            Some(&self.team_instance_id),
                            assignee_id,
                            "pending",
                            "medium",
                            Some(&payload),
                        );
                    }
                    let msg = crate::infrastructure::message_bus::routing::TeamMessage::new_role_group(
                        self.team_instance_id.clone(),
                        self.agent_id.clone(),
                        role.to_string(),
                        format!("[NEW_TASK] {}", desc)
                    );
                    let _ = self.team_bus.route_message(msg).await;
                }
                return format!("Created {} subtasks and dispatched.", tasks.len());
            }
        }
        

        if name == "web_search" {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
            
            let client = reqwest::Client::new();
            match client.get(&url).header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)").send().await {
                Ok(resp) => {
                    if let Ok(text) = resp.text().await {
                        let mut in_tag = false;
                        let mut stripped = String::new();
                        for c in text.chars() {
                            if c == '<' { in_tag = true; continue; }
                            if c == '>' { in_tag = false; stripped.push(' '); continue; }
                            if !in_tag { stripped.push(c); }
                        }
                        let truncated: String = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
                        let limit = std::cmp::min(3000, truncated.len());
                        return format!("Search results:\n{}", &truncated[..limit]);
                    }
                }
                Err(e) => {
                    return format!("Web search failed: {}", e);
                }
            }
        }

        if name == "run_cli" {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let cmd_args: Vec<String> = args
                .get("args")
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                .collect();

            let output = std::process::Command::new(cmd)
                .args(cmd_args)
                .output();

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let mut result = String::new();
                    if !stdout.is_empty() { result.push_str(&format!("STDOUT:\n{}\n", stdout)); }
                    if !stderr.is_empty() { result.push_str(&format!("STDERR:\n{}\n", stderr)); }
                    return if result.is_empty() { "Command executed with no output.".to_string() } else { result };
                }
                Err(e) => {
                    return format!("Failed to execute command: {}", e);
                }
            }
        }

        if let Some(mcp_tool) = self.mcp_registry.list_tools().into_iter().find(|t| t.name == name) {
            let mut process_args = mcp_tool.args.clone();
            process_args.push(args.to_string());
            
            let output = std::process::Command::new(&mcp_tool.command)
                .args(process_args)
                .output();

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let mut result = String::new();
                    if !stdout.is_empty() { result.push_str(&format!("STDOUT:\n{}\n", stdout)); }
                    if !stderr.is_empty() { result.push_str(&format!("STDERR:\n{}\n", stderr)); }
                    return if result.is_empty() { "MCP Tool executed with no output.".to_string() } else { result };
                }
                Err(e) => {
                    return format!("Failed to execute MCP tool: {}", e);
                }
            }
        }

        format!("Tool {} executed successfully with args: {}", name, args)
    }
}
