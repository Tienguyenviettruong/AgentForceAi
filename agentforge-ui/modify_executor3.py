import re

filepath = "/workspace/agentforge-ui/src/application/orchestration/executor.rs"
with open(filepath, 'r') as f:
    content = f.read()

# Let's insert the implementation of `web_search`, `run_cli` and MCP tools execution.
# Inside `execute_tool`:

tools_impl = r"""
        if name == "web_search" {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(args) {
                let query = v["query"].as_str().unwrap_or("");
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
        }

        if name == "run_cli" {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(args) {
                let cmd = v["command"].as_str().unwrap_or("");
                let cmd_args: Vec<String> = v["args"]
                    .as_array()
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
"""

content = content.replace("        format!(\"Tool {} executed successfully with args: {}\", name, args)", tools_impl + "\n        format!(\"Tool {} executed successfully with args: {}\", name, args)")

with open(filepath, 'w') as f:
    f.write(content)

