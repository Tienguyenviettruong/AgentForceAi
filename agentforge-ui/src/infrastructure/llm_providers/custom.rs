use super::{BaseProviderAdapter, ChatMessage, ChatResponse, TokenUsage};
use crate::infrastructure::security::keychain::resolve_credential_reference;
use anyhow::{anyhow, Result};
use futures::stream::StreamExt;
use gpui::SharedString;
use std::env;
use std::future::Future;
use std::pin::Pin;

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

/// Custom provider adapter framework and SDK supporting both Claude-compatible and OpenAI-compatible dynamic URLs
/// (Task 1.26)
pub struct CustomAdapterSDK {
    config: Option<crate::db::Provider>,
    _plugin_name: String,
    client: reqwest::Client,
}

impl CustomAdapterSDK {
    pub fn new(plugin_name: &str) -> Self {
        Self {
            config: None,
            _plugin_name: plugin_name.to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn is_claude_format(&self, base_url: &str, model: &str) -> bool {
        let base_url = base_url.trim().to_lowercase();
        let model = model.trim().to_lowercase();
        if base_url.contains("/chat/completions") || base_url.contains("/responses") {
            return false;
        }
        if base_url.contains("/messages") {
            return true;
        }
        if model.contains("claude")
            || model.contains("sonnet")
            || model.contains("opus")
            || model.contains("haiku")
        {
            return true;
        }
        if base_url.contains("anthropic") {
            return true;
        }
        false
    }

    fn claude_messages_endpoint(base_url: &str) -> String {
        let base_url = base_url.trim().trim_matches('`').trim_end_matches('/');
        if base_url.contains("/messages") {
            base_url.to_string()
        } else if base_url.contains("/v1") || base_url.contains("/v2") {
            format!("{}/messages", base_url)
        } else {
            format!("{}/v1/messages", base_url)
        }
    }

    fn openai_chat_completions_endpoint(base_url: &str) -> String {
        let base_url = base_url.trim().trim_matches('`').trim_end_matches('/');
        if base_url.contains("/chat/completions") {
            base_url.to_string()
        } else if base_url.contains("/v1") || base_url.contains("/v2") {
            format!("{}/chat/completions", base_url)
        } else {
            format!("{}/v1/chat/completions", base_url)
        }
    }

    fn validate_model_id(model: &str) -> Result<()> {
        if model.trim().is_empty() || model.trim() == "custom-model" {
            return Err(anyhow!(
                "Custom provider model is not configured. Replace placeholder 'custom-model' with the provider's real model or deployment id in Settings > AI Provider."
            ));
        }
        Ok(())
    }

    fn truncate_error_body(body: &str) -> String {
        const MAX_ERROR_BODY: usize = 1500;
        let body = body.trim();
        if body.chars().count() <= MAX_ERROR_BODY {
            return body.to_string();
        }
        let truncated: String = body.chars().take(MAX_ERROR_BODY).collect();
        format!("{}... [truncated]", truncated)
    }

    fn http_status_error(status: reqwest::StatusCode, body: &str) -> anyhow::Error {
        let body = Self::truncate_error_body(body);
        if body.is_empty() {
            anyhow!("HTTP {}", status)
        } else {
            anyhow!("HTTP {} response body: {}", status, body)
        }
    }

    fn is_transient_stream_status(status: reqwest::StatusCode) -> bool {
        matches!(
            status,
            reqwest::StatusCode::TOO_MANY_REQUESTS
                | reqwest::StatusCode::BAD_GATEWAY
                | reqwest::StatusCode::SERVICE_UNAVAILABLE
                | reqwest::StatusCode::GATEWAY_TIMEOUT
        )
    }

    async fn retry_delay(attempt: usize) {
        let delay_ms = match attempt {
            0 => 500,
            1 => 1_500,
            _ => 3_000,
        };
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
    }

    fn next_sse_frame(buffer: &str) -> Option<(usize, usize)> {
        let lf = buffer.find("\n\n").map(|pos| (pos, 2));
        let crlf = buffer.find("\r\n\r\n").map(|pos| (pos, 4));
        match (lf, crlf) {
            (Some(lf), Some(crlf)) => Some(if lf.0 <= crlf.0 { lf } else { crlf }),
            (Some(lf), None) => Some(lf),
            (None, Some(crlf)) => Some(crlf),
            (None, None) => None,
        }
    }

    fn drain_sse_data_events(buffer: &mut String) -> Vec<String> {
        let mut events = Vec::new();
        while let Some((frame_end, separator_len)) = Self::next_sse_frame(buffer) {
            let frame = buffer[..frame_end].to_string();
            buffer.drain(..frame_end + separator_len);

            let data_lines = frame
                .lines()
                .filter_map(|line| {
                    line.trim_end_matches('\r')
                        .strip_prefix("data:")
                        .map(|data| data.trim_start().to_string())
                })
                .collect::<Vec<_>>();

            if !data_lines.is_empty() {
                events.push(data_lines.join("\n"));
            }
        }
        events
    }

    fn drain_line_delimited_events(buffer: &mut String) -> Vec<String> {
        let mut events = Vec::new();
        loop {
            let Some(line_end) = buffer.find('\n') else {
                break;
            };
            let line = buffer[..line_end].trim_end_matches('\r').trim().to_string();
            buffer.drain(..line_end + 1);

            if line.is_empty() || line.starts_with(':') || line.starts_with("event:") {
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim_start();
                if !data.is_empty() {
                    events.push(data.to_string());
                }
            } else if line.starts_with('{') || line.starts_with('[') {
                events.push(line);
            }
        }
        events
    }

    fn data_lines_from_frame(frame: &str) -> Option<String> {
        let data_lines = frame
            .lines()
            .filter_map(|line| {
                line.trim_end_matches('\r')
                    .strip_prefix("data:")
                    .map(|data| data.trim_start().to_string())
            })
            .collect::<Vec<_>>();
        (!data_lines.is_empty()).then(|| data_lines.join("\n"))
    }

    fn content_value_to_text(value: &serde_json::Value) -> Option<String> {
        if let Some(text) = value.as_str() {
            return (!text.is_empty()).then(|| text.to_string());
        }

        if let Some(parts) = value.as_array() {
            let text = parts
                .iter()
                .filter_map(|part| {
                    part.get("text")
                        .and_then(|v| v.as_str())
                        .or_else(|| part.get("content").and_then(|v| v.as_str()))
                })
                .collect::<Vec<_>>()
                .join("\n");
            return (!text.is_empty()).then_some(text);
        }

        None
    }

    fn text_from_provider_json(value: &serde_json::Value) -> Option<String> {
        value
            .get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| {
                choice
                    .get("delta")
                    .and_then(|delta| delta.get("content"))
                    .and_then(Self::content_value_to_text)
                    .or_else(|| {
                        choice
                            .get("message")
                            .and_then(|message| message.get("content"))
                            .and_then(Self::content_value_to_text)
                    })
                    .or_else(|| choice.get("text").and_then(Self::content_value_to_text))
            })
            .or_else(|| {
                value
                    .get("delta")
                    .and_then(|delta| delta.get("text"))
                    .and_then(Self::content_value_to_text)
            })
            .or_else(|| value.get("content").and_then(Self::content_value_to_text))
            .or_else(|| {
                value
                    .get("output_text")
                    .and_then(Self::content_value_to_text)
            })
            .or_else(|| value.get("text").and_then(Self::content_value_to_text))
    }

    fn update_usage_from_provider_json(
        usage: &mut crate::providers::TokenUsage,
        value: &serde_json::Value,
    ) {
        let usage_value = value.get("usage").or_else(|| {
            value
                .get("message")
                .and_then(|message| message.get("usage"))
        });
        let Some(usage_object) = usage_value.and_then(|usage| usage.as_object()) else {
            return;
        };

        if let Some(input) = usage_object
            .get("prompt_tokens")
            .or_else(|| usage_object.get("input_tokens"))
            .and_then(|value| value.as_u64())
        {
            usage.input_tokens = input as usize;
        }
        if let Some(output) = usage_object
            .get("completion_tokens")
            .or_else(|| usage_object.get("output_tokens"))
            .and_then(|value| value.as_u64())
        {
            usage.output_tokens = output as usize;
        }
        usage.total_tokens = usage_object
            .get("total_tokens")
            .and_then(|value| value.as_u64())
            .map(|value| value as usize)
            .unwrap_or_else(|| usage.input_tokens + usage.output_tokens);
    }

    fn text_from_provider_payload(
        payload: &str,
        usage: &mut crate::providers::TokenUsage,
    ) -> Option<String> {
        let payload = payload.trim();
        if payload.is_empty() || payload == "[DONE]" {
            return None;
        }

        let data = Self::data_lines_from_frame(payload).unwrap_or_else(|| payload.to_string());
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            return None;
        }

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
            Self::update_usage_from_provider_json(usage, &value);
            return Self::text_from_provider_json(&value);
        }

