use gpui::{
    div, img, linear_color_stop, linear_gradient, prelude::FluentBuilder, px, Animation,
    AnimationExt, AppContext, Context, Corner, CursorStyle, InteractiveElement, IntoElement,
    MouseButton, ObjectFit, ParentElement, StatefulInteractiveElement, Styled, StyledImage, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, RopeExt as _},
    popover::Popover,
    scroll::ScrollableElement,
    slider::Slider,
    v_flex, ActiveTheme as _, Icon, IconName, Selectable, Sizable,
};

use super::{
    model::{
        solo_templates, SoloActivity, SoloActivityKind, SoloActivityStatus, SoloMessage,
        SoloMessageMetadata, SoloSection, SoloTemplate, SoloTool,
    },
    SoloWorkspacePanel,
};

use futures::StreamExt as _;
use std::time::Duration;

const SOLO_COLLAPSE_LINE_LIMIT: usize = 80;
const SOLO_COLLAPSE_CHAR_LIMIT: usize = 4_000;
const SOLO_RENDER_CHAR_LIMIT: usize = 30_000;
const SOLO_AUTO_PROVIDER_SETTING: &str = "solo_auto_provider_id";

enum SoloStreamEvent {
    ProviderSelected(String),
    Activity(SoloActivity),
    Delta(String),
    Finished(Result<(), String>),
}

#[derive(Clone, serde::Deserialize)]
struct SoloToolCall {
    #[serde(default)]
    id: String,
    name: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

#[derive(Clone)]
struct SoloMentionOption {
    id: String,
    label: String,
    detail: String,
    icon: IconName,
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct SoloWebviewParent {
    hwnd: std::num::NonZeroIsize,
}

#[cfg(target_os = "windows")]
impl raw_window_handle::HasWindowHandle for SoloWebviewParent {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let handle = raw_window_handle::Win32WindowHandle::new(self.hwnd);
        Ok(unsafe {
            raw_window_handle::WindowHandle::borrow_raw(raw_window_handle::RawWindowHandle::Win32(
                handle,
            ))
        })
    }
}

#[cfg(target_os = "windows")]
fn webview_parent_from_window(window: &Window) -> Result<SoloWebviewParent, String> {
    let raw_handle = raw_window_handle::HasWindowHandle::window_handle(window)
        .map_err(|error| error.to_string())?
        .as_raw();

    match raw_handle {
        raw_window_handle::RawWindowHandle::Win32(handle) => {
            Ok(SoloWebviewParent { hwnd: handle.hwnd })
        }
        handle => Err(format!("Unsupported window handle for WebView: {handle:?}")),
    }
}

impl SoloWorkspacePanel {
    fn solo_display_count(&self) -> usize {
        self.messages.len()
    }

    pub(super) fn reset_solo_chat_list_state(&mut self) {
        self.solo_chat_list_state = gpui::ListState::new(
            self.solo_display_count(),
            gpui::ListAlignment::Bottom,
            px(200.),
        );
    }

    pub(super) fn submit_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.solo_is_thinking {
            return;
        }

        let raw_prompt = self.prompt_input.read(cx).text().to_string();
        let mut prompt = raw_prompt.trim().to_string();
        if prompt.is_empty() && self.solo_attachments.is_empty() {
            return;
        }
        if prompt.is_empty() {
            prompt = "Please analyze the attached file(s).".to_string();
        }

        let attachments = std::mem::take(&mut self.solo_attachments);
        let selected_model = self
            .solo_model_select
            .read(cx)
            .selected_value()
            .map(ToString::to_string)
            .unwrap_or_else(|| "Auto".to_string());
        let speed = self.solo_effort_label(cx).to_string();
        let tools = self.enabled_solo_tools(cx);
        let selected_mcp_tools = self
            .available_solo_mcp_tools(cx)
            .into_iter()
            .filter(|(tool, _)| self.solo_selected_mcp_tool_ids.contains(&tool.id))
            .map(|(tool, _)| tool)
            .collect::<Vec<_>>();
        let runtime = crate::AppState::global(cx).tokio_runtime.clone();
        let db = crate::AppState::global(cx).db.clone();
        let mcp_servers = db.list_mcp_servers().unwrap_or_default();
        let provider_configs = Self::resolve_solo_providers(&selected_model, cx);
        let model = selected_model.clone();

        self.messages.push(SoloMessage {
            role: "user",
            content: prompt.clone(),
            attachments: attachments.clone(),
            model: Some(model.clone()),
            speed: Some(speed.clone()),
            tools: tools.clone(),
            activities: Vec::new(),
        });
        let mut system_prompt = self.solo_system_prompt(&speed, &tools);
        system_prompt.push_str(&Self::solo_mcp_tool_instructions(&selected_mcp_tools));
        let mut history = vec![crate::providers::ChatMessage::new_text(
            "system",
            system_prompt,
        )];
        history.extend(
            self.messages
                .iter()
                .rev()
                .filter(|message| {
                    !(message.role == "assistant"
                        && message.content.starts_with("Request prepared with "))
                })
                .take(24)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|message| {
                    crate::providers::ChatMessage::new_text(message.role, message.content.clone())
                }),
        );

        let assistant_index = self.messages.len();
        self.messages.push(SoloMessage {
            role: "assistant",
            content: String::new(),
            attachments: Vec::new(),
            model: Some(model.clone()),
            speed: Some(speed.clone()),
            tools: tools.clone(),
            activities: Vec::new(),
        });
        self.solo_is_thinking = true;
        self.solo_response_generation = self.solo_response_generation.wrapping_add(1);
        let response_generation = self.solo_response_generation;

        self.prompt_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        self.reset_solo_chat_list_state();
        self.upsert_active_conversation(cx);
        cx.notify();

        let (stream_tx, mut stream_rx) = tokio::sync::mpsc::unbounded_channel();
        let view = cx.entity().clone();
        cx.spawn(async move |_, cx| {
            while let Some(event) = stream_rx.recv().await {
                let finished = matches!(&event, SoloStreamEvent::Finished(_));
                let _ = cx.update(|cx| {
                    let _ = view.update(cx, |this, cx| {
                        if !this.solo_is_thinking
                            || this.solo_response_generation != response_generation
                            || this
                                .messages
                                .get(assistant_index)
                                .map(|message| message.role)
                                != Some("assistant")
                        {
                            return;
                        }

                        match event {
                            SoloStreamEvent::ProviderSelected(model) => {
                                if let Some(message) = this.messages.get_mut(assistant_index) {
                                    message.model = Some(model);
                                }
                            }
                            SoloStreamEvent::Activity(activity) => {
                                if let Some(message) = this.messages.get_mut(assistant_index) {
                                    if let Some(existing) = message
                                        .activities
                                        .iter_mut()
                                        .find(|existing| existing.id == activity.id)
                                    {
                                        *existing = activity;
                                    } else {
                                        message.activities.push(activity);
                                    }
                                }
                            }
                            SoloStreamEvent::Delta(delta) => {
                                if let Some(message) = this.messages.get_mut(assistant_index) {
                                    message.content.push_str(&delta);
                                }
                            }
                            SoloStreamEvent::Finished(result) => {
                                this.solo_is_thinking = false;
                                if let Err(error) = result {
                                    if let Some(message) = this.messages.get_mut(assistant_index) {
                                        if message.content.trim().is_empty() {
                                            message.content = error;
                                        } else {
                                            message.content.push_str(&format!(
                                                "\n\n> Response interrupted: {}",
                                                error
                                            ));
                                        }
                                    }
                                }
                                this.upsert_active_conversation(cx);
                            }
                        }
                        this.reset_solo_chat_list_state();
                        cx.notify();
                    });
                });
                if finished {
                    break;
                }
            }
        })
        .detach();

