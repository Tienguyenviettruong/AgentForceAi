use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
    cancel_flag: Option<Arc<AtomicBool>>,
    stream_callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl AgentExecutor {
    pub fn new(
        provider: Arc<dyn BaseProviderAdapter>,
        mcp_registry: Arc<McpToolRegistry>,
        db: Arc<dyn DatabasePort>,
        team_bus: Arc<TeamBusRouter>,
        team_instance_id: String,
        agent_id: String,
        cancel_flag: Option<Arc<AtomicBool>>,
        stream_callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Self {
        Self { provider, mcp_registry, db, team_bus, team_instance_id, agent_id, cancel_flag, stream_callback }
    }

    pub async fn execute_task(&self, mut history: Vec<ChatMessage>) -> Result<String> {
        if self.cancel_flag.as_ref().is_some_and(|f| f.load(Ordering::SeqCst)) {
            return Ok("Cancelled.".to_string());
        }
        let mut iteration = 0;
        let max_iterations = 5;

        // Apply Smart Context Pruning (summarize evicted messages)
        history = self.smart_prune_history(&history, 20).await;

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
                    "description": "Run a shell command on the host machine. Commands are sandboxed to the workspace directory.",
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
                },
                {
                    "name": "write_file",
                    "description": "Write content to a file in the workspace. Creates parent directories if needed. Use for creating new files.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Relative or absolute file path" },
                            "content": { "type": "string", "description": "Full file content to write" }
                        },
                        "required": ["path", "content"]
                    }
                },
                {
                    "name": "read_file",
                    "description": "Read the content of an existing file in the workspace.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Relative or absolute file path" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "edit_file",
                    "description": "Edit an existing file by finding and replacing text. Use this to modify specific parts of existing files.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Relative or absolute file path" },
                            "find": { "type": "string", "description": "Exact text to find in the file" },
                            "replace": { "type": "string", "description": "Text to replace it with" }
                        },
                        "required": ["path", "find", "replace"]
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

        // 2. Semantic Memory (RAG) — hybrid FTS + vector search
        let mut rag_context = String::new();
        if let Some(last_msg) = history.last() {
            if last_msg.role == "user" {
                let query = last_msg.content.to_string();
                let mut any = false;
                // 2a. FTS on knowledge_entries (agent-saved knowledge)
                if let Ok(entries) = self.db.search_knowledge_entries_fts(&query, 3) {
                    if !entries.is_empty() {
                        any = true;
                        rag_context.push_str("\n\n--- RELEVANT KNOWLEDGE (RAG) ---\n");
                        for entry in entries {
                            rag_context.push_str(&format!("Title: {}\nContent: {}\n\n", entry.title, entry.content));
                        }
                    }
                }
                // 2b. FTS on knowledge table (Obsidian/document vault)
                if !any {
                    if let Ok(items) = self.db.search_knowledge_fts(&query, 3) {
                        if !items.is_empty() {
                            any = true;
                            rag_context.push_str("\n\n--- RELEVANT KNOWLEDGE (RAG) ---\n");
                            for item in items {
                                rag_context.push_str(&format!("Title: {}\nContent: {}\n\n", item.title, item.content));
                            }
                        }
                    }
                }
                // 2c. Vector/semantic search on knowledge_chunks (embedding-based)
                if let Ok(query_vec) = crate::providers::embeddings::EmbeddingProvider::new()
                    .get_embedding(&query)
                    .await
                {
                    if let Ok(similar) = self.db.search_similar_chunks(&query_vec, 3) {
                        if !similar.is_empty() {
                            if !any {
                                rag_context.push_str("\n\n--- RELEVANT KNOWLEDGE (RAG) ---\n");
                            }
                            rag_context.push_str("\n--- Semantic matches ---\n");
                            for (title, chunk_content, sim) in &similar {
                                if *sim > 0.5 {
                                    rag_context.push_str(&format!("Document: {} (sim: {:.2})\n{}\n\n", title, sim, chunk_content));
                                }
                            }
                        }
                    }
                }
                if !rag_context.is_empty() {
                    rag_context.push_str("----------------------------------\n");
                }
            }
        }

        // 3. Skills injection — inject available skill names for agent awareness
        let mut skills_context = String::new();
        {
            let registry = crate::application::skills::initialize_skills().await;
            let skills = registry.discover_skills().await;
            if !skills.is_empty() {
                skills_context.push_str("\n\n--- AVAILABLE SKILLS ---\n");
                for skill in &skills {
                    skills_context.push_str(&format!("- {} ({}): {}\n", skill.name, skill.id, skill.description));
                }
                skills_context.push_str("---\n");
            }
        }

        let injection = format!(
            "\n\n[SYSTEM INJECTION]\nYou have access to the following tools:\n{}\n\nTo use a tool, you MUST return ONLY a JSON object wrapped in `<tool_call>` tags like this:\n<tool_call>{{\"id\":\"call_1\",\"name\":\"tool_name\",\"arguments\":{{\"key\":\"value\"}}}}</tool_call>\nDo not output any other text when making a tool call.{}{}",
            tools_schema_str, rag_context, skills_context
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
            if self.cancel_flag.as_ref().is_some_and(|f| f.load(Ordering::SeqCst)) {
                return Ok("Cancelled.".to_string());
            }
            let mut response_text = String::new();
            let mut tool_calls = Vec::<ToolCall>::new();
            let mut token_usage = crate::core::models::TokenUsage::default();
            let mut stream = self.provider.send_message_stream(history.clone()).await?;
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                if self.cancel_flag.as_ref().is_some_and(|f| f.load(Ordering::SeqCst)) {
                    return Ok("Cancelled.".to_string());
                }
                match chunk {
                    Ok(crate::core::models::chat::StreamChunk::Text(t)) => {
                        response_text.push_str(&t);
                        if let Some(cb) = &self.stream_callback {
                            cb(Self::sanitize_for_display(&response_text));
                        }
                    }
                    Ok(crate::core::models::chat::StreamChunk::Done(u)) => {
                        token_usage = u;
                        break;
                    }
                    Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
                }
            }
            let _ = self.db.insert_token_usage(
                Some(&self.team_instance_id),
                &self.agent_id,
                token_usage.input_tokens,
                token_usage.output_tokens,
                token_usage.total_tokens,
            );

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
                return Ok(Self::sanitize_for_display(&response_text));
            }

            // Execute tools
            history.push(ChatMessage {
                role: "assistant".into(),
                content: response_text.into(),
                agent_name: None,
            });

            for tc in tool_calls.clone() {
                let result = self.execute_tool(&tc.name, &tc.arguments).await;

                // Audit log: record every tool invocation
                let audit_event = crate::infrastructure::security::audit::AuditEvent {
                    timestamp: chrono::Utc::now(),
                    action: format!("tool_call:{}", tc.name),
                    user_id: Some(self.agent_id.clone()),
                    resource: self.team_instance_id.clone(),
                    details: format!("Tool: {}, Args: {}, Result (truncated): {}", tc.name, tc.arguments, &result[..std::cmp::min(200, result.len())]),
                };
                let _ = self.db.insert_audit_log(&audit_event);

                history.push(ChatMessage {
                    role: "user".into(),
                    content: format!("Tool result (id={}, name={}):\n{}", tc.id, tc.name, result).into(),
                    agent_name: Some(tc.name.clone().into()),
                });
            }

            iteration += 1;
        }
        // 4. Auto-summarize session and save to knowledge for long-term memory
        self.auto_summarize_session(&history).await;

        Ok("Max tool iterations reached".to_string())
    }

    fn sanitize_for_display(raw: &str) -> String {
        let mut out = String::new();
        let mut i = 0usize;
        loop {
            let Some(start_rel) = raw[i..].find("<tool_call") else {
                out.push_str(&raw[i..]);
                break;
            };
            let start = i + start_rel;
            out.push_str(&raw[i..start]);
            let after = start + "<tool_call>".len();
            if raw[start..].starts_with("<tool_call>") {
                if let Some(end_rel) = raw[after..].find("</tool_call>") {
                    i = after + end_rel + "</tool_call>".len();
                    continue;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        out
    }

    /// Smart context pruning: keeps system prompt, summarizes evicted messages, keeps recent ones.
    async fn smart_prune_history(&self, history: &[ChatMessage], max_messages: usize) -> Vec<ChatMessage> {
        let mut pruned = Vec::new();
        // Keep System Prompt (Index 0)
        if let Some(sys) = history.first().filter(|m| m.role == "system") {
            pruned.push(sys.clone());
        }
        
        let tail_start = history.len().saturating_sub(max_messages);
        let start_idx = std::cmp::max(1, tail_start);
        
        // If there are evicted messages, create a summary
        if start_idx > 1 {
            let evicted = &history[1..start_idx];
            let mut summary_parts = Vec::new();
            for msg in evicted {
                let role = &msg.role;
                let content_preview: String = msg.content.chars().take(200).collect();
                summary_parts.push(format!("{}: {}", role, content_preview));
            }
            let summary_text = format!(
                "[CONTEXT SUMMARY — {} earlier messages condensed]\n{}",
                evicted.len(),
                summary_parts.join("\n")
            );
            // Truncate summary to fit token budget (~2000 chars)
            let truncated: String = summary_text.chars().take(2000).collect();
            pruned.push(ChatMessage {
                role: "system".into(),
                content: truncated.into(),
                agent_name: None,
            });
        }

        if start_idx < history.len() {
            pruned.extend_from_slice(&history[start_idx..]);
        }
        pruned
    }

    /// Auto-summarize the session and save key facts to knowledge base for long-term memory.
    async fn auto_summarize_session(&self, history: &[ChatMessage]) {
        // Only summarize if there are enough messages
        if history.len() < 4 {
            return;
        }
        // Extract key user messages and assistant responses
        let mut key_points = Vec::new();
        for msg in history.iter().rev().take(6) {
            if msg.role == "user" || msg.role == "assistant" {
                let preview: String = msg.content.chars().take(300).collect();
                key_points.push(format!("{}: {}", msg.role, preview));
            }
        }
        if key_points.is_empty() {
            return;
        }
        key_points.reverse();

        let summary = format!(
            "Session summary for instance {}:\n{}",
            self.team_instance_id,
            key_points.join("\n")
        );

        let entry = crate::core::models::knowledge::KnowledgeEntry {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id: self.agent_id.clone(),
            session_id: Some(self.team_instance_id.clone()),
            title: format!("Session Summary {}", chrono::Utc::now().format("%Y-%m-%d %H:%M")),
            content: summary,
            tags: vec!["auto_summary".to_string()],
            created_at: chrono::Utc::now(),
        };
        let _ = self.db.upsert_knowledge_entry(&entry);
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
                let mut workflow_node_ids = Vec::new();
                let mut workflow_nodes = Vec::<(String, String, String, String)>::new();
                for t in tasks {
                    let desc = t.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    let role = t.get("role").and_then(|v| v.as_str()).unwrap_or("");
                    let assignee_id = name_map.get(role).map(|s| s.as_str());
                    let dag_id = uuid::Uuid::new_v4().to_string();
                    let task_id = format!("{}:{}", self.team_instance_id, dag_id);
                    let payload = serde_json::to_string(&crate::application::orchestration::core::DagTask {
                        id: dag_id.clone(),
                        name: role.to_string(),
                        description: desc.to_string(),
                        dependencies: Vec::new(),
                        priority: 2,
                        deadline: None,
                        assignee_id: assignee_id.map(|s| s.to_string()),
                    })
                    .unwrap_or_default();
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
                    workflow_node_ids.push(dag_id.clone());
                    workflow_nodes.push((
                        dag_id,
                        role.to_string(),
                        desc.to_string(),
                        assignee_id.unwrap_or("auto").to_string(),
                    ));
                }
                if !workflow_node_ids.is_empty() {
                    let mut nodes = std::collections::HashMap::new();
                    nodes.insert(
                        "start".to_string(),
                        crate::application::iflow_engine::nodes::Node {
                            id: "start".to_string(),
                            name: "Start".to_string(),
                            node_type: crate::application::iflow_engine::nodes::NodeType::Start,
                            next_nodes: workflow_node_ids.clone(),
                        },
                    );
                    for (id, role, desc, agent_id) in workflow_nodes {
                        nodes.insert(
                            id.clone(),
                            crate::application::iflow_engine::nodes::Node {
                                id: id.clone(),
                                name: role,
                                node_type: crate::application::iflow_engine::nodes::NodeType::AgentTask {
                                    agent_id,
                                    instruction: desc,
                                    input_vars: Vec::new(),
                                    output_var: None,
                                },
                                next_nodes: vec!["end".to_string()],
                            },
                        );
                    }
                    nodes.insert(
                        "end".to_string(),
                        crate::application::iflow_engine::nodes::Node {
                            id: "end".to_string(),
                            name: "End".to_string(),
                            node_type: crate::application::iflow_engine::nodes::NodeType::End,
                            next_nodes: Vec::new(),
                        },
                    );

                    let workflow_id = uuid::Uuid::new_v4().to_string();
                    let workflow = crate::application::iflow_engine::engine::Workflow {
                        id: workflow_id.clone(),
                        name: format!("Chat Flow {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")),
                        version: "1.0".to_string(),
                        nodes,
                        start_node_id: "start".to_string(),
                        team_id: if team_id.is_empty() { None } else { Some(team_id.clone()) },
                        instance_id: Some(self.team_instance_id.clone()),
                    };
                    let record = crate::core::models::workflow::WorkflowRecord {
                        id: workflow.id.clone(),
                        name: workflow.name.clone(),
                        definition: serde_json::to_string(&workflow).unwrap_or_default(),
                        version: workflow.version.clone(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                    };
                    let _ = self.db.upsert_workflow(&record);
                    return format!(
                        "Created {} subtasks and generated workflow {}.",
                        tasks.len(),
                        workflow_id
                    );
                }
                return format!("Created {} subtasks and dispatched.", tasks.len());
            }
        }
        

        if name == "web_search" {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
            
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            match client.get(&url).header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36").send().await {
                Ok(resp) => {
                    if let Ok(text) = resp.text().await {
                        // Extract structured results from DuckDuckGo HTML
                        let mut results = Vec::new();
                        let mut result_num = 0;

                        // Extract result snippets between result__snippet class markers
                        for segment in text.split("result__snippet") {
                            if result_num > 0 && result_num <= 8 {
                                // Strip HTML tags
                                let mut in_tag = false;
                                let mut clean = String::new();
                                for c in segment.chars().take(500) {
                                    if c == '<' { in_tag = true; continue; }
                                    if c == '>' { in_tag = false; continue; }
                                    if !in_tag { clean.push(c); }
                                }
                                let clean = clean.trim().to_string();
                                if !clean.is_empty() && clean.len() > 20 {
                                    results.push(format!("{}. {}", result_num, clean));
                                }
                            }
                            result_num += 1;
                        }

                        // Fallback: if structured extraction failed, use raw strip
                        if results.is_empty() {
                            let mut in_tag = false;
                            let mut stripped = String::new();
                            for c in text.chars() {
                                if c == '<' { in_tag = true; continue; }
                                if c == '>' { in_tag = false; stripped.push(' '); continue; }
                                if !in_tag { stripped.push(c); }
                            }
                            let truncated: String = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
                            let limit = std::cmp::min(4000, truncated.len());
                            return format!("Search results for '{}':\n{}", query, &truncated[..limit]);
                        }

                        return format!("Search results for '{}':\n{}", query, results.join("\n\n"));
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

            // Sandbox: block dangerous commands
            let blocked = ["rm -rf /", "format", "del /s /q", "mkfs", "dd if=", "shutdown", "reboot", ":(){ :|:", ">\\.\\physicaldrive"];
            let full_cmd = format!("{} {}", cmd, cmd_args.join(" "));
            for b in &blocked {
                if full_cmd.to_lowercase().contains(b) {
                    return format!("Command blocked by security policy: contains '{}'", b);
                }
            }

            // Resolve workspace directory for working dir
            let workspace_dir = self.db
                .get_setting(&format!("workspace_{}", self.team_instance_id))
                .ok()
                .flatten();

            let mut command = std::process::Command::new(cmd);
            command.args(&cmd_args);
            if let Some(ref ws) = workspace_dir {
                command.current_dir(ws);
            }

            // Timeout: spawn in a thread with a 60-second timeout
            let output = tokio::task::spawn_blocking(move || {
                command.output()
            });

            match tokio::time::timeout(std::time::Duration::from_secs(60), output).await {
                Ok(Ok(Ok(out))) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let mut result = String::new();
                    if !stdout.is_empty() {
                        let truncated: String = stdout.chars().take(8000).collect();
                        result.push_str(&format!("STDOUT:\n{}\n", truncated));
                    }
                    if !stderr.is_empty() {
                        let truncated: String = stderr.chars().take(4000).collect();
                        result.push_str(&format!("STDERR:\n{}\n", truncated));
                    }
                    return if result.is_empty() { "Command executed with no output.".to_string() } else { result };
                }
                Ok(Ok(Err(e))) => {
                    return format!("Failed to execute command: {}", e);
                }
                Ok(Err(e)) => {
                    return format!("Command thread panicked: {}", e);
                }
                Err(_) => {
                    return "Command timed out after 60 seconds.".to_string();
                }
            }
        }

        // --- File manipulation tools ---
        if name == "write_file" {
            let file_path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if file_path.is_empty() {
                return "Error: path is required.".to_string();
            }
            let workspace_dir = self.db.get_setting(&format!("workspace_{}", self.team_instance_id)).ok().flatten();
            let path = std::path::Path::new(file_path);
            let resolved = if path.is_relative() {
                if let Some(ref ws) = workspace_dir {
                    std::path::PathBuf::from(ws).join(path)
                } else {
                    path.to_path_buf()
                }
            } else {
                path.to_path_buf()
            };
            if let Some(parent) = resolved.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&resolved, content) {
                Ok(_) => return format!("File written successfully: {}", resolved.display()),
                Err(e) => return format!("Failed to write file: {}", e),
            }
        }

        if name == "read_file" {
            let file_path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if file_path.is_empty() {
                return "Error: path is required.".to_string();
            }
            let workspace_dir = self.db.get_setting(&format!("workspace_{}", self.team_instance_id)).ok().flatten();
            let path = std::path::Path::new(file_path);
            let resolved = if path.is_relative() {
                if let Some(ref ws) = workspace_dir {
                    std::path::PathBuf::from(ws).join(path)
                } else {
                    path.to_path_buf()
                }
            } else {
                path.to_path_buf()
            };
            match std::fs::read_to_string(&resolved) {
                Ok(content) => {
                    let truncated: String = content.chars().take(12000).collect();
                    return format!("File content of {}:\n{}", resolved.display(), truncated);
                }
                Err(e) => return format!("Failed to read file: {}", e),
            }
        }

        if name == "edit_file" {
            let file_path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let find_text = args.get("find").and_then(|v| v.as_str()).unwrap_or("");
            let replace_text = args.get("replace").and_then(|v| v.as_str()).unwrap_or("");
            if file_path.is_empty() || find_text.is_empty() {
                return "Error: path and find are required.".to_string();
            }
            let workspace_dir = self.db.get_setting(&format!("workspace_{}", self.team_instance_id)).ok().flatten();
            let path = std::path::Path::new(file_path);
            let resolved = if path.is_relative() {
                if let Some(ref ws) = workspace_dir {
                    std::path::PathBuf::from(ws).join(path)
                } else {
                    path.to_path_buf()
                }
            } else {
                path.to_path_buf()
            };
            match std::fs::read_to_string(&resolved) {
                Ok(content) => {
                    if !content.contains(find_text) {
                        return format!("Error: text to find not found in {}", resolved.display());
                    }
                    let new_content = content.replacen(find_text, replace_text, 1);
                    match std::fs::write(&resolved, &new_content) {
                        Ok(_) => return format!("File edited successfully: {}", resolved.display()),
                        Err(e) => return format!("Failed to write edited file: {}", e),
                    }
                }
                Err(e) => return format!("Failed to read file for editing: {}", e),
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
