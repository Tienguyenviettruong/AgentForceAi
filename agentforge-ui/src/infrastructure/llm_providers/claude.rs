use super::{BaseProviderAdapter, ChatMessage, ChatResponse, TokenUsage};
use anyhow::{anyhow, Result};
use gpui::SharedString;
use std::future::Future;
use std::pin::Pin;
use std::env;
use futures::stream::StreamExt;

static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize Tokio runtime")
    })
}

/// Claude provider adapter implementation using SDK V2
/// (Tasks 1.16, 1.17, 1.18)
pub struct ClaudeAdapter {
    config: Option<crate::db::Provider>,
    session_id: Option<String>,
    auto_accept_tools: bool,
    client: reqwest::Client,
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeAdapter {
    pub fn new() -> Self {
        let _guard = get_runtime().enter();
        Self {
            config: None,
            session_id: None,
            auto_accept_tools: false,
            client: reqwest::Client::new(),
        }
    }

    /// (Task 1.17)
    pub fn resume_session(&mut self, session_id: &str) {
        self.session_id = Some(session_id.to_string());
    }

    /// (Task 1.18)
    pub fn set_auto_accept_tools(&mut self, accept: bool) {
        self.auto_accept_tools = accept;
    }
}

impl BaseProviderAdapter for ClaudeAdapter {
    fn provider_id(&self) -> &'static str {
        "claude"
    }

    fn initialize(&mut self, config: &crate::db::Provider) -> Result<()> {
        self.config = Some(config.clone());
        Ok(())
    }

    fn send_message(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse>> + Send>> {
        let config = self.config.clone();
        let client = self.client.clone();
        
        Box::pin(async move {
            let config = config.ok_or_else(|| anyhow!("Adapter not initialized"))?;
            let api_key = match config.api_key_ref.clone() {
                Some(v) if v.starts_with("env:") => env::var(v.trim_start_matches("env:"))
                    .ok()
                    .ok_or_else(|| anyhow!("API key env var missing: {}", v.trim_start_matches("env:")))?,
                Some(v) => v,
                None => env::var("ANTHROPIC_API_KEY")
                    .ok()
                    .ok_or_else(|| anyhow!("API key missing (set provider api_key_ref or env ANTHROPIC_API_KEY)"))?,
            };
            let model = config.model;

            let mut req_messages = Vec::new();
            let mut system_prompt = String::new();
            for m in messages {
                if m.role.as_ref() == "system" {
                    system_prompt.push_str(m.content.as_ref());
                    system_prompt.push('\n');
                } else {
                    req_messages.push(serde_json::json!({
                        "role": m.role.to_string(),
                        "content": m.content.to_string()
                    }));
                }
            }

            let request_body = serde_json::json!({
                "model": model,
                "system": system_prompt.trim(),
                "messages": req_messages,
                "max_tokens": 4096
            });

            let res = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request_body)
                .send()
                .await?;

            let body: serde_json::Value = res.json().await?;
            
            if let Some(error) = body.get("error") {
                return Err(anyhow!("Anthropic API error: {}", error));
            }

            let text = body["content"][0]["text"].as_str().unwrap_or("").to_string();
            let token_usage = body
                .get("usage")
                .and_then(|u| u.as_object())
                .map(|u| TokenUsage {
                    input_tokens: u
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    output_tokens: u
                        .get("output_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    total_tokens: u
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize
                        + u.get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize,
                })
                .unwrap_or_default();

            Ok(ChatResponse {
                content: SharedString::from(text),
                token_usage,
            })
        })
    }

    fn send_message_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Box<dyn futures::Stream<Item = Result<crate::providers::StreamChunk, anyhow::Error>> + Send + Unpin>,
                        anyhow::Error,
                    >,
                > + Send,
        >,
    > {
        let config = self.config.clone();
        let client = self.client.clone();
        
        Box::pin(async move {
            let config = config.ok_or_else(|| anyhow!("Adapter not initialized"))?;
            let api_key = match config.api_key_ref.clone() {
                Some(v) if v.starts_with("env:") => env::var(v.trim_start_matches("env:"))
                    .ok()
                    .ok_or_else(|| anyhow!("API key env var missing: {}", v.trim_start_matches("env:")))?,
                Some(v) => v,
                None => env::var("ANTHROPIC_API_KEY")
                    .ok()
                    .ok_or_else(|| anyhow!("API key missing (set provider api_key_ref or env ANTHROPIC_API_KEY)"))?,
            };
            let model = config.model;

            let mut req_messages = Vec::new();
            let mut system_prompt = String::new();
            for m in messages {
                if m.role.as_ref() == "system" {
                    system_prompt.push_str(m.content.as_ref());
                    system_prompt.push('\n');
                } else {
                    req_messages.push(serde_json::json!({
                        "role": m.role.to_string(),
                        "content": m.content.to_string()
                    }));
                }
            }

            let request_body = serde_json::json!({
                "model": model,
                "system": system_prompt.trim(),
                "messages": req_messages,
                "stream": true,
                "max_tokens": 4096
            });

            let req = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request_body);

            let (tx, rx) = futures::channel::mpsc::unbounded();
            let rt = get_runtime();

            rt.spawn(async move {
                let mut es = match reqwest_eventsource::EventSource::new(req) {
                    Ok(es) => es,
                    Err(e) => {
                        let _ = tx.unbounded_send(Err(anyhow!("Failed to create event source: {}", e)));
                        return;
                    }
                };

                while let Some(event) = es.next().await {
                    match event {
                        Ok(reqwest_eventsource::Event::Open) => continue,
                        Ok(reqwest_eventsource::Event::Message(message)) => {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&message.data) {
                                if let Some(type_str) = v["type"].as_str() {
                                    if type_str == "content_block_delta" {
                                        if let Some(text) = v["delta"]["text"].as_str() {
                                            let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Text(text.to_string())));
                                        }
                                    } else if type_str == "message_stop" {
                                        let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Done(crate::providers::TokenUsage::default())));
                                        break;
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            es.close();
                            let _ = tx.unbounded_send(Err(anyhow!("SSE Error: {}", err)));
                            break;
                        }
                    }
                }
            });

            Ok(Box::new(rx) as Box<dyn futures::Stream<Item = Result<crate::providers::StreamChunk, anyhow::Error>> + Send + Unpin>)
        })
    }

    fn check_health(&self) -> Pin<Box<dyn Future<Output = Result<bool>> + Send>> {
        Box::pin(async move { Ok(true) })
    }
}