        (!data.starts_with('{') && !data.starts_with('[')).then(|| data.to_string())
    }

    fn credential_source(config: &crate::db::Provider, fallback_env_keys: &[&str]) -> String {
        match config.api_key_ref.as_deref().map(str::trim) {
            Some(value) if value.starts_with("secret://") => value.to_string(),
            Some(value) if value.starts_with("env:") => value.to_string(),
            Some(value) if !value.is_empty() => "blocked-raw-credential-reference".to_string(),
            _ => format!("fallback-env:{}", fallback_env_keys.join("|")),
        }
    }

    fn stream_context_error(
        provider_name: &str,
        protocol: &str,
        endpoint: &str,
        model: &str,
        credential_source: &str,
        error: impl std::fmt::Display,
    ) -> anyhow::Error {
        anyhow!(
            "Custom provider stream failed: provider='{}', protocol='{}', endpoint='{}', model='{}', credential_source='{}': {}. If the status is 401/403, update the provider credential or verify that the selected protocol/base URL matches the custom provider. If the status is 429/502/503/504, the upstream provider or gateway is overloaded/unavailable; AgentForge retries transient stream setup automatically, then surfaces the final failure.",
            provider_name,
            protocol,
            endpoint,
            model,
            credential_source,
            error
        )
    }
}

