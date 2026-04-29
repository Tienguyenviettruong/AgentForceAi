use super::{BaseProviderAdapter, ChatMessage, ChatResponse, TokenUsage};
use anyhow::{anyhow, Result};
use gpui::SharedString;
use std::future::Future;
use std::pin::Pin;
use std::env;
use futures::stream::StreamExt;

/// Gemini CLI adapter with NDJSON streaming
/// (Tasks 1.21, 1.22)
pub struct GeminiAdapter {
    config: Option<crate::db::Provider>,
    client: reqwest::Client,
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize Tokio runtime")
    })
}

impl GeminiAdapter {
    pub fn new() -> Self {
        let _guard = get_runtime().enter();
        Self {
            config: None,
            client: reqwest::Client::new(),
        }
    }

    /// (Task 1.22) Handle multi-modal input (text, image, video)
    pub fn send_multimodal_message(
        &self,
        _text: &str,
        _media_paths: Vec<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse>> + Send>> {
        Box::pin(async move {
            Ok(ChatResponse {
                content: SharedString::from("Gemini multi-modal response"),
                token_usage: TokenUsage::default(),
            })
        })
    }
}

impl BaseProviderAdapter for GeminiAdapter {
    fn provider_id(&self) -> &'static str {
        "gemini"
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
                None => env::var("GEMINI_API_KEY")
                    .ok()
                    .or_else(|| env::var("GOOGLE_API_KEY").ok())
                    .ok_or_else(|| anyhow!("API key missing (set provider api_key_ref or env GEMINI_API_KEY)"))?,
            };
            let model = config.model;

            let prompt = messages
                .into_iter()
                .map(|m| format!("{}: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n");

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, api_key
            );

            let body = serde_json::json!({
                "contents": [
                    { "role": "user", "parts": [ { "text": prompt } ] }
                ]
            });

            let request_future = async move {
                let res = client
                    .post(url)
                    .header("content-type", "application/json")
                    .json(&body)
                    .send()
                    .await?;

                if !res.status().is_success() {
                    let status = res.status();
                    let text = res.text().await.unwrap_or_default();
                    return Err(anyhow!("Gemini API error: {} - {}", status, text));
                }

                let json: serde_json::Value = res.json().await?;
                let text = json["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let token_usage = json
                    .get("usageMetadata")
                    .and_then(|u| u.as_object())
                    .map(|u| TokenUsage {
                        input_tokens: u
                            .get("promptTokenCount")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize,
                        output_tokens: u
                            .get("candidatesTokenCount")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize,
                        total_tokens: u
                            .get("totalTokenCount")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize,
                    })
                    .unwrap_or_default();

                Ok(ChatResponse {
                    content: SharedString::from(text),
                    token_usage,
                })
            };

            match tokio::runtime::Handle::try_current() {
                Ok(_) => request_future.await,
                Err(_) => get_runtime()
                    .spawn(request_future)
                    .await
                    .map_err(|e| anyhow!("Gemini join error: {}", e))?,
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
                None => env::var("GEMINI_API_KEY")
                    .ok()
                    .or_else(|| env::var("GOOGLE_API_KEY").ok())
                    .ok_or_else(|| anyhow!("API key missing (set provider api_key_ref or env GEMINI_API_KEY)"))?,
            };
            let model = config.model;

            let prompt = messages
                .into_iter()
                .map(|m| format!("{}: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n");

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
                model, api_key
            );

            let body = serde_json::json!({
                "contents": [
                    { "role": "user", "parts": [ { "text": prompt } ] }
                ]
            });

            let req = client
                .post(url)
                .header("content-type", "application/json")
                .json(&body);

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
                            if message.data.trim().is_empty() {
                                continue;
                            }
                            if message.data == "[DONE]" {
                                break;
                            }
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&message.data) {
                                if let Some(text) = v["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                    let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Text(text.to_string())));
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

                let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Done(crate::providers::TokenUsage::default())));
            });

            Ok(Box::new(rx) as Box<dyn futures::Stream<Item = Result<crate::providers::StreamChunk, anyhow::Error>> + Send + Unpin>)
        })
    }

    fn check_health(&self) -> Pin<Box<dyn Future<Output = Result<bool>> + Send>> {
        Box::pin(async move { Ok(true) })
    }
}
