import re

filepath = "/workspace/agentforge-ui/src/application/orchestration/executor.rs"
with open(filepath, 'r') as f:
    content = f.read()

injection_logic = r"""
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
                            "briefing_package": { "type": "string" }
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
                if let Ok(entries) = self.db.search_knowledge_entries_fts(&query, 3) {
                    if !entries.is_empty() {
                        rag_context.push_str("\n\n--- RELEVANT KNOWLEDGE (RAG) ---\n");
                        for entry in entries {
                            rag_context.push_str(&format!("Title: {}\nContent: {}\n\n", entry.title, entry.content));
                        }
                        rag_context.push_str("----------------------------------\n");
                    }
                }
            }
        }

        let injection = format!(
            "\n\n[SYSTEM INJECTION]\nYou have access to the following tools:\n{}\n\nTo use a tool, you MUST return ONLY a JSON object wrapped in `<tool_call>` tags like this:\n<tool_call>{{\"name\": \"tool_name\", \"arguments\": \"{{\\\"key\\\": \\\"value\\\"}}\"}}</tool_call>\nDo not output any other text when making a tool call.{}",
            tools_schema_str, rag_context
        );

        if let Some(sys_msg) = history.first_mut().filter(|m| m.role == "system") {
            sys_msg.content = format!("{}{}", sys_msg.content, injection).into();
        } else {
            history.insert(0, ChatMessage {
                role: "system".into(),
                content: injection.into(),
                agent_name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
"""

content = content.replace("        history = self.prune_history(&history, 20);", "        history = self.prune_history(&history, 20);\n" + injection_logic)

with open(filepath, 'w') as f:
    f.write(content)