        let _stream_task = runtime.spawn(async move {
            if !attachments.is_empty() {
                let _ = stream_tx.send(SoloStreamEvent::Activity(SoloActivity {
                    id: "attachments".to_string(),
                    kind: SoloActivityKind::File,
                    label: format!("Reading {} attachment(s)", attachments.len()),
                    detail: Some(attachments.join("\n")),
                    status: SoloActivityStatus::Running,
                }));
            }
            let attachment_context =
                crate::application::file_intelligence::build_chat_context(
                    &prompt,
                    &attachments,
                    None,
                    crate::application::file_intelligence::AnalyzeOptions {
                        max_text_chars: 12_000,
                        max_total_bytes: 20 * 1024 * 1024,
                        include_external_media_analysis: false,
                    },
                )
                .await;
            if !attachments.is_empty() {
                let _ = stream_tx.send(SoloStreamEvent::Activity(SoloActivity {
                    id: "attachments".to_string(),
                    kind: SoloActivityKind::File,
                    label: format!("Read {} attachment(s)", attachments.len()),
                    detail: Some(attachments.join("\n")),
                    status: SoloActivityStatus::Completed,
                }));
            }
            if !attachment_context.is_empty() {
                if let Some(last_user_message) = history
                    .iter_mut()
                    .rev()
                    .find(|message| message.role.as_ref() == "user")
                {
                    let content = format!("{}{}", last_user_message.content, attachment_context);
                    *last_user_message =
                        crate::providers::ChatMessage::new_text("user", content);
                }
            }

            let is_auto = selected_model == "Auto";
            let result = if provider_configs.is_empty() {
                Err(if is_auto {
                    "No available AI provider is configured. Add or enable a provider in Settings before using Solo chat."
                        .to_string()
                } else {
                    format!(
                        "The selected provider '{}' is no longer available. Refresh the Solo workspace or choose another model.",
                        selected_model
                    )
                })
            } else {
                let mut failures = Vec::new();
                let mut result = None;

                for provider_config in provider_configs {
                    let provider_label = format!(
                        "{} / {}",
                        provider_config.provider_name, provider_config.model
                    );
                    if stream_tx
                        .send(SoloStreamEvent::ProviderSelected(provider_label.clone()))
                        .is_err()
                    {
                        return;
                    }

                    let Some(adapter) =
                        crate::application::services::provider_factory::create_adapter(
                            &provider_config,
                        )
                    else {
                        let error = format!(
                            "Unable to initialize provider adapter for {}. Check its adapter type and configuration.",
                            provider_label
                        );
                        if !is_auto {
                            result = Some(Err(error));
                            break;
                        }
                        failures.push(error);
                        continue;
                    };

                    if !selected_mcp_tools.is_empty() {
                        match Self::run_solo_mcp_loop(
                            adapter,
                            history.clone(),
                            &selected_mcp_tools,
                            &mcp_servers,
                            &stream_tx,
                        )
                        .await
                        {
                            Ok(()) => {
                                let _ = db.set_setting(
                                    SOLO_AUTO_PROVIDER_SETTING,
                                    provider_config.id.as_str(),
                                );
                                result = Some(Ok(()));
                                break;
                            }
                            Err(error) => {
                                let error = format!(
                                    "Provider request failed for {}: {}",
                                    provider_label, error
                                );
                                if !is_auto {
                                    result = Some(Err(error));
                                    break;
                                }
                                failures.push(error);
                                continue;
                            }
                        }
                    }

                    match adapter.send_message_stream(history.clone()).await {
                        Ok(mut stream) => {
                            let mut received_text = false;
                            let mut stream_error = None;
                            while let Some(chunk) = stream.next().await {
                                match chunk {
                                    Ok(crate::providers::StreamChunk::Text(text)) => {
                                        if !text.is_empty() {
                                            received_text = true;
                                            if stream_tx
                                                .send(SoloStreamEvent::Delta(text))
                                                .is_err()
                                            {
                                                return;
                                            }
                                        }
                                    }
                                    Ok(crate::providers::StreamChunk::Done(_)) => break,
                                    Err(error) => {
                                        stream_error = Some(format!(
                                            "Provider request failed for {}: {}",
                                            provider_label, error
                                        ));
                                        break;
                                    }
                                }
                            }

                            if received_text {
                                result = Some(match stream_error {
                                    Some(error) => Err(error),
                                    None => {
                                        let _ = db.set_setting(
                                            SOLO_AUTO_PROVIDER_SETTING,
                                            provider_config.id.as_str(),
                                        );
                                        Ok(())
                                    }
                                });
                                break;
                            }

                            let error = stream_error.unwrap_or_else(|| {
                                format!(
                                    "{} returned an empty response. Verify the model id and provider protocol.",
                                    provider_label
                                )
                            });
                            if !is_auto {
                                result = Some(Err(error));
                                break;
                            }
                            failures.push(error);
                        }
                        Err(error) => {
                            let error = format!(
                                "Provider request failed for {}: {}",
                                provider_label, error
                            );
                            if !is_auto {
                                result = Some(Err(error));
                                break;
                            }
                            failures.push(error);
                        }
                    }
                }

                result.unwrap_or_else(|| {
                    let last_error = failures
                        .last()
                        .map(String::as_str)
                        .unwrap_or("No provider accepted the request.");
                    Err(format!(
                        "Auto tried {} available provider(s), but none returned a response. Last error: {}",
                        failures.len(),
                        last_error
                    ))
                })
            };
            let _ = stream_tx.send(SoloStreamEvent::Finished(result));
        });
    }

    fn resolve_solo_providers(selected_model: &str, cx: &gpui::App) -> Vec<crate::db::Provider> {
        let db = &crate::AppState::global(cx).db;
        let mut providers = db.list_providers().unwrap_or_default();
        if selected_model != "Auto" {
            return providers
                .into_iter()
                .filter(|provider| {
                    format!("{} / {}", provider.provider_name, provider.model) == selected_model
                })
                .collect();
        }

        let preferred_provider_id = db.get_setting(SOLO_AUTO_PROVIDER_SETTING).ok().flatten();
        Self::order_solo_auto_providers(&mut providers, preferred_provider_id.as_deref());
        providers
    }

    fn order_solo_auto_providers(
        providers: &mut Vec<crate::db::Provider>,
        preferred_provider_id: Option<&str>,
    ) {
        providers.retain(|provider| Self::solo_auto_status_rank(&provider.status).is_some());
        providers.sort_by(|left, right| {
            let left_preferred = preferred_provider_id == Some(left.id.as_str());
            let right_preferred = preferred_provider_id == Some(right.id.as_str());
            (!left_preferred)
                .cmp(&(!right_preferred))
                .then_with(|| {
                    Self::solo_auto_status_rank(&left.status)
                        .cmp(&Self::solo_auto_status_rank(&right.status))
                })
                .then_with(|| {
                    format!("{} / {}", left.provider_name, left.model)
                        .to_lowercase()
                        .cmp(&format!("{} / {}", right.provider_name, right.model).to_lowercase())
                })
        });
    }

    fn solo_auto_status_rank(status: &str) -> Option<u8> {
        match status.trim().to_ascii_lowercase().as_str() {
            "active" => Some(0),
            "healthy" | "ready" | "online" => Some(1),
            "available" => Some(2),
            _ => None,
        }
    }

    fn solo_mcp_tool_instructions(
        tools: &[crate::infrastructure::mcp::registry::McpTool],
    ) -> String {
        if tools.is_empty() {
            return String::new();
        }

        let schemas = tools
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": serde_json::from_str::<serde_json::Value>(&tool.input_schema)
                        .unwrap_or_else(|_| serde_json::json!({})),
                })
            })
            .collect::<Vec<_>>();
        format!(
            "\n\nThe following MCP tools are enabled for this request:\n{}\n\nWhen a tool is necessary, return only one or more JSON objects wrapped individually in <tool_call> tags. Use this exact shape: <tool_call>{{\"id\":\"call_1\",\"name\":\"tool_name\",\"arguments\":{{}}}}</tool_call>. Do not claim a tool was used until its result is provided. After receiving <tool_result>, continue solving the request and return a normal final answer.",
            serde_json::to_string_pretty(&schemas).unwrap_or_default()
        )
    }

    fn parse_solo_tool_calls(raw: &str) -> Vec<SoloToolCall> {
        let mut calls = Vec::new();
        let mut offset = 0usize;
        while let Some(start_relative) = raw[offset..].find("<tool_call>") {
            let start = offset + start_relative + "<tool_call>".len();
            let Some(end_relative) = raw[start..].find("</tool_call>") else {
                break;
            };
            let end = start + end_relative;
            if let Ok(mut call) = serde_json::from_str::<SoloToolCall>(raw[start..end].trim()) {
                if call.id.trim().is_empty() {
                    call.id = format!("call_{}", uuid::Uuid::new_v4());
                }
                if let Some(arguments) = call.arguments.as_str() {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(arguments) {
                        call.arguments = parsed;
                    }
                }
                calls.push(call);
            }
            offset = end + "</tool_call>".len();
        }
        calls
    }

    fn solo_tool_activity_kind(tool_name: &str) -> SoloActivityKind {
        let name = tool_name.to_ascii_lowercase();
        if name.contains("search") || name.contains("browse") || name.contains("web") {
            SoloActivityKind::Search
        } else if name.contains("command") || name.contains("shell") || name.contains("terminal") {
            SoloActivityKind::Command
        } else if name.contains("file")
            || name.contains("read")
            || name.contains("write")
            || name.contains("edit")
        {
            SoloActivityKind::File
        } else {
            SoloActivityKind::Tool
        }
    }

    fn solo_tool_activity_label(
        kind: SoloActivityKind,
        tool_name: &str,
        status: SoloActivityStatus,
    ) -> String {
        match (kind, status) {
            (SoloActivityKind::Search, SoloActivityStatus::Running) => {
                format!("Searching with {tool_name}")
            }
            (SoloActivityKind::Search, SoloActivityStatus::Completed) => {
                format!("Searched with {tool_name}")
            }
            (SoloActivityKind::Command, SoloActivityStatus::Running) => {
                format!("Running {tool_name}")
            }
            (SoloActivityKind::Command, SoloActivityStatus::Completed) => {
                format!("Ran {tool_name}")
            }
            (SoloActivityKind::File, SoloActivityStatus::Running) => {
                format!("Using {tool_name}")
            }
            (SoloActivityKind::File, SoloActivityStatus::Completed) => {
                format!("Used {tool_name}")
            }
            (_, SoloActivityStatus::Running) => format!("Calling {tool_name}"),
            (_, SoloActivityStatus::Completed) => format!("Called {tool_name}"),
            _ => format!("{tool_name} failed"),
        }
    }

    fn solo_activity_detail(value: &str) -> String {
        truncate_solo_chars(value, 2_000, "\n[Detail truncated]")
            .unwrap_or_else(|| value.to_string())
    }

    async fn run_solo_mcp_loop(
        adapter: std::sync::Arc<dyn crate::providers::BaseProviderAdapter>,
        mut history: Vec<crate::providers::ChatMessage>,
        tools: &[crate::infrastructure::mcp::registry::McpTool],
        servers: &[crate::infrastructure::mcp::registry::McpServerRecord],
        stream_tx: &tokio::sync::mpsc::UnboundedSender<SoloStreamEvent>,
    ) -> Result<(), String> {
        const MAX_TOOL_ITERATIONS: usize = 4;

        for iteration in 0..=MAX_TOOL_ITERATIONS {
            let mut stream = adapter
                .send_message_stream(history.clone())
                .await
                .map_err(|error| format!("Provider request failed: {error}"))?;
            let mut response_text = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(crate::providers::StreamChunk::Text(text)) => {
                        response_text.push_str(&text);
                    }
                    Ok(crate::providers::StreamChunk::Done(_)) => break,
                    Err(error) => return Err(format!("Provider stream failed: {error}")),
                }
            }
            if response_text.trim().is_empty() {
                return Err("Provider returned an empty response.".to_string());
            }

            let tool_calls = Self::parse_solo_tool_calls(&response_text);
            if tool_calls.is_empty() {
                stream_tx
                    .send(SoloStreamEvent::Delta(response_text))
                    .map_err(|_| "Solo response was closed before completion.".to_string())?;
                return Ok(());
            }
            if iteration == MAX_TOOL_ITERATIONS {
                return Err(format!(
                    "Stopped after {MAX_TOOL_ITERATIONS} tool rounds to prevent an infinite loop."
                ));
            }

            history.push(crate::providers::ChatMessage::new_text(
                "assistant",
                response_text,
            ));
            for call in tool_calls {
                let activity_id = format!("tool-{iteration}-{}", call.id);
                let kind = Self::solo_tool_activity_kind(&call.name);
                let arguments = serde_json::to_string_pretty(&call.arguments)
                    .unwrap_or_else(|_| call.arguments.to_string());
                stream_tx
                    .send(SoloStreamEvent::Activity(SoloActivity {
                        id: activity_id.clone(),
                        kind,
                        label: Self::solo_tool_activity_label(
                            kind,
                            &call.name,
                            SoloActivityStatus::Running,
                        ),
                        detail: Some(Self::solo_activity_detail(&arguments)),
                        status: SoloActivityStatus::Running,
                    }))
                    .map_err(|_| "Solo response was closed before tool execution.".to_string())?;

                let tool_result = if let Some(tool) =
                    tools.iter().find(|tool| tool.name == call.name)
                {
                    if tool.server_id.as_deref() == Some("builtin-team-tools") {
                        Err(
                            "Built-in team tools require a Team workspace and cannot run in Solo."
                                .to_string(),
                        )
                    } else if let Some(server) = tool.server_id.as_deref().and_then(|server_id| {
                        servers
                            .iter()
                            .find(|server| server.id == server_id && server.is_enabled)
                    }) {
                        crate::infrastructure::mcp::server::McpServer::invoke_tool(
                            server,
                            tool,
                            call.arguments.clone(),
                        )
                        .await
                        .map(|value| value.to_string())
                        .map_err(|error| error.to_string())
                    } else {
                        Err("The MCP server is unavailable or disabled.".to_string())
                    }
                } else {
                    Err(format!(
                        "Tool '{}' was not enabled for this request.",
                        call.name
                    ))
                };

                let (status, result_text) = match tool_result {
                    Ok(result) => (SoloActivityStatus::Completed, result),
                    Err(error) => (SoloActivityStatus::Failed, error),
                };
                stream_tx
                    .send(SoloStreamEvent::Activity(SoloActivity {
                        id: activity_id,
                        kind,
                        label: Self::solo_tool_activity_label(kind, &call.name, status),
                        detail: Some(Self::solo_activity_detail(&result_text)),
                        status,
                    }))
                    .map_err(|_| "Solo response was closed after tool execution.".to_string())?;

                let result_for_model =
                    truncate_solo_chars(&result_text, 12_000, "\n[Tool result truncated]")
                        .unwrap_or(result_text);
                history.push(crate::providers::ChatMessage::new_text(
                    "user",
                    format!(
                        "<tool_result id=\"{}\" name=\"{}\">\n{}\n</tool_result>",
                        call.id, call.name, result_for_model
                    ),
                ));
            }
        }

        Err("Tool loop ended unexpectedly.".to_string())
    }

    fn solo_system_prompt(&self, speed: &str, tools: &[String]) -> String {
        let effort_instruction = match speed {
            "Light" => "Answer briefly and directly, using minimal reasoning.",
            "Medium" => "Answer efficiently and focus on the user's immediate request.",
            "Extra High" => {
                "Reason deeply, verify important assumptions, and cover meaningful edge cases."
            }
            "Ultra" => {
                "Reason carefully, check assumptions, and provide the strongest complete answer you can."
            }
            _ => "Think through the request carefully and give a clear, useful answer.",
        };
        let skill_instructions = self
            .solo_selected_skill_ids
            .iter()
            .filter_map(|skill_id| {
                self.solo_skills
                    .iter()
                    .find(|skill| &skill.id == skill_id)
                    .map(|skill| format!("- {}: {}", skill.name, skill.instructions))
            })
            .collect::<Vec<_>>()
            .join("\n");
        let capability_context = if tools.is_empty() {
            "No optional capabilities are selected.".to_string()
        } else {
            format!("Selected capabilities: {}.", tools.join(", "))
        };
        let project_context = self
            .active_project_id
            .as_deref()
            .and_then(|project_id| {
                self.projects
                    .iter()
                    .find(|project| project.id == project_id)
            })
            .map(|project| format!("Active project: {}.", project.name))
            .unwrap_or_else(|| "No project is currently selected.".to_string());

        format!(
            "You are Personal AI in AgentForge Solo Mode. Respond directly to the user's latest message and use prior conversation context when relevant. Do not echo or merely restate the user's message. Never claim that a tool, MCP capability, file change, or remote action was executed unless the conversation contains an actual result.\n\n{}\n{}\n{}\nEffort profile: {}. {}{}",
            project_context,
            capability_context,
            effort_instruction,
            speed,
            if skill_instructions.is_empty() {
                ""
            } else {
                "\nApplied skill instructions:\n"
            },
            skill_instructions
        )
    }

    fn enabled_solo_tools(&self, cx: &gpui::App) -> Vec<String> {
        let mut tools = Vec::new();
        if self.solo_build_enabled {
            tools.push(SoloTool::Build.label().to_string());
        }
        for skill_id in &self.solo_selected_skill_ids {
            if let Some(skill) = self.solo_skills.iter().find(|skill| &skill.id == skill_id) {
                tools.push(format!("Skill: {}", skill.name));
            }
        }
        let available_mcp_tools = self.available_solo_mcp_tools(cx);
        for tool_id in &self.solo_selected_mcp_tool_ids {
            if let Some((tool, _)) = available_mcp_tools
                .iter()
                .find(|(tool, _)| &tool.id == tool_id)
            {
                tools.push(format!("MCP: {}", tool.name));
            }
        }
        tools
    }

    fn solo_effort_label(&self, cx: &gpui::App) -> &'static str {
        match self.solo_effort_slider.read(cx).value().end().round() as usize {
            0 => "Light",
            1 => "Medium",
            2 => "High",
            3 => "Extra High",
            _ => "Ultra",
        }
    }

    fn toggle_solo_tool(&mut self, tool: SoloTool, cx: &mut Context<Self>) {
        match tool {
            SoloTool::Build => self.solo_build_enabled = !self.solo_build_enabled,
            SoloTool::Skills => {}
        }
        cx.notify();
    }

    fn available_solo_mcp_tools(
        &self,
        cx: &gpui::App,
    ) -> Vec<(crate::infrastructure::mcp::registry::McpTool, String)> {
        let db = &crate::AppState::global(cx).db;
        let servers = db.list_mcp_servers().unwrap_or_default();
        let enabled_servers: std::collections::HashMap<_, _> = servers
            .into_iter()
            .filter(|server| server.is_enabled)
            .map(|server| (server.id, server.name))
            .collect();
        let mut tools: Vec<_> = db
            .list_mcp_tools()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tool| {
                if !tool.is_active {
                    return None;
                }
                let server_name = tool
                    .server_id
                    .as_ref()
                    .and_then(|server_id| enabled_servers.get(server_id))?
                    .clone();
                Some((tool, server_name))
            })
            .collect();
        tools.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| left.0.name.cmp(&right.0.name))
        });
        tools
    }

    fn toggle_solo_mcp_tool(&mut self, tool_id: &str, cx: &mut Context<Self>) {
        if let Some(index) = self
            .solo_selected_mcp_tool_ids
            .iter()
            .position(|selected| selected == tool_id)
        {
            self.solo_selected_mcp_tool_ids.remove(index);
        } else {
            self.solo_selected_mcp_tool_ids.push(tool_id.to_string());
        }
        cx.notify();
    }

    fn toggle_solo_skill(&mut self, skill_id: &str, cx: &mut Context<Self>) {
        if let Some(index) = self
            .solo_selected_skill_ids
            .iter()
            .position(|selected| selected == skill_id)
        {
            self.solo_selected_skill_ids.remove(index);
        } else {
            self.solo_selected_skill_ids.push(skill_id.to_string());
        }
        cx.notify();
    }

    fn solo_mention_options(&self) -> Vec<SoloMentionOption> {
        let mut options = vec![
            SoloMentionOption {
                id: "current-project".to_string(),
                label: "Current project".to_string(),
                detail: "Project context".to_string(),
                icon: IconName::Folder,
            },
            SoloMentionOption {
                id: "memory".to_string(),
                label: "Memory".to_string(),
                detail: "Personal memory".to_string(),
                icon: IconName::BookOpen,
            },
            SoloMentionOption {
                id: "webview".to_string(),
                label: "Webview".to_string(),
                detail: "Open browser context".to_string(),
                icon: IconName::Globe,
            },
        ];

        options.extend(self.projects.iter().map(|project| SoloMentionOption {
            id: project.id.clone(),
            label: project.name.clone(),
            detail: "Project".to_string(),
            icon: IconName::Folder,
        }));
        options.extend(self.solo_attachments.iter().map(|path| {
            let file_name = Self::attachment_name(path);
            SoloMentionOption {
                id: Self::mention_slug(&file_name),
                label: file_name,
                detail: "Attached file".to_string(),
                icon: IconName::File,
            }
        }));
        options.extend(self.solo_skills.iter().map(|skill| SoloMentionOption {
            id: skill.id.clone(),
            label: skill.name.clone(),
            detail: format!("{} skill", skill.category),
            icon: IconName::Settings2,
        }));
        options
    }

    fn mention_slug(value: &str) -> String {
        let mut slug = String::with_capacity(value.len());
        let mut last_was_dash = false;
        for character in value.chars() {
            if character.is_alphanumeric() || matches!(character, '.' | '_' | '-') {
                slug.extend(character.to_lowercase());
                last_was_dash = false;
            } else if !last_was_dash {
                slug.push('-');
                last_was_dash = true;
            }
        }
        slug.trim_matches('-').to_string()
    }

    pub(super) fn sync_solo_mention_query(&mut self, cx: &mut Context<Self>) {
        let input = self.prompt_input.read(cx);
        let text = input.text().to_string();
        let cursor = input.cursor().min(text.len());
        let prefix = &text[..cursor];
        let start = prefix
            .char_indices()
            .rev()
            .find_map(|(index, character)| {
                character
                    .is_whitespace()
                    .then_some(index + character.len_utf8())
            })
            .unwrap_or(0);
        let token = &prefix[start..];

        if let Some(query) = token.strip_prefix('@') {
            if !query.chars().any(char::is_whitespace) {
                let query = query.to_ascii_lowercase();
                if self.solo_mention_query.as_deref() != Some(query.as_str()) {
                    self.solo_mention_selection_index = 0;
                }
                self.solo_mention_query = Some(query);
                self.solo_mention_start = Some(start);
                cx.notify();
                return;
            }
        }

        if self.solo_mention_query.take().is_some() {
            self.solo_mention_start = None;
            self.solo_mention_selection_index = 0;
            cx.notify();
        }
    }

    fn open_solo_mentions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt_input.update(cx, |state, cx| {
            state.insert("@", window, cx);
        });
        self.sync_solo_mention_query(cx);
    }

    fn insert_solo_mention(
        &mut self,
        mention_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = self.prompt_input.read(cx);
        let text = input.text().to_string();
        let cursor = input.cursor().min(text.len());
        let start = self.solo_mention_start.unwrap_or(cursor).min(cursor);
        let mention = format!("@{} ", mention_id);
        let mut value = String::with_capacity(text.len() + mention.len());
        value.push_str(&text[..start]);
        value.push_str(&mention);
        value.push_str(&text[cursor..]);
        let new_cursor = start + mention.len();

        self.prompt_input.update(cx, |state, cx| {
            state.set_value(value, window, cx);
            let position = state.text().offset_to_position(new_cursor);
            state.set_cursor_position(position, window, cx);
        });
        self.solo_mention_query = None;
        self.solo_mention_start = None;
        self.solo_mention_selection_index = 0;
        cx.notify();
    }

    fn filtered_solo_mention_options(&self) -> Vec<SoloMentionOption> {
        let query = self
            .solo_mention_query
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        self.solo_mention_options()
            .into_iter()
            .filter(|option| {
                query.is_empty()
                    || option.id.to_ascii_lowercase().contains(&query)
                    || option.label.to_ascii_lowercase().contains(&query)
                    || option.detail.to_ascii_lowercase().contains(&query)
            })
            .take(8)
            .collect()
    }

    fn move_solo_mention_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.filtered_solo_mention_options().len();
        if count == 0 {
            self.solo_mention_selection_index = 0;
            return;
        }
        self.solo_mention_selection_index = (self.solo_mention_selection_index as isize + delta)
            .rem_euclid(count as isize) as usize;
        cx.notify();
    }

    fn confirm_solo_mention(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.solo_mention_query.is_none() {
            return false;
        }
        let options = self.filtered_solo_mention_options();
        let Some(option) = options.get(
            self.solo_mention_selection_index
                .min(options.len().saturating_sub(1)),
        ) else {
            return true;
        };
        let mention_id = option.id.clone();
        self.insert_solo_mention(&mention_id, window, cx);
        true
    }

    fn pick_solo_attachments(&mut self, images_only: bool, cx: &mut Context<Self>) {
        let view = cx.entity().clone();
        cx.spawn(async move |_, cx| {
            let mut dialog = rfd::AsyncFileDialog::new().set_title(if images_only {
                "Add images"
            } else {
                "Add files"
            });
            if images_only {
                dialog = dialog.add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"]);
            }

            if let Some(files) = dialog.pick_files().await {
                let paths: Vec<String> = files
                    .into_iter()
                    .map(|file| file.path().to_string_lossy().to_string())
                    .collect();
                let _ = cx.update(|cx| {
                    let _ = view.update(cx, |this, cx| {
                        for path in paths {
                            if this.solo_attachments.len() >= 10 {
                                break;
                            }
                            if !this.solo_attachments.contains(&path) {
                                this.solo_attachments.push(path);
                            }
                        }
                        cx.notify();
                    });
                });
            }
        })
        .detach();
    }

    fn remove_solo_attachment(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.solo_attachments.len() {
            self.solo_attachments.remove(index);
            cx.notify();
        }
    }

    fn resize_webview_at(&mut self, mouse_x: f32, window: &Window, cx: &mut Context<Self>) {
        if !self.solo_webview_resizing {
            return;
        }

        let window_width: f32 = window.bounds().size.width.into();
        let max_width = (window_width - 940.0).max(420.0);
        self.solo_webview_width = (window_width - mouse_x).clamp(420.0, max_width);
        cx.notify();
    }

    fn on_prompt_confirm(
        &mut self,
        _: &crate::ChatComposerConfirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.confirm_solo_mention(window, cx) {
            return;
        }
        self.submit_prompt(window, cx);
    }

    fn on_solo_mention_previous(
        &mut self,
        _: &crate::MentionPrevious,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_solo_mention_selection(-1, cx);
    }

    fn on_solo_mention_next(
        &mut self,
        _: &crate::MentionNext,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_solo_mention_selection(1, cx);
    }

    fn open_webview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.upsert_active_conversation(cx);
        self.active_section = SoloSection::Chat;
        self.solo_webview_open = true;
        self.history_sidebar_open = false;

        if self.solo_webview.is_none() {
            let url =
                Self::normalize_webview_url(&self.webview_url_input.read(cx).text().to_string());
            self.start_webview_creation(url, window, cx);
        }
        cx.notify();
    }

    fn close_webview(&mut self, cx: &mut Context<Self>) {
        self.solo_webview_open = false;
        self.solo_webview_resizing = false;
        self.solo_webview = None;
        self.solo_webview_error = None;
        self.solo_webview_generation = self.solo_webview_generation.wrapping_add(1);
        cx.notify();
    }

    fn normalize_webview_url(raw_url: &str) -> String {
        let url = raw_url.trim();
        if url.is_empty() {
            return "about:blank".to_string();
        }

        if url.starts_with("about:") || url.contains("://") {
            url.to_string()
        } else {
            format!("https://{}", url)
        }
    }

    fn normalize_agent_link_for_webview(raw_url: &str) -> Option<String> {
        let url = raw_url.trim();
        if url.starts_with("https://") || url.starts_with("http://") {
            return Some(url.to_string());
        }
        if url.starts_with("//") {
            return Some(format!("https:{url}"));
        }
        if url.starts_with("localhost:") || url.starts_with("127.0.0.1:") {
            return Some(format!("http://{url}"));
        }
        if !url.is_empty()
            && !url.contains(':')
            && !url.starts_with('/')
            && !url.starts_with('#')
            && url.contains('.')
            && !url.chars().any(char::is_whitespace)
        {
            return Some(format!("https://{url}"));
        }
        None
    }

    fn open_agent_link_in_webview(
        &mut self,
        raw_url: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(url) = Self::normalize_agent_link_for_webview(raw_url) else {
            return false;
        };

        self.upsert_active_conversation(cx);
        self.active_section = SoloSection::Chat;
        self.solo_webview_open = true;
        self.history_sidebar_open = false;
        self.webview_url_input.update(cx, |state, cx| {
            state.set_value(url, window, cx);
        });
        self.commit_webview_url(window, cx);
        true
    }

    fn commit_webview_url(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = Self::normalize_webview_url(&self.webview_url_input.read(cx).text().to_string());
        self.webview_url_input.update(cx, |state, cx| {
            state.set_value(url.clone(), window, cx);
        });

        if let Some(webview) = self.solo_webview.clone() {
            let result = webview.update(cx, |webview, cx| {
                let load_result = webview
                    .raw()
                    .load_url(&url)
                    .map_err(|error| error.to_string());
                cx.notify();
                load_result
            });
            match result {
                Ok(()) => self.solo_webview_error = None,
                Err(error) => self.solo_webview_error = Some(error),
            }
        } else {
            self.solo_webview_open = true;
            self.start_webview_creation(url, window, cx);
        }
        cx.notify();
    }

    #[cfg(target_os = "windows")]
    fn start_webview_creation(&mut self, url: String, window: &mut Window, cx: &mut Context<Self>) {
        let parent = match webview_parent_from_window(window) {
            Ok(parent) => parent,
            Err(error) => {
                self.solo_webview = None;
                self.solo_webview_error = Some(error);
                return;
            }
        };

        self.solo_webview_generation = self.solo_webview_generation.wrapping_add(1);
        let generation = self.solo_webview_generation;
        self.solo_webview = None;
        self.solo_webview_error = Some("Loading WebView...".to_string());

        let view = cx.entity().clone();
        let window_handle = window.window_handle();
        cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1))
                .await;

            let mut result = Some(
                gpui_component::wry::WebViewBuilder::new()
                    .with_url(&url)
                    .build_as_child(&parent)
                    .map_err(|error| error.to_string()),
            );

            let _ = cx.update(|cx| {
                let _ = window_handle.update(cx, |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        if !this.solo_webview_open || this.solo_webview_generation != generation {
                            return;
                        }

                        match result
                            .take()
                            .expect("webview creation result should be available")
                        {
                            Ok(webview) => {
                                let webview = cx.new(|cx| {
                                    gpui_component::webview::WebView::new(webview, window, cx)
                                });
                                this.solo_webview = Some(webview);
                                this.solo_webview_error = None;
                            }
                            Err(error) => {
                                this.solo_webview = None;
                                this.solo_webview_error = Some(error);
                            }
                        }
                        cx.notify();
                    });
                });
            });
        })
        .detach();
    }

    #[cfg(not(target_os = "windows"))]
    fn start_webview_creation(
        &mut self,
        _url: String,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.solo_webview = None;
        self.solo_webview_error =
            Some("Embedded WebView is only wired for Windows in this build.".to_string());
    }

    fn prefill_prompt(
        &mut self,
        prompt: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.prompt_input.update(cx, |state, cx| {
            state.set_value(prompt, window, cx);
        });
        cx.notify();
    }

    pub(super) fn prefill_personal_prompt(
        &mut self,
        prompt: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_section = SoloSection::Chat;
        self.active_project_id = None;
        self.history_sidebar_open = true;
        self.prefill_prompt(prompt, window, cx);
    }

    fn conversation_title(messages: &[SoloMessage]) -> String {
        messages
            .iter()
            .find(|message| message.role == "user")
            .map(|message| {
                let mut title = message.content.trim().replace('\n', " ");
                if title.chars().count() > 42 {
                    title = title.chars().take(42).collect::<String>();
                    title.push_str("...");
                }
                title
            })
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| "Untitled chat".to_string())
    }

    fn persist_solo_conversation(&self, id: usize, cx: &Context<Self>) {
        let Some(conversation) = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
        else {
            return;
        };
        let db = crate::AppState::global(cx).db.clone();
        if let Err(error) = db.upsert_solo_conversation(
            id as i64,
            conversation.project_id.as_deref(),
            &conversation.title,
        ) {
            tracing::error!("failed to persist Solo conversation: {error}");
            return;
        }
        let messages = conversation
            .messages
            .iter()
            .map(|message| {
                let metadata = SoloMessageMetadata {
                    attachments: message.attachments.clone(),
                    model: message.model.clone(),
                    speed: message.speed.clone(),
                    tools: message.tools.clone(),
                    activities: message.activities.clone(),
                };
                crate::core::models::SoloMessageRecord {
                    role: message.role.to_string(),
                    content: message.content.clone(),
                    metadata: serde_json::to_string(&metadata).ok(),
                }
            })
            .collect::<Vec<_>>();
        if let Err(error) = db.replace_solo_messages(id as i64, &messages) {
            tracing::error!("failed to persist Solo messages: {error}");
        }
    }

    pub(super) fn upsert_active_conversation(&mut self, cx: &mut Context<Self>) {
        if self.messages.is_empty() {
            return;
        }

        let title = Self::conversation_title(&self.messages);
        if let Some(id) = self.active_conversation_id {
            let mut updated = false;
            if let Some(conversation) = self
                .conversations
                .iter_mut()
                .find(|conversation| conversation.id == id)
            {
                conversation.title = title.clone();
                conversation.messages = self.messages.clone();
                conversation.project_id = self.active_project_id.clone();
                updated = true;
            }
            if updated {
                self.persist_solo_conversation(id, cx);
                return;
            }
        }

        let id = self.next_conversation_id;
        self.next_conversation_id += 1;
        self.active_conversation_id = Some(id);
        self.conversations.insert(
            0,
            super::model::SoloConversation {
                id,
                project_id: self.active_project_id.clone(),
                title,
                messages: self.messages.clone(),
            },
        );
        self.persist_solo_conversation(id, cx);
    }

    pub(super) fn start_new_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.solo_is_thinking {
            self.solo_is_thinking = false;
            self.solo_response_generation = self.solo_response_generation.wrapping_add(1);
            if self
                .messages
                .last()
                .is_some_and(|message| message.role == "assistant" && message.content.is_empty())
            {
                self.messages.pop();
            }
        }
        self.upsert_active_conversation(cx);
        self.messages.clear();
        self.solo_attachments.clear();
        self.solo_is_thinking = false;
        self.solo_response_generation = self.solo_response_generation.wrapping_add(1);
        self.solo_expanded_messages.clear();
        self.active_conversation_id = None;
        self.active_section = SoloSection::Chat;
        self.history_sidebar_open = true;
        self.prompt_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        self.reset_solo_chat_list_state();
        cx.notify();
    }

    pub(super) fn load_conversation(
        &mut self,
        id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.solo_is_thinking {
            self.solo_is_thinking = false;
            self.solo_response_generation = self.solo_response_generation.wrapping_add(1);
            if self
                .messages
                .last()
                .is_some_and(|message| message.role == "assistant" && message.content.is_empty())
            {
                self.messages.pop();
            }
        }
        self.upsert_active_conversation(cx);
        if let Some(conversation) = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .cloned()
        {
            self.messages = conversation.messages;
            self.solo_attachments.clear();
            self.solo_is_thinking = false;
            self.solo_response_generation = self.solo_response_generation.wrapping_add(1);
            self.solo_expanded_messages.clear();
            self.active_conversation_id = Some(id);
            self.active_project_id = conversation.project_id;
            self.active_section = SoloSection::Chat;
            self.history_sidebar_open = true;
            self.prompt_input.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
            self.reset_solo_chat_list_state();
            cx.notify();
        }
    }

    fn template_card(&self, template: SoloTemplate, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let background = linear_gradient(
            135.,
            linear_color_stop(template.base.opacity(0.58), 0.),
            linear_color_stop(template.accent.opacity(0.34), 1.),
        );

        div()
            .id(gpui::ElementId::Name(
                format!("solo-template-card-{}", template.title).into(),
            ))
            .w(px(196.))
            .h(px(128.))
            .rounded_md()
            .border_1()
            .border_color(template.accent.opacity(0.34))
            .bg(background)
            .p(px(16.))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.prefill_prompt(template.prompt, window, cx);
            }))
            .child(
                v_flex()
                    .size_full()
                    .justify_between()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_start()
                            .child(
                                div()
                                    .w(px(36.))
                                    .h(px(36.))
                                    .rounded_md()
                                    .bg(theme.background.opacity(0.34))
                                    .border_1()
                                    .border_color(theme.foreground.opacity(0.08))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(theme.foreground)
                                    .child(Icon::new(template.icon).size(px(17.))),
                            )
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded_md()
                                    .bg(theme.background.opacity(0.26))
                                    .text_size(px(10.))
                                    .text_color(theme.foreground.opacity(0.78))
                                    .child("Solo"),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap(px(5.))
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.foreground)
                                    .child(template.title),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .line_height(gpui::relative(1.3))
                                    .text_color(theme.foreground.opacity(0.74))
                                    .child(template.detail),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_thinking_row(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();

        div()
            .w_full()
            .px(px(2.))
            .py(px(6.))
            .flex()
            .justify_start()
            .child(
                v_flex()
                    .max_w(px(640.))
                    .gap(px(4.))
                    .items_start()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.muted_foreground)
                            .child("Personal AI"),
                    )
                    .child(
                        div()
                            .w(px(92.))
                            .h(px(64.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                img("icons/thinking.gif")
                                    .size(px(52.))
                                    .object_fit(ObjectFit::Contain),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn attachment_name(path: &str) -> String {
        std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path)
            .to_string()
    }

    fn attachment_is_image(path: &str) -> bool {
        std::path::Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp"
                )
            })
            .unwrap_or(false)
    }

    fn attachment_size(path: &str) -> String {
        let Ok(bytes) = std::fs::metadata(path).map(|metadata| metadata.len()) else {
            return "File".to_string();
        };
        if bytes >= 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        }
    }

    fn message_attachment(&self, path: &str, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let preview = div()
            .w(px(36.))
            .h(px(36.))
            .flex_shrink_0()
            .rounded(px(4.))
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background.opacity(0.46));
        let preview = if Self::attachment_is_image(path) {
            preview.child(
                img(std::path::PathBuf::from(path))
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
        } else {
            preview.child(Icon::new(IconName::File).size(px(16.)))
        };

        h_flex()
            .max_w(px(260.))
            .gap(px(8.))
            .items_center()
            .child(preview)
            .child(
                v_flex()
                    .min_w_0()
                    .gap(px(1.))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(Self::attachment_name(path)),
                    )
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(theme.muted_foreground)
                            .child(Self::attachment_size(path)),
                    ),
            )
            .into_any_element()
    }

    fn render_solo_activities(
        &self,
        message_index: usize,
        activities: &[SoloActivity],
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let mut list = v_flex().w_full().gap(px(6.));

        for (activity_index, activity) in activities.iter().enumerate() {
            let expanded = self
                .solo_expanded_activities
                .contains(&(message_index, activity_index));
            let has_detail = activity
                .detail
                .as_deref()
                .is_some_and(|detail| !detail.trim().is_empty());
            let icon = match activity.status {
                SoloActivityStatus::Failed => IconName::CircleX,
                _ => match activity.kind {
                    SoloActivityKind::Search => IconName::Search,
                    SoloActivityKind::Command => IconName::SquareTerminal,
                    SoloActivityKind::File => IconName::File,
                    SoloActivityKind::Tool => IconName::Globe,
                },
            };
            let color = match activity.status {
                SoloActivityStatus::Running => theme.primary,
                SoloActivityStatus::Completed => theme.muted_foreground,
                SoloActivityStatus::Failed => gpui::red(),
            };
            let row = h_flex()
                .id(("solo-activity-row", message_index * 1000 + activity_index))
                .w_full()
                .min_w_0()
                .gap(px(7.))
                .items_center()
                .text_color(color)
                .child(
                    div()
                        .w(px(16.))
                        .h(px(16.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(activity.status == SoloActivityStatus::Running, |icon_box| {
                            icon_box.child(
                                img("icons/thinking.gif")
                                    .size(px(16.))
                                    .object_fit(ObjectFit::Contain),
                            )
                        })
                        .when(activity.status != SoloActivityStatus::Running, |icon_box| {
                            icon_box.child(Icon::new(icon).size(px(13.)))
                        }),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_size(px(11.))
                        .child(activity.label.clone()),
                )
                .when(has_detail, |row| {
                    row.child(
                        Icon::new(if expanded {
                            IconName::ChevronUp
                        } else {
                            IconName::ChevronDown
                        })
                        .size(px(12.)),
                    )
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let key = (message_index, activity_index);
                        if !this.solo_expanded_activities.remove(&key) {
                            this.solo_expanded_activities.insert(key);
                        }
                        this.reset_solo_chat_list_state();
                        cx.notify();
                    }))
                });
            list = list.child(row);

            if expanded {
                if let Some(detail) = activity.detail.as_deref() {
                    let detail = detail.replace("```", "` ` `");
                    list = list.child(
                        div()
                            .ml(px(23.))
                            .max_w(px(680.))
                            .rounded(px(5.))
                            .bg(theme.secondary.opacity(0.42))
                            .px(px(9.))
                            .py(px(7.))
                            .text_size(px(10.))
                            .text_color(theme.muted_foreground)
                            .child(
                                crate::ui::text::TextView::markdown(
                                    (
                                        "solo-activity-detail",
                                        message_index * 1000 + activity_index,
                                    ),
                                    format!("```text\n{}\n```", detail),
                                )
                                .selectable(true),
                            ),
                    );
                }
            }
        }

        list.into_any_element()
    }

    fn render_message_entry(&self, ix: usize, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let message = &self.messages[ix];
        let is_user = message.role == "user";
        if !is_user
            && message.content.is_empty()
            && message.activities.is_empty()
            && self.solo_is_thinking
            && ix + 1 == self.messages.len()
        {
            return self.render_thinking_row(cx);
        }

        let author = if is_user { "You" } else { "Personal AI" };
        let label_color = if is_user {
            theme.primary
        } else {
            theme.muted_foreground
        };
        let mut attachment_rows = h_flex().w_full().gap(px(12.)).flex_wrap();
        for path in &message.attachments {
            attachment_rows = attachment_rows.child(self.message_attachment(path, cx));
        }
        let mut request_metadata = Vec::new();
        if let Some(model) = &message.model {
            request_metadata.push(model.clone());
        }
        if let Some(speed) = &message.speed {
            request_metadata.push(format!("{speed} speed"));
        }
        if !message.tools.is_empty() {
            request_metadata.push(message.tools.join(" + "));
        }

        let is_expanded = self.solo_expanded_messages.contains(&ix);
        let needs_collapse = !is_user
            && (message.content.lines().count() > SOLO_COLLAPSE_LINE_LIMIT
                || exceeds_solo_char_limit(&message.content, SOLO_COLLAPSE_CHAR_LIMIT));
        let content_to_render = if needs_collapse && !is_expanded {
            collapse_solo_message(&message.content)
        } else {
            message.content.clone()
        };
        let content_to_render = truncate_solo_chars(
            &content_to_render,
            SOLO_RENDER_CHAR_LIMIT,
            "\n\n[Response truncated for render safety.]",
        )
        .unwrap_or(content_to_render);
        let mut message_text =
            crate::ui::text::TextView::markdown(("solo-message-markdown", ix), content_to_render)
                .selectable(true);
        if !is_user {
            let view = cx.entity().clone();
            message_text = message_text.on_link_click(move |url, window, cx| {
                view.update(cx, |this, cx| {
                    this.open_agent_link_in_webview(url, window, cx)
                })
            });
        }

        let bubble = v_flex()
            .max_w(px(720.))
            .gap(px(10.))
            .rounded_md()
            .border_1()
            .border_color(if is_user {
                theme.primary.opacity(0.24)
            } else {
                theme.border
            })
            .bg(if is_user {
                theme.primary.opacity(0.10)
            } else {
                theme.secondary.opacity(0.30)
            })
            .p(px(11.))
            .text_size(px(12.))
            .line_height(gpui::relative(1.45))
            .text_color(theme.foreground)
            .child(message_text)
            .when(!message.attachments.is_empty(), |bubble| {
                bubble.child(attachment_rows)
            })
            .when(!request_metadata.is_empty(), |bubble| {
                bubble.child(
                    div()
                        .text_size(px(9.))
                        .text_color(theme.muted_foreground)
                        .child(request_metadata.join(" | ")),
                )
            })
            .when(needs_collapse, |bubble| {
                bubble.child(
                    h_flex()
                        .id(("solo-message-collapse", ix))
                        .w_full()
                        .gap(px(4.))
                        .items_center()
                        .cursor_pointer()
                        .text_size(px(10.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.primary)
                        .child(
                            Icon::new(if is_expanded {
                                IconName::ChevronUp
                            } else {
                                IconName::ChevronDown
                            })
                            .size(px(12.)),
                        )
                        .child(if is_expanded {
                            "Show less"
                        } else {
                            "Show more"
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.solo_expanded_messages.remove(&ix) {
                                this.solo_expanded_messages.insert(ix);
                            }
                            this.reset_solo_chat_list_state();
                            cx.notify();
                        })),
                )
            });

        div()
            .w_full()
            .px(px(2.))
            .py(px(6.))
            .flex()
            .when(is_user, |row| row.justify_end())
            .when(!is_user, |row| row.justify_start())
            .child(
                v_flex()
                    .max_w(gpui::relative(0.82))
                    .gap(px(4.))
                    .when(is_user, |col| col.items_end())
                    .when(!is_user, |col| col.items_start())
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(label_color)
                            .child(author),
                    )
                    .when(!message.activities.is_empty(), |col| {
                        col.child(self.render_solo_activities(ix, &message.activities, cx))
                    })
                    .when(!message.content.is_empty(), |col| col.child(bubble))
                    .when(
                        !is_user
                            && message.content.is_empty()
                            && self.solo_is_thinking
                            && ix + 1 == self.messages.len(),
                        |col| {
                            col.child(
                                div()
                                    .w(px(44.))
                                    .h(px(34.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        img("icons/thinking.gif")
                                            .size(px(28.))
                                            .object_fit(ObjectFit::Contain),
                                    ),
                            )
                        },
                    ),
            )
            .into_any_element()
    }

    fn tool_toggle(&self, tool: SoloTool, cx: &Context<Self>) -> gpui::AnyElement {
        let selected = match tool {
            SoloTool::Build => self.solo_build_enabled,
            SoloTool::Skills => !self.solo_selected_skill_ids.is_empty(),
        };
        Button::new(gpui::ElementId::Name(
            format!("solo-tool-{}", tool.label().to_ascii_lowercase()).into(),
        ))
        .small()
        .selected(selected)
        .icon(tool.icon())
        .label(tool.label())
        .tooltip(format!("Toggle {}", tool.label()))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.toggle_solo_tool(tool, cx);
        }))
        .into_any_element()
    }

    fn skill_control(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let skills = self.solo_skills.clone();
        let selected_skill_ids = self.solo_selected_skill_ids.clone();
        let selected_count = selected_skill_ids.len();
        let view = cx.entity().clone();
        let trigger = Button::new("solo-skill-toggle")
            .small()
            .selected(selected_count > 0)
            .icon(SoloTool::Skills.icon())
            .label(if selected_count == 0 {
                "Skills".to_string()
            } else {
                format!("Skills ({selected_count})")
            })
            .dropdown_caret(true)
            .tooltip("Choose skills for this request");

        Popover::new("solo-skills-popover")
            .anchor(Corner::BottomLeft)
            .trigger(trigger)
            .content(move |_state, _window, cx| {
                let theme = cx.theme().clone();
                let mut list = v_flex().w_full().gap(px(3.));
                for skill in skills.iter().cloned() {
                    let selected = selected_skill_ids.contains(&skill.id);
                    let skill_id = skill.id.clone();
                    let view = view.clone();
                    list = list.child(
                        h_flex()
                            .id(gpui::ElementId::Name(
                                format!("solo-skill-option-{}", skill.id).into(),
                            ))
                            .w_full()
                            .min_w_0()
                            .gap(px(9.))
                            .items_center()
                            .px(px(9.))
                            .py(px(7.))
                            .rounded(px(5.))
                            .cursor_pointer()
                            .when(selected, |row| row.bg(theme.primary.opacity(0.10)))
                            .hover(|row| row.bg(theme.secondary))
                            .child(
                                div()
                                    .w(px(18.))
                                    .h(px(18.))
                                    .flex_shrink_0()
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(if selected {
                                        theme.primary
                                    } else {
                                        theme.border
                                    })
                                    .bg(if selected {
                                        theme.primary
                                    } else {
                                        theme.background
                                    })
                                    .text_color(theme.primary_foreground)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(selected, |box_| {
                                        box_.child(Icon::new(IconName::Check).size(px(12.)))
                                    }),
                            )
                            .child(
                                v_flex()
                                    .min_w_0()
                                    .flex_1()
                                    .gap(px(2.))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .min_w_0()
                                            .gap(px(7.))
                                            .items_center()
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .truncate()
                                                    .text_size(px(12.))
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .child(skill.name),
                                            )
                                            .child(
                                                div()
                                                    .flex_shrink_0()
                                                    .px(px(5.))
                                                    .py(px(1.))
                                                    .rounded(px(3.))
                                                    .bg(theme.secondary)
                                                    .text_size(px(9.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(skill.category),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(10.))
                                            .text_color(theme.muted_foreground)
                                            .child(skill.description),
                                    ),
                            )
                            .on_click(move |_, _, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    this.toggle_solo_skill(&skill_id, cx);
                                });
                            }),
                    );
                }

                v_flex()
                    .w(px(360.))
                    .gap(px(8.))
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("Apply skills"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{selected_count} selected")),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .max_h(px(340.))
                            .overflow_y_scrollbar()
                            .child(list),
                    )
            })
            .into_any_element()
    }

    fn mcp_control(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let tools = self.available_solo_mcp_tools(cx);
        let selected_tool_ids = self.solo_selected_mcp_tool_ids.clone();
        let selected_count = selected_tool_ids
            .iter()
            .filter(|tool_id| tools.iter().any(|(tool, _)| &tool.id == *tool_id))
            .count();
        let view = cx.entity().clone();
        let trigger = Button::new("solo-mcp-toggle")
            .small()
            .selected(selected_count > 0)
            .icon(Icon::empty().path("icons/Mcp.svg").size(px(14.)))
            .label(if selected_count == 0 {
                "MCP".to_string()
            } else {
                format!("MCP ({selected_count})")
            })
            .dropdown_caret(true)
            .tooltip("Choose MCP tools for this request");

        Popover::new("solo-mcp-popover")
            .anchor(Corner::BottomLeft)
            .trigger(trigger)
            .content(move |_state, _window, cx| {
                let theme = cx.theme().clone();
                let mut list = v_flex().w_full().gap(px(3.));
                if tools.is_empty() {
                    list = list.child(
                        v_flex()
                            .w_full()
                            .items_center()
                            .gap(px(8.))
                            .px(px(14.))
                            .py(px(20.))
                            .child(
                                Icon::empty()
                                    .path("icons/Mcp.svg")
                                    .size(px(22.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("No active MCP tools"),
                            )
                            .child(
                                div()
                                    .text_align(gpui::TextAlign::Center)
                                    .text_size(px(10.))
                                    .text_color(theme.muted_foreground)
                                    .child("Enable a server and discover its tools in MCP Marketplace."),
                            ),
                    );
                } else {
                    for (tool, server_name) in tools.iter().cloned() {
                        let selected = selected_tool_ids.contains(&tool.id);
                        let tool_id = tool.id.clone();
                        let view = view.clone();
                        list = list.child(
                            h_flex()
                                .id(gpui::ElementId::Name(
                                    format!("solo-mcp-option-{}", tool.id).into(),
                                ))
                                .w_full()
                                .min_w_0()
                                .gap(px(9.))
                                .items_center()
                                .px(px(9.))
                                .py(px(7.))
                                .rounded(px(5.))
                                .cursor_pointer()
                                .when(selected, |row| row.bg(theme.primary.opacity(0.10)))
                                .hover(|row| row.bg(theme.secondary))
                                .child(
                                    div()
                                        .w(px(18.))
                                        .h(px(18.))
                                        .flex_shrink_0()
                                        .rounded(px(4.))
                                        .border_1()
                                        .border_color(if selected {
                                            theme.primary
                                        } else {
                                            theme.border
                                        })
                                        .bg(if selected {
                                            theme.primary
                                        } else {
                                            theme.background
                                        })
                                        .text_color(theme.primary_foreground)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .when(selected, |box_| {
                                            box_.child(Icon::new(IconName::Check).size(px(12.)))
                                        }),
                                )
                                .child(
                                    v_flex()
                                        .min_w_0()
                                        .flex_1()
                                        .gap(px(2.))
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .min_w_0()
                                                .gap(px(7.))
                                                .items_center()
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .truncate()
                                                        .text_size(px(12.))
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .child(tool.name),
                                                )
                                                .child(
                                                    div()
                                                        .flex_shrink_0()
                                                        .max_w(px(120.))
                                                        .truncate()
                                                        .px(px(5.))
                                                        .py(px(1.))
                                                        .rounded(px(3.))
                                                        .bg(theme.secondary)
                                                        .text_size(px(9.))
                                                        .text_color(theme.muted_foreground)
                                                        .child(server_name),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(10.))
                                                .text_color(theme.muted_foreground)
                                                .child(tool.description),
                                        ),
                                )
                                .on_click(move |_, _, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.toggle_solo_mcp_tool(&tool_id, cx);
                                    });
                                }),
                        );
                    }
                }

                v_flex()
                    .w(px(380.))
                    .gap(px(8.))
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("MCP tools"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{selected_count} selected")),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .max_h(px(340.))
                            .overflow_y_scrollbar()
                            .child(list),
                    )
            })
            .into_any_element()
    }

    fn mention_picker(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let options = self.filtered_solo_mention_options();
        let mut list = v_flex().w_full().gap(px(2.));
        if options.is_empty() {
            list = list.child(
                div()
                    .px(px(10.))
                    .py(px(12.))
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child("No matching mentions"),
            );
        } else {
            for (index, option) in options.into_iter().enumerate() {
                let mention_id = option.id.clone();
                let is_selected = index == self.solo_mention_selection_index;
                list = list.child(
                    h_flex()
                        .id(gpui::ElementId::Name(
                            format!("solo-mention-{}", option.id).into(),
                        ))
                        .w_full()
                        .min_w_0()
                        .gap(px(9.))
                        .items_center()
                        .px(px(9.))
                        .py(px(7.))
                        .rounded(px(5.))
                        .cursor_pointer()
                        .when(is_selected, |row| row.bg(theme.primary.opacity(0.14)))
                        .hover(|row| row.bg(theme.secondary))
                        .child(
                            div()
                                .w(px(28.))
                                .h(px(28.))
                                .flex_shrink_0()
                                .rounded(px(5.))
                                .bg(theme.primary.opacity(0.10))
                                .text_color(theme.primary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(Icon::new(option.icon).size(px(14.))),
                        )
                        .child(
                            v_flex()
                                .min_w_0()
                                .flex_1()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(12.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(option.label),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(10.))
                                        .text_color(theme.muted_foreground)
                                        .child(option.detail),
                                ),
                        )
                        .child(
                            div()
                                .max_w(px(135.))
                                .truncate()
                                .text_size(px(10.))
                                .text_color(theme.primary)
                                .child(format!("@{}", option.id)),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.insert_solo_mention(&mention_id, window, cx);
                        })),
                );
            }
        }

        v_flex()
            .id("solo-mention-picker")
            .w(px(390.))
            .max_h(px(330.))
            .p(px(6.))
            .rounded(px(7.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .shadow_lg()
            .overflow_y_scrollbar()
            .child(list)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.solo_mention_query = None;
                this.solo_mention_start = None;
                this.solo_mention_selection_index = 0;
                cx.notify();
            }))
            .into_any_element()
    }

    fn webview_button(&self, cx: &Context<Self>) -> gpui::AnyElement {
        Button::new("solo-open-webview")
            .small()
            .ghost()
            .icon(IconName::Globe)
            .tooltip("Open Webview")
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_webview(window, cx);
            }))
            .into_any_element()
    }

    fn webview_split_panel(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let url = Self::normalize_webview_url(&self.webview_url_input.read(cx).text().to_string());
        let webview_content = if let Some(webview) = self.solo_webview.clone() {
            div().size_full().child(webview).into_any_element()
        } else {
            let message = self
                .solo_webview_error
                .clone()
                .unwrap_or_else(|| format!("Unable to start WebView for {}.", url));
            v_flex()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .w(px(46.))
                        .h(px(46.))
                        .rounded_md()
                        .bg(theme.primary.opacity(0.10))
                        .text_color(theme.primary)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(IconName::Globe).size(px(22.))),
                )
                .child(
                    div()
                        .max_w(px(420.))
                        .text_align(gpui::TextAlign::Center)
                        .text_size(px(12.))
                        .line_height(gpui::relative(1.35))
                        .text_color(theme.muted_foreground)
                        .child(message),
                )
                .into_any_element()
        };

        v_flex()
            .w(px(self.solo_webview_width))
            .min_w(px(420.))
            .h_full()
            .flex_shrink_0()
            .bg(theme.secondary.opacity(0.18))
            .p(px(16.))
            .gap(px(12.))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(8.))
                            .text_color(theme.foreground)
                            .child(Icon::new(IconName::Globe).size(px(16.)))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Webview"),
                            ),
                    )
                    .child(
                        Button::new("solo-close-webview")
                            .small()
                            .ghost()
                            .icon(IconName::Close)
                            .tooltip("Close Webview")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_webview(cx);
                            })),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .h(px(38.))
                    .gap(px(8.))
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.webview_url_input).small().w_full()),
                    )
                    .child(
                        Button::new("solo-webview-go")
                            .small()
                            .primary()
                            .icon(IconName::ArrowRight)
                            .tooltip("Load URL")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.commit_webview_url(window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background.opacity(0.72))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(webview_content),
            )
            .into_any_element()
    }

    fn webview_resize_handle(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        div()
            .id("solo-webview-resize-handle")
            .group("solo-webview-resize-group")
            .w(px(6.))
            .h_full()
            .flex_shrink_0()
            .cursor(CursorStyle::ResizeLeftRight)
            .flex()
            .justify_center()
            .child(
                div()
                    .w(px(1.))
                    .h_full()
                    .bg(theme.border.opacity(0.78))
                    .group_hover("solo-webview-resize-group", |style| style.bg(theme.primary)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.solo_webview_resizing = true;
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    fn composer_attachment(
        &self,
        index: usize,
        path: &str,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let hover_group = format!("solo-attachment-hover-{index}");
        let file_name = Self::attachment_name(path);
        let is_image = Self::attachment_is_image(path);
        let card_width = if is_image {
            142.0
        } else {
            (96.0 + file_name.chars().count() as f32 * 5.6).clamp(140.0, 220.0)
        };
        let preview = div()
            .w(px(30.))
            .h(px(30.))
            .flex_shrink_0()
            .rounded(px(4.))
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background.opacity(0.74));
        let preview = if is_image {
            preview.child(
                img(std::path::PathBuf::from(path))
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
        } else {
            preview.child(Icon::new(IconName::File).size(px(14.)))
        };

        h_flex()
            .relative()
            .group(hover_group.clone())
            .h(px(42.))
            .w(px(card_width))
            .flex_shrink_0()
            .gap(px(7.))
            .px(px(6.))
            .rounded(px(4.))
            .bg(theme.secondary.opacity(0.62))
            .child(preview)
            .child(
                v_flex()
                    .min_w_0()
                    .flex_1()
                    .pr(px(12.))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(10.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(file_name),
                    )
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(theme.muted_foreground)
                            .child(Self::attachment_size(path)),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px(2.))
                    .right(px(2.))
                    .w(px(16.))
                    .h(px(16.))
                    .rounded_full()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .invisible()
                    .group_hover(hover_group, |style| {
                        style.visible().text_color(theme.foreground)
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Icon::new(IconName::Close).size(px(10.)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.remove_solo_attachment(index, cx);
                            cx.stop_propagation();
                        }),
                    ),
            )
            .into_any_element()
    }

    fn aurora_cosmic_effort(level: usize, colors: [gpui::Hsla; 3]) -> gpui::AnyElement {
        let mut layer = div().absolute().top(px(0.)).left(px(0.)).w_full().h_full();
        match level {
            // Light: a single calm Aurora breath.
            0 => {
                layer = layer.child(
                    div()
                        .absolute()
                        .left(px(86.))
                        .top(px(5.))
                        .w(px(168.))
                        .h(px(20.))
                        .rounded_full()
                        .bg(linear_gradient(
                            90.,
                            linear_color_stop(colors[0].opacity(0.03), 0.),
                            linear_color_stop(colors[1].opacity(0.20), 1.),
                        ))
                        .with_animation(
                            "effort-light-aurora-breath",
                            Animation::new(Duration::from_secs_f64(4.8)).repeat(),
                            |glow, delta| {
                                let wave = (delta * std::f32::consts::TAU).sin().abs();
                                glow.opacity(0.28 + wave * 0.42)
                            },
                        ),
                );
            }
            // Medium: a quiet field of independently twinkling stars.
            1 => {
                let positions = [(38., 8.), (94., 20.), (166., 7.), (242., 19.), (307., 9.)];
                for (index, (left, top)) in positions.into_iter().enumerate() {
                    let phase = index as f32 * 0.19;
                    layer = layer.child(
                        div()
                            .absolute()
                            .left(px(left))
                            .top(px(top))
                            .w(px(if index % 2 == 0 { 3.6 } else { 2.6 }))
                            .h(px(if index % 2 == 0 { 3.6 } else { 2.6 }))
                            .rounded_full()
                            .bg(colors[index % colors.len()])
                            .with_animation(
                                gpui::ElementId::Name(format!("effort-medium-star-{index}").into()),
                                Animation::new(Duration::from_secs_f64(2.2 + index as f64 * 0.23))
                                    .repeat(),
                                move |star, delta| {
                                    let twinkle = ((delta + phase).fract() * std::f32::consts::TAU)
                                        .sin()
                                        .abs();
                                    star.opacity(0.18 + twinkle * 0.64)
                                },
                            ),
                    );
                }
            }
            // High: one comet crossing a soft Aurora trail.
            2 => {
                layer = layer
                    .child(
                        div()
                            .absolute()
                            .left(px(0.))
                            .top(px(8.))
                            .w(px(96.))
                            .h(px(13.))
                            .rounded_full()
                            .bg(linear_gradient(
                                90.,
                                linear_color_stop(colors[2].opacity(0.02), 0.),
                                linear_color_stop(colors[0].opacity(0.28), 1.),
                            ))
                            .with_animation(
                                "effort-high-comet-trail",
                                Animation::new(Duration::from_secs_f64(3.2)).repeat(),
                                |trail, delta| {
                                    let wave = (delta * std::f32::consts::TAU).sin();
                                    trail.left(px(-84. + delta * 420.)).top(px(8. + wave * 3.))
                                },
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(0.))
                            .top(px(0.))
                            .w(px(6.))
                            .h(px(6.))
                            .rounded_full()
                            .bg(colors[1])
                            .border_1()
                            .border_color(colors[0])
                            .with_animation(
                                "effort-high-comet",
                                Animation::new(Duration::from_secs_f64(3.2)).repeat(),
                                |star, delta| {
                                    let wave = (delta * std::f32::consts::TAU).sin();
                                    star.left(px(8. + delta * 324.)).top(px(12. + wave * 3.))
                                },
                            ),
                    );
            }
            // Extra High: a single galaxy core with one moving orbit.
            3 => {
                let core_x = 174.0_f32;
                let core_y = 15.0_f32;
                layer = layer
                    .child(
                        div()
                            .absolute()
                            .left(px(core_x - 62.))
                            .top(px(core_y - 9.))
                            .w(px(124.))
                            .h(px(18.))
                            .rounded_full()
                            .border_1()
                            .border_color(colors[1].opacity(0.34)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(core_x - 10.))
                            .top(px(core_y - 10.))
                            .w(px(20.))
                            .h(px(20.))
                            .rounded_full()
                            .bg(colors[2].opacity(0.24))
                            .with_animation(
                                "effort-extra-high-galaxy-halo",
                                Animation::new(Duration::from_secs_f64(2.6)).repeat(),
                                |halo, delta| {
                                    let pulse = (delta * std::f32::consts::TAU).sin().abs();
                                    halo.opacity(0.35 + pulse * 0.50)
                                },
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(core_x - 5.))
                            .top(px(core_y - 5.))
                            .w(px(10.))
                            .h(px(10.))
                            .rounded_full()
                            .bg(colors[0])
                            .border_1()
                            .border_color(colors[1]),
                    );
                for index in 0..4 {
                    let phase = index as f32 / 4.0;
                    layer = layer.child(
                        div()
                            .absolute()
                            .left(px(0.))
                            .top(px(0.))
                            .w(px(if index == 0 { 4.4 } else { 3.0 }))
                            .h(px(if index == 0 { 4.4 } else { 3.0 }))
                            .rounded_full()
                            .bg(colors[index % colors.len()])
                            .with_animation(
                                gpui::ElementId::Name(
                                    format!("effort-extra-high-orbit-{index}").into(),
                                ),
                                Animation::new(Duration::from_secs_f64(3.1 + index as f64 * 0.12))
                                    .repeat(),
                                move |star, delta| {
                                    let angle = (delta + phase) * std::f32::consts::TAU;
                                    star.left(px(core_x + angle.cos() * 62. - 2.))
                                        .top(px(core_y + angle.sin() * 9. - 2.))
                                },
                            ),
                    );
                }
            }
            // Ultra: two galaxy cores with counter-rotating star systems.
            _ => {
                let cores = [(122.0_f32, colors[1]), (226.0_f32, colors[2])];
                for (core_index, (core_x, core_color)) in cores.into_iter().enumerate() {
                    let core_y = 15.0_f32;
                    layer = layer
                        .child(
                            div()
                                .absolute()
                                .left(px(core_x - 47.))
                                .top(px(core_y - 7.))
                                .w(px(94.))
                                .h(px(14.))
                                .rounded_full()
                                .border_1()
                                .border_color(core_color.opacity(0.32)),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(px(core_x - 8.))
                                .top(px(core_y - 8.))
                                .w(px(16.))
                                .h(px(16.))
                                .rounded_full()
                                .bg(core_color.opacity(0.25)),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(px(core_x - 4.))
                                .top(px(core_y - 4.))
                                .w(px(8.))
                                .h(px(8.))
                                .rounded_full()
                                .bg(colors[core_index]),
                        );
                    for star_index in 0..3 {
                        let phase = star_index as f32 / 3.0;
                        let direction = if core_index == 0 { 1.0_f32 } else { -1.0_f32 };
                        layer = layer.child(
                            div()
                                .absolute()
                                .left(px(0.))
                                .top(px(0.))
                                .w(px(3.4))
                                .h(px(3.4))
                                .rounded_full()
                                .bg(colors[(star_index + core_index) % colors.len()])
                                .with_animation(
                                    gpui::ElementId::Name(
                                        format!("effort-ultra-orbit-{core_index}-{star_index}")
                                            .into(),
                                    ),
                                    Animation::new(Duration::from_secs_f64(
                                        2.5 + star_index as f64 * 0.18,
                                    ))
                                    .repeat(),
                                    move |star, delta| {
                                        let angle =
                                            (delta * direction + phase) * std::f32::consts::TAU;
                                        star.left(px(core_x + angle.cos() * 47. - 1.7))
                                            .top(px(core_y + angle.sin() * 7. - 1.7))
                                    },
                                ),
                        );
                    }
                }
            }
        }

        layer.into_any_element()
    }

    fn compact_solo_model_label(selected_model: &str) -> String {
        let model = selected_model
            .split_once(" / ")
            .map(|(_, model)| model)
            .unwrap_or(selected_model);
        let mut chars = model.chars();
        let compact = chars.by_ref().take(24).collect::<String>();
        if chars.next().is_some() {
            format!("{}...", compact)
        } else {
            compact
        }
    }

    fn runtime_control(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let slider = self.solo_effort_slider.clone();
        let model_select = self.solo_model_select.clone();
        let model_options = super::solo_model_options(cx);
        let panel = cx.entity().clone();
        let selected_model = model_select
            .read(cx)
            .selected_value()
            .map(ToString::to_string)
            .unwrap_or_else(|| "Auto".to_string());
        let selected_model_for_menu = selected_model.clone();
        let effort_label = self.solo_effort_label(cx);
        let trigger = Button::new("solo-runtime-toggle")
            .small()
            .w(px(196.))
            .label(format!(
                "{} | {}",
                Self::compact_solo_model_label(&selected_model),
                effort_label
            ))
            .tooltip(format!("{} | {} effort", selected_model, effort_label))
            .dropdown_caret(true);

        Popover::new("solo-runtime-popover")
            .anchor(Corner::BottomRight)
            .trigger(trigger)
            .content(move |_state, _window, cx| {
                let theme = cx.theme().clone();
                let popover = cx.entity().clone();
                let effort_label = match slider.read(cx).value().end().round() as usize {
                    0 => "Light",
                    1 => "Medium",
                    2 => "High",
                    3 => "Extra High",
                    _ => "Ultra",
                };
                let effort_level = slider.read(cx).value().end().round() as usize;
                let effort_effect = Some(Self::aurora_cosmic_effort(
                    effort_level,
                    [
                        theme.primary,
                        gpui::Hsla::from(gpui::rgb(0x60a5fa)),
                        gpui::Hsla::from(gpui::rgb(0xa855f7)),
                    ],
                ));
                let mut model_list = v_flex().w_full().gap(px(2.));
                for (index, option) in model_options.iter().cloned().enumerate() {
                    let is_selected = option.as_ref() == selected_model_for_menu.as_str();
                    let model_select = model_select.clone();
                    let panel = panel.clone();
                    let popover = popover.clone();
                    let option_for_click = option.clone();
                    model_list = model_list.child(
                        h_flex()
                            .id(("solo-runtime-model-option", index))
                            .w_full()
                            .min_w_0()
                            .h(px(32.))
                            .gap(px(8.))
                            .items_center()
                            .px(px(9.))
                            .rounded(px(5.))
                            .cursor_pointer()
                            .when(is_selected, |row| row.bg(theme.primary.opacity(0.12)))
                            .hover(|row| row.bg(theme.secondary))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(theme.foreground)
                                    .child(option),
                            )
                            .when(is_selected, |row| {
                                row.child(
                                    Icon::new(IconName::Check)
                                        .size(px(14.))
                                        .text_color(theme.primary),
                                )
                            })
                            .on_click(move |_, window, cx| {
                                model_select.update(cx, |state, cx| {
                                    state.set_selected_value(&option_for_click, window, cx);
                                });
                                let _ = panel.update(cx, |_this, cx| cx.notify());
                                let _ = popover.update(cx, |state, cx| {
                                    state.dismiss(window, cx);
                                });
                            }),
                    );
                }

                v_flex()
                    .w(px(348.))
                    .gap(px(14.))
                    .child(
                        v_flex()
                            .w_full()
                            .gap(px(7.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.foreground)
                                    .child("Model"),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .max_h(px(210.))
                                    .overflow_y_scrollbar()
                                    .child(model_list),
                            ),
                    )
                    .child(div().w_full().h(px(1.)).bg(theme.border))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap(px(7.))
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.foreground)
                                            .child("Effort"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.primary)
                                            .child(effort_label),
                                    ),
                            )
                            .child(
                                Button::new("solo-effort-help")
                                    .xsmall()
                                    .compact()
                                    .ghost()
                                    .icon(IconName::Info)
                                    .tooltip("Higher effort spends more time reasoning"),
                            ),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .gap(px(8.))
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Faster"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Smarter"),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(30.))
                                    .relative()
                                    .px(px(4.))
                                    .rounded(px(6.))
                                    .overflow_hidden()
                                    .bg(linear_gradient(
                                        90.,
                                        linear_color_stop(theme.primary.opacity(0.14), 0.),
                                        linear_color_stop(
                                            gpui::Hsla::from(gpui::rgb(0xa855f7)).opacity(0.34),
                                            1.,
                                        ),
                                    ))
                                    .flex()
                                    .items_center()
                                    .when_some(effort_effect, |track, effect| track.child(effect))
                                    .child(Slider::new(&slider).w_full()),
                            ),
                    )
            })
            .into_any_element()
    }

    fn composer_left_controls(&self, cx: &Context<Self>) -> gpui::AnyElement {
        h_flex()
            .min_w_0()
            .gap(px(6.))
            .items_center()
            .child(
                Button::new("solo-add-files")
                    .small()
                    .compact()
                    .ghost()
                    .icon(Icon::empty().path("icons/attachment.svg").size(px(15.)))
                    .tooltip("Add files")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.pick_solo_attachments(false, cx);
                    })),
            )
            .child(
                Button::new("solo-add-images")
                    .small()
                    .compact()
                    .ghost()
                    .icon(IconName::GalleryVerticalEnd)
                    .tooltip("Add images")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.pick_solo_attachments(true, cx);
                    })),
            )
            .child(
                Button::new("solo-add-mention")
                    .small()
                    .compact()
                    .ghost()
                    .label("@")
                    .tooltip("Mention project, file, memory, or skill")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_solo_mentions(window, cx);
                    })),
            )
            .child(self.tool_toggle(SoloTool::Build, cx))
            .child(self.skill_control(cx))
            .child(self.mcp_control(cx))
            .into_any_element()
    }

    fn composer_right_controls(&self, cx: &Context<Self>) -> gpui::AnyElement {
        h_flex()
            .flex_shrink_0()
            .min_w_0()
            .gap(px(6.))
            .items_center()
            .child(self.runtime_control(cx))
            .child(
                Button::new("solo-send")
                    .primary()
                    .icon(Icon::empty().path("icons/send.svg").size(px(16.)))
                    .tooltip("Send")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.submit_prompt(window, cx);
                    })),
            )
            .into_any_element()
    }

    fn composer(&self, compact: bool, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let has_attachments = !self.solo_attachments.is_empty();
        let has_mentions = self.solo_mention_query.is_some();
        let input_height = 92.0;
        let footer_height = if compact { 92.0 } else { 52.0 };
        let composer_height = input_height + footer_height;
        let attachment_offset = if has_attachments { 70.0 } else { 0.0 };
        let wrapper_height = composer_height + attachment_offset;
        let mut attachments = h_flex().h(px(50.)).gap(px(8.)).items_center();
        for (index, path) in self.solo_attachments.iter().enumerate() {
            attachments = attachments.child(self.composer_attachment(index, path, cx));
        }
        let left_controls = self.composer_left_controls(cx);
        let right_controls = self.composer_right_controls(cx);
        let footer = if compact {
            v_flex()
                .w_full()
                .min_w_0()
                .h(px(footer_height))
                .min_h(px(footer_height))
                .max_h(px(footer_height))
                .flex_shrink_0()
                .gap(px(6.))
                .px(px(12.))
                .py(px(7.))
                .border_t_1()
                .border_color(theme.border)
                .child(div().w_full().min_w_0().child(left_controls))
                .child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .justify_end()
                        .child(right_controls),
                )
                .into_any_element()
        } else {
            h_flex()
                .h(px(footer_height))
                .min_h(px(footer_height))
                .max_h(px(footer_height))
                .flex_shrink_0()
                .w_full()
                .min_w_0()
                .px(px(12.))
                .border_t_1()
                .border_color(theme.border)
                .justify_between()
                .items_center()
                .child(div().min_w_0().flex_1().child(left_controls))
                .child(right_controls)
                .into_any_element()
        };

        let composer_key_context = if has_mentions {
            "SoloChatMentions"
        } else {
            "SoloChatComposer"
        };
        let composer_card = v_flex()
            .w_full()
            .min_w_0()
            .h(px(composer_height))
            .min_h(px(composer_height))
            .max_h(px(composer_height))
            .flex_shrink_0()
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .overflow_hidden()
            .key_context(composer_key_context)
            .on_action(cx.listener(Self::on_prompt_confirm))
            .on_action(cx.listener(Self::on_solo_mention_previous))
            .on_action(cx.listener(Self::on_solo_mention_next))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .h(px(input_height))
                    .min_h(px(input_height))
                    .max_h(px(input_height))
                    .flex_shrink_0()
                    .p(px(14.))
                    .overflow_hidden()
                    .child(
                        Input::new(&self.prompt_input)
                            .appearance(false)
                            .w_full()
                            .h_full(),
                    ),
            )
            .child(footer);

        div()
            .w_full()
            .min_w_0()
            .h(px(wrapper_height))
            .relative()
            .when(has_attachments, |wrapper| {
                wrapper.child(
                    div()
                        .absolute()
                        .top(px(0.))
                        .left(px(0.))
                        .w_full()
                        .min_w_0()
                        .h(px(78.))
                        .px(px(30.))
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .h_full()
                                .pt(px(15.))
                                .px(px(10.))
                                .rounded(px(8.))
                                .border_1()
                                .border_color(theme.border)
                                .bg(theme.background)
                                .overflow_hidden()
                                .child(
                                    div()
                                        .w_full()
                                        .min_w_0()
                                        .h(px(50.))
                                        .overflow_x_scrollbar()
                                        .child(attachments),
                                ),
                        ),
                )
            })
            .child(
                div()
                    .absolute()
                    .top(px(attachment_offset))
                    .left(px(0.))
                    .w_full()
                    .min_w_0()
                    .h(px(composer_height))
                    .child(composer_card),
            )
            .when(has_mentions, |wrapper| {
                wrapper.child(
                    div()
                        .absolute()
                        .bottom(px(wrapper_height + 8.))
                        .left(px(0.))
                        .child(self.mention_picker(cx)),
                )
            })
            .into_any_element()
    }

    pub(super) fn chat_content(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let window_width: f32 = window.bounds().size.width.into();
        if self.solo_webview_open {
            let max_width = (window_width - 940.0).max(420.0);
            self.solo_webview_width = self.solo_webview_width.clamp(420.0, max_width);
        }
        let occupied_width = 248.0
            + if self.history_sidebar_open {
                260.0
            } else {
                0.0
            }
            + if self.solo_webview_open {
                self.solo_webview_width + 6.0
            } else {
                0.0
            };
        let content_width = (window_width - occupied_width - 56.0).clamp(360.0, 880.0);
        let compact = content_width < 720.0;
        let has_messages = !self.messages.is_empty();
        let display_count = self.solo_display_count();
        if self.solo_chat_list_state.item_count() != display_count {
            self.solo_chat_list_state.reset(display_count);
        }
        let templates = solo_templates();
        let template_row = if compact {
            let mut rows = v_flex().w(px(content_width)).gap(px(12.)).items_center();
            for pair in templates.chunks(2) {
                let mut row = h_flex().gap(px(12.)).items_center().justify_center();
                for template in pair {
                    row = row.child(self.template_card(template.clone(), cx));
                }
                rows = rows.child(row);
            }
            rows.into_any_element()
        } else {
            let mut row = h_flex()
                .w(px(content_width))
                .gap(px(12.))
                .items_center()
                .justify_center();
            for template in templates {
                row = row.child(self.template_card(template, cx));
            }
            row.into_any_element()
        };

        let top_bar = h_flex()
            .w(px(content_width))
            .gap(px(6.))
            .justify_end()
            .items_center()
            .child(self.webview_button(cx));

        let header = v_flex()
            .w(px(content_width))
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(24.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme.foreground)
                    .child("Solo Mode"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(gpui::relative(1.35))
                    .text_align(gpui::TextAlign::Center)
                    .text_color(theme.muted_foreground)
                    .child("Private AI workspace for your own domains, projects, tasks, automations, memory, and guarded desktop control."),
            );

        let chat_pane = if !has_messages {
            v_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .items_center()
                .justify_center()
                .px(px(28.))
                .pt(px(18.))
                .pb(px(20.))
                .child(
                    v_flex()
                        .w(px(content_width))
                        .h_full()
                        .items_center()
                        .gap(px(18.))
                        .child(top_bar)
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    v_flex()
                                        .w_full()
                                        .items_center()
                                        .gap(px(18.))
                                        .child(header)
                                        .child(template_row)
                                        .child(self.composer(compact, cx)),
                                ),
                        ),
                )
                .into_any_element()
        } else {
            v_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .items_center()
                .px(px(28.))
                .pt(px(18.))
                .pb(px(20.))
                .gap(px(12.))
                .child(top_bar)
                .child(
                    div()
                        .w(px(content_width))
                        .flex_1()
                        .min_h_0()
                        .pb(px(12.))
                        .child(
                            gpui::list(
                                self.solo_chat_list_state.clone(),
                                cx.processor(|this: &mut Self, ix, _window, cx| {
                                    this.render_message_entry(ix, cx)
                                }),
                            )
                            .size_full(),
                        ),
                )
                .child(
                    div()
                        .w(px(content_width))
                        .flex_shrink_0()
                        .child(self.composer(compact, cx)),
                )
                .into_any_element()
        };
        if self.solo_webview_open {
            h_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .overflow_hidden()
                .on_mouse_move(
                    cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| {
                        let mouse_x: f32 = event.position.x.into();
                        this.resize_webview_at(mouse_x, window, cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        if this.solo_webview_resizing {
                            this.solo_webview_resizing = false;
                            cx.notify();
                        }
                    }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .overflow_hidden()
                        .child(chat_pane),
                )
                .child(self.webview_resize_handle(cx))
                .child(self.webview_split_panel(cx))
                .into_any_element()
        } else {
            chat_pane
        }
    }
}