impl BaseProviderAdapter for CustomAdapterSDK {
    fn provider_id(&self) -> &'static str {
        "custom"
    }

    fn capabilities(&self) -> crate::core::models::ModelCapability {
        if let Some(config) = &self.config {
            if let Some(cap) = &config.capabilities {
                return cap.clone();
            }
        }
        crate::core::models::ModelCapability::text_only()
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
        let is_claude = if let Some(ref c) = config {
            let base_url = c.command.clone().unwrap_or_default();
            let model = c.model.clone();
            self.is_claude_format(&base_url, &model)
        } else {
            false
        };

        if is_claude {
            // Claude-compatible REST call
            Box::pin(async move {
                let config = config.ok_or_else(|| anyhow!("Adapter not initialized"))?;
                let api_key = resolve_credential_reference(
                    config.api_key_ref.as_deref(),
                    &[
                        "ANTHROPIC_AUTH_TOKEN",
                        "ANTHROPIC_API_KEY",
                        "CUSTOM_API_KEY",
                    ],
                )
                .await?
                .ok_or_else(|| anyhow!("API key missing"))?;

                let model = match config.model.as_str() {
                    "opus" => env::var("ANTHROPIC_DEFAULT_OPUS_MODEL")
                        .unwrap_or_else(|_| config.model.clone()),
                    "sonnet" => env::var("ANTHROPIC_DEFAULT_SONNET_MODEL")
                        .unwrap_or_else(|_| config.model.clone()),
                    "haiku" => env::var("ANTHROPIC_DEFAULT_HAIKU_MODEL")
                        .unwrap_or_else(|_| config.model.clone()),
                    _ => config.model.clone(),
                };
                CustomAdapterSDK::validate_model_id(&model)?;

                let base_url = config
                    .command
                    .clone()
                    .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
                let endpoint = CustomAdapterSDK::claude_messages_endpoint(&base_url);

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

                let mut req = client
                    .post(&endpoint)
                    .header("x-api-key", api_key.clone())
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&request_body);
                // Also add Authorization Bearer if custom URL
                req = req.header("authorization", format!("Bearer {}", api_key));

                let request_future = async move {
                    let res = req.send().await.map_err(|e| {
                        anyhow!("error sending custom request for url ({}): {}", endpoint, e)
                    })?;

                    let body: serde_json::Value = res.json().await?;

                    if let Some(error) = body.get("error") {
                        return Err(anyhow!("Anthropic API error: {}", error));
                    }

                    let text = body["content"][0]["text"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
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
                                + u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                                    as usize,
                        })
                        .unwrap_or_default();

                    Ok(ChatResponse {
                        content: SharedString::from(text),
                        token_usage,
                    })
                };

                request_future.await
            })
        } else {
            // OpenAI-compatible REST call
            Box::pin(async move {
                let config = config.ok_or_else(|| anyhow!("Adapter not initialized"))?;
                let api_key = resolve_credential_reference(
                    config.api_key_ref.as_deref(),
                    &["OPENAI_API_KEY", "CUSTOM_API_KEY"],
                )
                .await?
                .ok_or_else(|| anyhow!("API key missing"))?;
                let model = config.model;
                CustomAdapterSDK::validate_model_id(&model)?;

                let req_messages: Vec<OpenRouterMessage> = messages
                    .into_iter()
                    .map(|m| OpenRouterMessage {
                        role: m.role.to_string(),
                        content: m.content.to_string(),
                    })
                    .collect();

                let request_body = OpenRouterRequest {
                    model,
                    messages: req_messages,
                    stream: None,
                    include_usage: Some(true),
                };

                let base_url = config
                    .command
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let endpoint = CustomAdapterSDK::openai_chat_completions_endpoint(&base_url);

                let request_future = async move {
                    let response = client
                        .post(&endpoint)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .header("content-type", "application/json")
                        .json(&request_body)
                        .send()
                        .await
                        .map_err(|e| {
                            anyhow!("error sending custom request for url ({}): {}", endpoint, e)
                        })?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await?;
                        return Err(anyhow!("OpenAI API error: {} - {}", status, text));
                    }

                    let response_body: OpenRouterResponse = response.json().await?;
                    let content = response_body
                        .choices
                        .first()
                        .and_then(|c| c.message.content.clone())
                        .unwrap_or_default();

                    let token_usage = response_body
                        .usage
                        .map(|u| TokenUsage {
                            input_tokens: u.prompt_tokens,
                            output_tokens: u.completion_tokens,
                            total_tokens: u.total_tokens,
                        })
                        .unwrap_or_default();

                    Ok(ChatResponse {
                        content: SharedString::from(content),
                        token_usage,
                    })
                };

                request_future.await
            })
        }
    }

    fn send_message_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Box<
                            dyn futures::Stream<
                                    Item = Result<crate::providers::StreamChunk, anyhow::Error>,
                                > + Send
                                + Unpin,
                        >,
                    >,
                > + Send,
        >,
    > {
        let config = self.config.clone();
        let client = self.client.clone();
        let is_claude = if let Some(ref c) = config {
            let base_url = c.command.clone().unwrap_or_default();
            let model = c.model.clone();
            self.is_claude_format(&base_url, &model)
        } else {
            false
        };

        if is_claude {
            // Claude-style streaming
            Box::pin(async move {
                let config = config.ok_or_else(|| anyhow!("Adapter not initialized"))?;
                let fallback_env_keys = [
                    "ANTHROPIC_AUTH_TOKEN",
                    "ANTHROPIC_API_KEY",
                    "CUSTOM_API_KEY",
                ];
                let credential_source =
                    CustomAdapterSDK::credential_source(&config, &fallback_env_keys);
                let api_key =
                    resolve_credential_reference(config.api_key_ref.as_deref(), &fallback_env_keys)
                        .await?
                        .ok_or_else(|| anyhow!("API key missing"))?;
                tracing::debug!("Custom Claude-compatible stream credential resolved");

                let model = match config.model.as_str() {
                    "opus" => env::var("ANTHROPIC_DEFAULT_OPUS_MODEL")
                        .unwrap_or_else(|_| config.model.clone()),
                    "sonnet" => env::var("ANTHROPIC_DEFAULT_SONNET_MODEL")
                        .unwrap_or_else(|_| config.model.clone()),
                    "haiku" => env::var("ANTHROPIC_DEFAULT_HAIKU_MODEL")
                        .unwrap_or_else(|_| config.model.clone()),
                    _ => config.model.clone(),
                };
                CustomAdapterSDK::validate_model_id(&model)?;

                let base_url = config
                    .command
                    .clone()
                    .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
                let endpoint = CustomAdapterSDK::claude_messages_endpoint(&base_url);
                let provider_name_for_error = config.provider_name.clone();
                let endpoint_for_error = endpoint.clone();
                let model_for_error = model.clone();
                let credential_source_for_error = credential_source.clone();

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

                let (tx, rx) = futures::channel::mpsc::unbounded();
                tracing::debug!("Custom Claude-compatible stream opening SSE connection");
                tokio::spawn(async move {
                    let mut usage = TokenUsage::default();
                    let mut response = None;
                    for attempt in 0..3 {
                        let req = client
                            .post(&endpoint)
                            .header("x-api-key", api_key.clone())
                            .header("anthropic-version", "2023-06-01")
                            .header("accept", "text/event-stream")
                            .header("cache-control", "no-cache")
                            .header("connection", "keep-alive")
                            .header("accept-encoding", "identity")
                            .header("x-accel-buffering", "no")
                            .header("content-type", "application/json")
                            .header("user-agent", "AgentForgeAI")
                            .header("authorization", format!("Bearer {}", api_key))
                            .json(&request_body);
                        match req.send().await {
                            Ok(candidate) if candidate.status().is_success() => {
                                response = Some(candidate);
                                break;
                            }
                            Ok(candidate) => {
                                let status = candidate.status();
                                let body = candidate.text().await.unwrap_or_default();
                                if CustomAdapterSDK::is_transient_stream_status(status)
                                    && attempt < 2
                                {
                                    tracing::warn!(
                                        status = %status,
                                        attempt = attempt + 1,
                                        "Custom Claude-compatible stream got transient HTTP status; retrying"
                                    );
                                    CustomAdapterSDK::retry_delay(attempt).await;
                                    continue;
                                }
                                let _ =
                                    tx.unbounded_send(Err(CustomAdapterSDK::stream_context_error(
                                        &provider_name_for_error,
                                        "claude-compatible",
                                        &endpoint_for_error,
                                        &model_for_error,
                                        &credential_source_for_error,
                                        CustomAdapterSDK::http_status_error(status, &body),
                                    )));
                                return;
                            }
                            Err(e) if attempt < 2 => {
                                tracing::warn!(
                                    error = %e,
                                    attempt = attempt + 1,
                                    "Custom Claude-compatible stream connection failed; retrying"
                                );
                                CustomAdapterSDK::retry_delay(attempt).await;
                                continue;
                            }
                            Err(e) => {
                                let _ =
                                    tx.unbounded_send(Err(CustomAdapterSDK::stream_context_error(
                                        &provider_name_for_error,
                                        "claude-compatible",
                                        &endpoint_for_error,
                                        &model_for_error,
                                        &credential_source_for_error,
                                        e,
                                    )));
                                return;
                            }
                        }
                    }

                    let Some(response) = response else {
                        let _ = tx.unbounded_send(Err(CustomAdapterSDK::stream_context_error(
                            &provider_name_for_error,
                            "claude-compatible",
                            &endpoint_for_error,
                            &model_for_error,
                            &credential_source_for_error,
                            anyhow!("Stream connection failed before receiving a response"),
                        )));
                        return;
                    };

                    let mut stream = response.bytes_stream();
                    let mut buffer = String::new();
                    while let Some(chunk) = stream.next().await {
                        let bytes = match chunk {
                            Ok(bytes) => bytes,
                            Err(err) => {
                                let _ =
                                    tx.unbounded_send(Err(CustomAdapterSDK::stream_context_error(
                                        &provider_name_for_error,
                                        "claude-compatible",
                                        &endpoint_for_error,
                                        &model_for_error,
                                        &credential_source_for_error,
                                        err,
                                    )));
                                return;
                            }
                        };
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        let mut events = CustomAdapterSDK::drain_sse_data_events(&mut buffer);
                        events.extend(CustomAdapterSDK::drain_line_delimited_events(&mut buffer));
                        for data in events {
                            if data == "[DONE]" {
                                usage.total_tokens = usage.input_tokens + usage.output_tokens;
                                let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Done(
                                    usage.clone(),
                                )));
                                return;
                            }
                            if let Some(text) =
                                CustomAdapterSDK::text_from_provider_payload(&data, &mut usage)
                            {
                                let _ = tx
                                    .unbounded_send(Ok(crate::providers::StreamChunk::Text(text)));
                            }
                        }
                    }
                    if let Some(text) =
                        CustomAdapterSDK::text_from_provider_payload(&buffer, &mut usage)
                    {
                        let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Text(text)));
                    }
                    usage.total_tokens = usage.input_tokens + usage.output_tokens;
                    let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Done(usage)));
                });
                tracing::debug!("Custom Claude-compatible stream task spawned");

                Ok(Box::new(rx)
                    as Box<
                        dyn futures::Stream<
                                Item = Result<crate::providers::StreamChunk, anyhow::Error>,
                            > + Send
                            + Unpin,
                    >)
            })
        } else {
            // OpenAI-style streaming
            Box::pin(async move {
                let config = config.ok_or_else(|| anyhow!("Adapter not initialized"))?;
                let fallback_env_keys = ["OPENAI_API_KEY", "CUSTOM_API_KEY"];
                let credential_source =
                    CustomAdapterSDK::credential_source(&config, &fallback_env_keys);
                let api_key =
                    resolve_credential_reference(config.api_key_ref.as_deref(), &fallback_env_keys)
                        .await?
                        .ok_or_else(|| anyhow!("API key missing"))?;
                tracing::debug!("Custom OpenAI-compatible stream credential resolved");
                let model = config.model;
                CustomAdapterSDK::validate_model_id(&model)?;

                let req_messages: Vec<OpenRouterMessage> = messages
                    .into_iter()
                    .map(|m| OpenRouterMessage {
                        role: m.role.to_string(),
                        content: m.content.to_string(),
                    })
                    .collect();

                let request_body = serde_json::json!({
                    "model": model,
                    "messages": req_messages,
                    "stream": true,
                    "include_usage": true
                });

                let base_url = config
                    .command
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let endpoint = CustomAdapterSDK::openai_chat_completions_endpoint(&base_url);
                let provider_name_for_error = config.provider_name.clone();
                let endpoint_for_error = endpoint.clone();
                let model_for_error = model.clone();
                let credential_source_for_error = credential_source.clone();

                let (tx, rx) = futures::channel::mpsc::unbounded();
                tokio::spawn(async move {
                    let mut response = None;
                    for attempt in 0..3 {
                        let req = client
                            .post(&endpoint)
                            .header("Authorization", format!("Bearer {}", api_key))
                            .header("accept", "text/event-stream")
                            .header("cache-control", "no-cache")
                            .header("connection", "keep-alive")
                            .header("accept-encoding", "identity")
                            .header("x-accel-buffering", "no")
                            .header("content-type", "application/json")
                            .header("user-agent", "AgentForgeAI")
                            .json(&request_body);
                        match req.send().await {
                            Ok(candidate) if candidate.status().is_success() => {
                                response = Some(candidate);
                                break;
                            }
                            Ok(candidate) => {
                                let status = candidate.status();
                                let body = candidate.text().await.unwrap_or_default();
                                if CustomAdapterSDK::is_transient_stream_status(status)
                                    && attempt < 2
                                {
                                    tracing::warn!(
                                        status = %status,
                                        attempt = attempt + 1,
                                        "Custom OpenAI-compatible stream got transient HTTP status; retrying"
                                    );
                                    CustomAdapterSDK::retry_delay(attempt).await;
                                    continue;
                                }
                                let _ =
                                    tx.unbounded_send(Err(CustomAdapterSDK::stream_context_error(
                                        &provider_name_for_error,
                                        "openai-compatible",
                                        &endpoint_for_error,
                                        &model_for_error,
                                        &credential_source_for_error,
                                        CustomAdapterSDK::http_status_error(status, &body),
                                    )));
                                return;
                            }
                            Err(e) if attempt < 2 => {
                                tracing::warn!(
                                    error = %e,
                                    attempt = attempt + 1,
                                    "Custom OpenAI-compatible stream connection failed; retrying"
                                );
                                CustomAdapterSDK::retry_delay(attempt).await;
                                continue;
                            }
                            Err(e) => {
                                let _ =
                                    tx.unbounded_send(Err(CustomAdapterSDK::stream_context_error(
                                        &provider_name_for_error,
                                        "openai-compatible",
                                        &endpoint_for_error,
                                        &model_for_error,
                                        &credential_source_for_error,
                                        e,
                                    )));
                                return;
                            }
                        }
                    }

                    let Some(response) = response else {
                        let _ = tx.unbounded_send(Err(CustomAdapterSDK::stream_context_error(
                            &provider_name_for_error,
                            "openai-compatible",
                            &endpoint_for_error,
                            &model_for_error,
                            &credential_source_for_error,
                            anyhow!("Stream connection failed before receiving a response"),
                        )));
                        return;
                    };

                    let mut usage = crate::providers::TokenUsage::default();
                    let mut stream = response.bytes_stream();
                    let mut buffer = String::new();
                    while let Some(chunk) = stream.next().await {
                        let bytes = match chunk {
                            Ok(bytes) => bytes,
                            Err(err) => {
                                let _ =
                                    tx.unbounded_send(Err(CustomAdapterSDK::stream_context_error(
                                        &provider_name_for_error,
                                        "openai-compatible",
                                        &endpoint_for_error,
                                        &model_for_error,
                                        &credential_source_for_error,
                                        err,
                                    )));
                                return;
                            }
                        };
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        let mut events = CustomAdapterSDK::drain_sse_data_events(&mut buffer);
                        events.extend(CustomAdapterSDK::drain_line_delimited_events(&mut buffer));
                        for data in events {
                            if data == "[DONE]" {
                                let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Done(
                                    usage.clone(),
                                )));
                                return;
                            }
                            if let Some(text) =
                                CustomAdapterSDK::text_from_provider_payload(&data, &mut usage)
                            {
                                let _ = tx
                                    .unbounded_send(Ok(crate::providers::StreamChunk::Text(text)));
                            }
                        }
                    }
                    if let Some(text) =
                        CustomAdapterSDK::text_from_provider_payload(&buffer, &mut usage)
                    {
                        let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Text(text)));
                    }
                    let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Done(usage)));
                });

                Ok(Box::new(rx)
                    as Box<
                        dyn futures::Stream<
                                Item = Result<crate::providers::StreamChunk, anyhow::Error>,
                            > + Send
                            + Unpin,
                    >)
            })
        }
    }

    fn check_health(&self) -> Pin<Box<dyn Future<Output = Result<bool>> + Send>> {
        Box::pin(async move { Ok(true) })
    }
}

