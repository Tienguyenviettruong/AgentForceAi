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

#[derive(serde::Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

#[derive(serde::Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_usage: Option<bool>,
}

#[derive(serde::Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(serde::Deserialize)]
struct Choice {
    message: OpenRouterResponseMessage,
}

#[derive(serde::Deserialize)]
struct OpenRouterResponseMessage {
    content: Option<String>,
}

#[derive(serde::Deserialize)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

pub struct OpenRouterAdapter {
    config: Option<crate::db::Provider>,
    client: reqwest::Client,
}

impl Default for OpenRouterAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenRouterAdapter {
    pub fn new() -> Self {
        let _guard = get_runtime().enter();
        Self {
            config: None,
            client: reqwest::Client::new(),
        }
    }
}

impl BaseProviderAdapter for OpenRouterAdapter {
    fn provider_id(&self) -> &'static str {
        "openrouter"
    }

    fn initialize(&mut self, config: &crate::db::Provider) -> Result<()> {
        self.config = Some(config.clone());
        Ok(())
    }

    fn send_message(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, anyhow::Error>> + Send>> {
        let config = self.config.clone();
        let client = self.client.clone();
        
        Box::pin(async move {
            let config = config.ok_or_else(|| anyhow!("Adapter not initialized"))?;
            let api_key = match config.api_key_ref.clone() {
                Some(v) if v.starts_with("env:") => env::var(v.trim_start_matches("env:"))
                    .ok()
                    .ok_or_else(|| anyhow!("API key env var missing: {}", v.trim_start_matches("env:")))?,
                Some(v) => v,
                None => env::var("AGENTFORGE_OPENROUTER_API_KEY")
                    .ok()
                    .or_else(|| env::var("OPENROUTER_API_KEY").ok())
                    .ok_or_else(|| anyhow!("API key missing (set provider api_key_ref or env AGENTFORGE_OPENROUTER_API_KEY)"))?,
            };
            let model = config.model;

            let req_messages: Vec<OpenRouterMessage> = messages.into_iter().map(|m| OpenRouterMessage {
                role: m.role.to_string(),
                content: m.content.to_string(),
            }).collect();

            let request_body = OpenRouterRequest {
                model,
                messages: req_messages,
                stream: None,
                include_usage: Some(true),
            };

            let request_future = async move {
                let response = client
                    .post("https://openrouter.ai/api/v1/chat/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("HTTP-Referer", "https://agentforge.local")
                    .header("X-Title", "AgentForgeAI")
                    .header("User-Agent", "AgentForgeAI")
                    .json(&request_body)
                    .send()
                    .await
                    .map_err(|e| anyhow!("error sending request for url (https://openrouter.ai/api/v1/chat/completions): {}", e))?;

                if !response.status().is_success() {
                    let status = response.status();
                    let text = response.text().await?;
                    return Err(anyhow!("OpenRouter API error: {} - {}", status, text));
                }

                let response_body: OpenRouterResponse = response.json().await?;
                let content = response_body
                    .choices
                    .first()
                    .and_then(|c| c.message.content.clone())
                    .unwrap_or_default();

                let token_usage = response_body.usage.map(|u| TokenUsage {
                    input_tokens: u.prompt_tokens,
                    output_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                }).unwrap_or_default();

                Ok(ChatResponse {
                    content: SharedString::from(content),
                    token_usage,
                })
            };

            match tokio::runtime::Handle::try_current() {
                Ok(_) => request_future.await,
                Err(_) => get_runtime()
                    .spawn(request_future)
                    .await
                    .map_err(|e| anyhow!("OpenRouter join error: {}", e))?,
            }
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
                None => env::var("AGENTFORGE_OPENROUTER_API_KEY")
                    .ok()
                    .or_else(|| env::var("OPENROUTER_API_KEY").ok())
                    .ok_or_else(|| anyhow!("API key missing (set provider api_key_ref or env AGENTFORGE_OPENROUTER_API_KEY)"))?,
            };
            let model = config.model;

            let req_messages: Vec<OpenRouterMessage> = messages.into_iter().map(|m| OpenRouterMessage {
                role: m.role.to_string(),
                content: m.content.to_string(),
            }).collect();

            let request_body = serde_json::json!({
                "model": model,
                "messages": req_messages,
                "stream": true
            });

            let req = client
                .post("https://openrouter.ai/api/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("HTTP-Referer", "https://agentforge.local")
                .header("X-Title", "AgentForgeAI")
                .header("User-Agent", "AgentForgeAI")
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
                            if message.data == "[DONE]" {
                                let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Done(crate::providers::TokenUsage::default())));
                                break;
                            }
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&message.data) {
                                if let Some(content) = v["choices"][0]["delta"]["content"].as_str() {
                                    let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Text(content.to_string())));
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

    fn check_health(&self) -> Pin<Box<dyn Future<Output = Result<bool, anyhow::Error>> + Send>> {
        Box::pin(async move { Ok(true) })
    }
}