fn exceeds_solo_char_limit(value: &str, limit: usize) -> bool {
    value.chars().nth(limit).is_some()
}

fn truncate_solo_chars(value: &str, max_chars: usize, marker: &str) -> Option<String> {
    let mut chars = value.chars();
    let mut output = String::new();
    for _ in 0..max_chars {
        let Some(ch) = chars.next() else {
            return None;
        };
        output.push(ch);
    }
    if chars.next().is_none() {
        return None;
    }
    output.push_str(marker);
    Some(output)
}

fn collapse_solo_message(value: &str) -> String {
    let line_limited = if value.lines().count() > SOLO_COLLAPSE_LINE_LIMIT {
        value
            .lines()
            .take(SOLO_COLLAPSE_LINE_LIMIT)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        value.to_string()
    };
    let mut output =
        truncate_solo_chars(&line_limited, SOLO_COLLAPSE_CHAR_LIMIT, "").unwrap_or(line_limited);
    output.push_str("\n\n[Response collapsed. Select Show more to continue reading.]");
    output
}

#[cfg(test)]
mod solo_chat_tests {
    use super::*;

    fn provider(id: &str, name: &str, status: &str) -> crate::db::Provider {
        crate::db::Provider {
            id: id.to_string(),
            provider_name: name.to_string(),
            model: format!("{name}-model"),
            adapter_type: "CustomAdapter".to_string(),
            command: Some("https://example.test".to_string()),
            api_key_ref: Some("env:TEST_KEY".to_string()),
            status: status.to_string(),
            capabilities: None,
        }
    }