#[cfg(test)]
mod tests {
    use super::CustomAdapterSDK;

    #[test]
    fn claude_endpoint_builder_handles_bare_v1_and_full_paths() {
        assert_eq!(
            CustomAdapterSDK::claude_messages_endpoint("https://api.claudepro.vn"),
            "https://api.claudepro.vn/v1/messages"
        );
        assert_eq!(
            CustomAdapterSDK::claude_messages_endpoint("https://api.claudepro.vn/v1"),
            "https://api.claudepro.vn/v1/messages"
        );
        assert_eq!(
            CustomAdapterSDK::claude_messages_endpoint("https://api.claudepro.vn/v1/messages"),
            "https://api.claudepro.vn/v1/messages"
        );
    }

    #[test]
    fn openai_endpoint_builder_handles_bare_v1_and_full_paths() {
        assert_eq!(
            CustomAdapterSDK::openai_chat_completions_endpoint("https://api.example.test"),
            "https://api.example.test/v1/chat/completions"
        );
        assert_eq!(
            CustomAdapterSDK::openai_chat_completions_endpoint("https://api.example.test/v1"),
            "https://api.example.test/v1/chat/completions"
        );
        assert_eq!(
            CustomAdapterSDK::openai_chat_completions_endpoint(
                "https://api.example.test/v1/chat/completions"
            ),
            "https://api.example.test/v1/chat/completions"
        );
    }

