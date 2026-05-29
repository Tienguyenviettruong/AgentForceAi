use super::registry::{McpServerRecord, McpTool};
use crate::infrastructure::security::keychain::{Keychain, SECURE_SECRET_SERVICE};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

pub struct McpServer;

impl McpServer {
    fn stdio_sandbox_wrapper() -> Option<String> {
        std::env::var("AGENTFORGE_MCP_SANDBOX_WRAPPER")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn stdio_command_allowed(command: &str) -> bool {
        let executable = std::path::Path::new(command)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(command)
            .to_ascii_lowercase();
        let configured = std::env::var("AGENTFORGE_MCP_STDIO_ALLOWLIST")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(|item| item.trim().to_ascii_lowercase())
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| {
                vec![
                    "npx".to_string(),
                    "npx.cmd".to_string(),
                    "uvx".to_string(),
                    "node".to_string(),
                    "node.exe".to_string(),
                    "python".to_string(),
                    "python.exe".to_string(),
                ]
            });
        configured.iter().any(|allowed| allowed == &executable)
    }

    pub fn stdio_runtime_status() -> &'static str {
        if Self::stdio_sandbox_wrapper().is_some() {
            "sandbox_ready"
        } else if std::env::var("AGENTFORGE_ALLOW_UNSANDBOXED_MCP_STDIO")
            .ok()
            .as_deref()
            == Some("true")
        {
            "unsandboxed_override"
        } else {
            "blocked_until_isolated"
        }
    }

    pub async fn discover_tools(server: &McpServerRecord) -> Result<Vec<McpTool>> {
        let result = Self::rpc_call(server, "tools/list", json!({})).await?;
        let tools = result
            .get("tools")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("MCP tools/list response does not contain a tools array."))?;
        let server_prefix = server
            .name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '_' {
                    character
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .to_ascii_lowercase();
        Ok(tools
            .iter()
            .filter_map(|tool| {
                let protocol_name = tool.get("name")?.as_str()?.to_string();
                let exposed_name = format!("{}__{}", server_prefix, protocol_name);
                Some(McpTool {
                    id: format!("{}::{}", server.id, protocol_name),
                    server_id: Some(server.id.clone()),
                    name: exposed_name,
                    description: tool
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or("MCP server tool")
                        .to_string(),
                    version: MCP_PROTOCOL_VERSION.to_string(),
                    command: String::new(),
                    args: vec![protocol_name],
                    input_schema: tool
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or_else(|| json!({"type": "object"}))
                        .to_string(),
                    is_active: true,
                })
            })
            .collect())
    }

    pub async fn invoke_tool(
        server: &McpServerRecord,
        tool: &McpTool,
        payload: Value,
    ) -> Result<Value> {
        if tool.server_id.as_deref() != Some(server.id.as_str()) {
            return Err(anyhow!("MCP tool is not owned by the configured server."));
        }
        let protocol_name = tool
            .args
            .first()
            .map(String::as_str)
            .unwrap_or(tool.name.as_str());
        Self::rpc_call(
            server,
            "tools/call",
            json!({"name": protocol_name, "arguments": payload}),
        )
        .await
    }

    async fn rpc_call(server: &McpServerRecord, method: &str, params: Value) -> Result<Value> {
        if !server.is_enabled {
            return Err(anyhow!("MCP server is disabled."));
        }
        match server.transport.as_str() {
            "stdio" => {
                let has_sandbox = Self::stdio_sandbox_wrapper().is_some();
                let has_override = std::env::var("AGENTFORGE_ALLOW_UNSANDBOXED_MCP_STDIO")
                    .ok()
                    .as_deref()
                    == Some("true");
                if !has_sandbox && !has_override {
                    return Err(anyhow!(
                        "External MCP stdio is disabled because no OS sandbox wrapper is configured. Set AGENTFORGE_MCP_SANDBOX_WRAPPER to an isolated launcher, or use remote HTTPS MCP."
                    ));
                }
                Self::stdio_call(server, method, params).await
            }
            "http" | "remote" | "streamable_http" => {
                Self::remote_call(server, method, params).await
            }
            other => Err(anyhow!("Unsupported external MCP transport '{}'.", other)),
        }
    }

    async fn resolved_environment(server: &McpServerRecord) -> Result<HashMap<String, String>> {
        let raw: HashMap<String, String> =
            serde_json::from_str(&server.env_secret_refs).unwrap_or_default();
        let keychain = Keychain::new().await?;
        let mut resolved = HashMap::new();
        for (name, reference) in raw {
            let account = reference.strip_prefix("secret://").ok_or_else(|| {
                anyhow!(
                    "MCP environment entry '{}' is not an opaque secret reference.",
                    name
                )
            })?;
            let secret = keychain
                .get_secret(SECURE_SECRET_SERVICE, account)
                .await?
                .ok_or_else(|| {
                    anyhow!("MCP secret '{}' was not found in the OS store.", account)
                })?;
            resolved.insert(name, secret);
        }
        Ok(resolved)
    }

    async fn resolved_headers(server: &McpServerRecord) -> Result<HashMap<String, String>> {
        let raw: HashMap<String, String> =
            serde_json::from_str(&server.header_secret_refs).unwrap_or_default();
        let keychain = Keychain::new().await?;
        let mut resolved = HashMap::new();
        for (name, value) in raw {
            if let Some(account) = value.strip_prefix("secret://") {
                let secret = keychain
                    .get_secret(SECURE_SECRET_SERVICE, account)
                    .await?
                    .ok_or_else(|| {
                        anyhow!("MCP secret '{}' was not found in the OS store.", account)
                    })?;
                resolved.insert(name, secret);
            } else {
                resolved.insert(name, value);
            }
        }
        Ok(resolved)
    }

    async fn stdio_call(server: &McpServerRecord, method: &str, params: Value) -> Result<Value> {
        let command = server
            .command
            .as_deref()
            .filter(|command| !command.trim().is_empty())
            .ok_or_else(|| anyhow!("Stdio MCP server has no command."))?;
        if !Self::stdio_command_allowed(command) {
            return Err(anyhow!(
                "Stdio MCP command '{}' is not in AGENTFORGE_MCP_STDIO_ALLOWLIST.",
                command
            ));
        }
        let environment = Self::resolved_environment(server).await?;
        let mut command_builder = if let Some(wrapper) = Self::stdio_sandbox_wrapper() {
            let mut builder = Command::new(wrapper);
            builder.arg("--").arg(command).args(&server.args);
            builder
        } else {
            let mut builder = Command::new(command);
            builder.args(&server.args);
            builder
        };
        let mut child = command_builder
            .env_clear()
            .envs(environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| anyhow!("Unable to start MCP stdio server: {}", error))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("MCP stdio stdin is unavailable."))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("MCP stdio stdout is unavailable."))?;
        let mut stdout = BufReader::new(stdout).lines();

        Self::stdio_request(
            &mut stdin,
            &mut stdout,
            1,
            "initialize",
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "agentforge-ui", "version": "0.1.0"}
            }),
        )
        .await?;
        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        stdin
            .write_all(format!("{}\n", initialized).as_bytes())
            .await?;
        stdin.flush().await?;
        let result = Self::stdio_request(&mut stdin, &mut stdout, 2, method, params).await;
        let _ = child.kill().await;
        result
    }

    async fn stdio_request(
        stdin: &mut tokio::process::ChildStdin,
        stdout: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
        id: i64,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        stdin.write_all(format!("{}\n", request).as_bytes()).await?;
        stdin.flush().await?;
        loop {
            let line = tokio::time::timeout(Duration::from_secs(15), stdout.next_line())
                .await
                .map_err(|_| anyhow!("MCP stdio server timed out responding to '{}'.", method))??
                .ok_or_else(|| anyhow!("MCP stdio server closed before responding."))?;
            let response: Value = serde_json::from_str(&line)
                .map_err(|error| anyhow!("Invalid MCP JSON-RPC response: {}", error))?;
            if response.get("id").and_then(Value::as_i64) != Some(id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                return Err(anyhow!("MCP '{}' error: {}", method, error));
            }
            return response
                .get("result")
                .cloned()
                .ok_or_else(|| anyhow!("MCP '{}' response has no result.", method));
        }
    }

    async fn remote_call(server: &McpServerRecord, method: &str, params: Value) -> Result<Value> {
        let endpoint = server
            .endpoint
            .as_deref()
            .filter(|endpoint| !endpoint.trim().is_empty())
            .ok_or_else(|| anyhow!("Remote MCP server has no endpoint."))?;
        let endpoint_url = reqwest::Url::parse(endpoint)
            .map_err(|error| anyhow!("Remote MCP endpoint is invalid: {}", error))?;
        if endpoint_url.scheme() != "https" {
            return Err(anyhow!("Remote MCP endpoint must use HTTPS."));
        }
        let host = endpoint_url
            .host_str()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1")
            || host.ends_with(".localhost")
        {
            return Err(anyhow!(
                "Remote MCP endpoint cannot target a loopback host."
            ));
        }
        let headers = Self::resolved_headers(server).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let initialize = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "agentforge-ui", "version": "0.1.0"}
            }
        });
        let initialize_response =
            Self::post_remote(&client, endpoint, &headers, None, &initialize).await?;
        if let Some(error) = initialize_response.0.get("error") {
            return Err(anyhow!("MCP initialize error: {}", error));
        }
        let session_id = initialize_response.1;
        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let _ = Self::post_remote(
            &client,
            endpoint,
            &headers,
            session_id.as_deref(),
            &initialized,
        )
        .await;
        let request = json!({"jsonrpc": "2.0", "id": 2, "method": method, "params": params});
        let (response, _) =
            Self::post_remote(&client, endpoint, &headers, session_id.as_deref(), &request).await?;
        if let Some(error) = response.get("error") {
            return Err(anyhow!("MCP '{}' error: {}", method, error));
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow!("MCP '{}' response has no result.", method))
    }

    async fn post_remote(
        client: &reqwest::Client,
        endpoint: &str,
        headers: &HashMap<String, String>,
        session_id: Option<&str>,
        body: &Value,
    ) -> Result<(Value, Option<String>)> {
        let mut request = client
            .post(endpoint)
            .header("Accept", "application/json, text/event-stream")
            .header("Content-Type", "application/json");
        for (name, value) in headers {
            request = request.header(name, value);
        }
        if let Some(session_id) = session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }
        let response = request
            .json(body)
            .send()
            .await
            .map_err(|error| anyhow!("Unable to contact remote MCP server: {}", error))?;
        let session_id = response
            .headers()
            .get("Mcp-Session-Id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let status = response.status();
        let body = response
            .json::<Value>()
            .await
            .map_err(|error| anyhow!("Invalid remote MCP JSON response: {}", error))?;
        if !status.is_success() {
            return Err(anyhow!("Remote MCP HTTP {}: {}", status, body));
        }
        Ok((body, session_id))
    }
}