    #[test]
    fn collapse_preview_respects_character_limit() {
        let content = "a".repeat(SOLO_COLLAPSE_CHAR_LIMIT + 20);
        let preview = collapse_solo_message(&content);

        assert!(preview.starts_with(&"a".repeat(SOLO_COLLAPSE_CHAR_LIMIT)));
        assert!(preview.contains("Response collapsed"));
    }

    #[test]
    fn collapse_preview_respects_line_limit() {
        let content = (0..100)
            .map(|line| format!("line {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let preview = collapse_solo_message(&content);

        assert!(preview.contains("line 79"));
        assert!(!preview.contains("line 80"));
    }

    #[test]
    fn agent_webview_links_accept_web_urls_and_reject_other_schemes() {
        assert_eq!(
            SoloWorkspacePanel::normalize_agent_link_for_webview("https://example.com/docs"),
            Some("https://example.com/docs".to_string())
        );
        assert_eq!(
            SoloWorkspacePanel::normalize_agent_link_for_webview("www.example.com"),
            Some("https://www.example.com".to_string())
        );
        assert_eq!(
            SoloWorkspacePanel::normalize_agent_link_for_webview("localhost:3000"),
            Some("http://localhost:3000".to_string())
        );
        assert_eq!(
            SoloWorkspacePanel::normalize_agent_link_for_webview("javascript:alert(1)"),
            None
        );
        assert_eq!(
            SoloWorkspacePanel::normalize_agent_link_for_webview("mailto:user@example.com"),
            None
        );
    }

    #[test]
    fn auto_provider_order_prefers_last_success_and_excludes_offline_entries() {
        let mut providers = vec![
            provider("recent-db-row", "Alpha", "available"),
            provider("preferred", "Zulu", "available"),
            provider("offline", "Beta", "disabled"),
        ];

        SoloWorkspacePanel::order_solo_auto_providers(&mut providers, Some("preferred"));

        assert_eq!(
            providers
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>(),
            vec!["preferred", "recent-db-row"]
        );
    }

    #[test]
    fn solo_tool_calls_parse_json_and_string_arguments() {
        let calls = SoloWorkspacePanel::parse_solo_tool_calls(
            r#"<tool_call>{"id":"one","name":"web_search","arguments":{"q":"rust"}}</tool_call>
<tool_call>{"name":"read_file","arguments":"{\"path\":\"README.md\"}"}</tool_call>"#,
        );

        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "web_search");
        assert_eq!(calls[0].arguments["q"], "rust");
        assert_eq!(calls[1].name, "read_file");
        assert_eq!(calls[1].arguments["path"], "README.md");
        assert!(!calls[1].id.is_empty());
    }

    #[test]
    fn solo_tool_activity_classifies_search_command_and_file_tools() {
        assert_eq!(
            SoloWorkspacePanel::solo_tool_activity_kind("web_search"),
            SoloActivityKind::Search
        );
        assert_eq!(
            SoloWorkspacePanel::solo_tool_activity_kind("run_shell_command"),
            SoloActivityKind::Command
        );
        assert_eq!(
            SoloWorkspacePanel::solo_tool_activity_kind("read_file"),
            SoloActivityKind::File
        );
    }
}