    #[test]
    fn protocol_detection_uses_explicit_endpoint_or_model_not_hostname_branding() {
        let adapter = CustomAdapterSDK::new("test");
        assert!(!adapter.is_claude_format("https://api.claudepro.vn", "custom-model"));
        assert!(adapter.is_claude_format("https://api.claudepro.vn/v1/messages", "custom-model"));
        assert!(!adapter.is_claude_format(
            "https://api.claudepro.vn/v1/chat/completions",
            "custom-model"
        ));
        assert!(adapter.is_claude_format("https://api.proxy.example/v1", "claude-sonnet-4-5"));
    }

    #[test]
    fn placeholder_custom_model_is_rejected_before_network_call() {
        assert!(CustomAdapterSDK::validate_model_id("custom-model").is_err());
        assert!(CustomAdapterSDK::validate_model_id("   ").is_err());
        assert!(CustomAdapterSDK::validate_model_id("gpt-4o-mini").is_ok());
    }

    #[test]
    fn sse_parser_extracts_data_frames_with_crlf() {
        let mut buffer =
            "event: message\r\ndata: {\"ok\":true}\r\n\r\ndata: [DONE]\r\n\r\n".to_string();
        assert_eq!(
            CustomAdapterSDK::drain_sse_data_events(&mut buffer),
            vec!["{\"ok\":true}".to_string(), "[DONE]".to_string()]
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn stream_parser_extracts_line_delimited_data_events() {
        let mut buffer = "data: {\"choices\":[{\"delta\":{\"content\":\"xin\"}}]}\n\
             data: {\"choices\":[{\"delta\":{\"content\":\" chao\"}}]}\n"
            .to_string();
        assert_eq!(
            CustomAdapterSDK::drain_line_delimited_events(&mut buffer),
            vec![
                "{\"choices\":[{\"delta\":{\"content\":\"xin\"}}]}".to_string(),
                "{\"choices\":[{\"delta\":{\"content\":\" chao\"}}]}".to_string()
            ]
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn provider_payload_parser_accepts_openai_json_without_sse_frames() {
        let mut usage = crate::providers::TokenUsage::default();
        let text = CustomAdapterSDK::text_from_provider_payload(
            r#"{"choices":[{"message":{"content":"hello"}}],"usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}}"#,
            &mut usage,
        );
        assert_eq!(text.as_deref(), Some("hello"));
        assert_eq!(usage.input_tokens, 2);
        assert_eq!(usage.output_tokens, 3);
        assert_eq!(usage.total_tokens, 5);
    }

    #[test]
    fn provider_payload_parser_accepts_claude_stream_json() {
        let mut usage = crate::providers::TokenUsage::default();
        let text = CustomAdapterSDK::text_from_provider_payload(
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"xin chao"}}"#,
            &mut usage,
        );
        assert_eq!(text.as_deref(), Some("xin chao"));
    }

    #[test]
    fn provider_payload_parser_preserves_streamed_markdown_whitespace() {
        let payloads = [
            r####"{"choices":[{"delta":{"content":"### Part 1"}}]}"####,
            r#"{"choices":[{"delta":{"content":"\n\n"}}]}"#,
            r#"{"choices":[{"delta":{"content":"| Framework | Type |"}}]}"#,
            r#"{"choices":[{"delta":{"content":"\n"}}]}"#,
            r#"{"choices":[{"delta":{"content":"| --- | --- |"}}]}"#,
            r#"{"choices":[{"delta":{"content":"\n"}}]}"#,
            r#"{"choices":[{"delta":{"content":"| Gin | Micro |"}}]}"#,
        ];
        let mut usage = crate::providers::TokenUsage::default();
        let markdown = payloads
            .iter()
            .filter_map(|payload| CustomAdapterSDK::text_from_provider_payload(payload, &mut usage))
            .collect::<String>();

        assert_eq!(
            markdown,
            "### Part 1\n\n| Framework | Type |\n| --- | --- |\n| Gin | Micro |"
        );
    }
}
