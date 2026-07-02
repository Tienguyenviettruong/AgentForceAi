use crate::infrastructure::mcp::registry::McpServerRecord;
use gpui::prelude::FluentBuilder;
use gpui::EventEmitter;
use gpui::{
    div, px, App, AppContext, Context, Focusable, IntoElement, ParentElement, Render, Styled,
    Window,
};
use gpui_component::dock::PanelEvent;
use gpui_component::dock::{Panel, TitleStyle};
use gpui_component::{
    button::Button,
    button::ButtonVariants,
    form::{field, v_form},
    h_flex,
    input::{Input, InputState},
    scroll::ScrollableElement,
    theme::ActiveTheme,
    v_flex, Sizable, StyledExt, WindowExt,
};
use std::collections::HashSet;

pub struct McpMarketplacePanel {
    focus_handle: gpui::FocusHandle,
    config_status: Option<String>,
}

#[derive(Clone)]
struct McpCatalogEntry {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    command: &'static str,
    args: &'static [&'static str],
    env_refs: &'static [(&'static str, &'static str)],
}

impl McpMarketplacePanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            config_status: None,
        }
    }

    fn catalog_entries() -> Vec<McpCatalogEntry> {
        vec![
            McpCatalogEntry {
                id: "github",
                name: "GitHub",
                description: "Repository issues, pull requests, files, and project automation.",
                command: "npx",
                args: &["-y", "@modelcontextprotocol/server-github"],
                env_refs: &[("GITHUB_PERSONAL_ACCESS_TOKEN", "secret://github-token")],
            },
            McpCatalogEntry {
                id: "sequential-thinking",
                name: "Sequential Thinking",
                description: "Structured planning tool for decomposing multi-step work.",
                command: "npx",
                args: &["-y", "@modelcontextprotocol/server-sequential-thinking"],
                env_refs: &[],
            },
            McpCatalogEntry {
                id: "memory",
                name: "Memory",
                description: "MCP memory server for durable facts and relationship lookup.",
                command: "npx",
                args: &["-y", "@modelcontextprotocol/server-memory"],
                env_refs: &[],
            },
        ]
    }

    fn catalog_record(entry: &McpCatalogEntry) -> Result<McpServerRecord, String> {
        let args = entry
            .args
            .iter()
            .map(|argument| argument.to_string())
            .collect::<Vec<_>>();
        Self::validate_args(&args)?;
        let stdio_status = crate::infrastructure::mcp::server::McpServer::stdio_runtime_status();
        let now = chrono::Utc::now().to_rfc3339();
        let env_secret_refs = entry
            .env_refs
            .iter()
            .map(|(name, reference)| {
                (
                    name.to_string(),
                    serde_json::Value::String(reference.to_string()),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        Ok(McpServerRecord {
            id: format!("mcp-server-{}", entry.id),
            name: entry.name.to_string(),
            transport: "stdio".to_string(),
            command: Some(entry.command.to_string()),
            args,
            endpoint: None,
            env_secret_refs: serde_json::Value::Object(env_secret_refs).to_string(),
            header_secret_refs: "{}".to_string(),
            source_kind: "catalog".to_string(),
            is_enabled: stdio_status != "blocked_until_isolated",
            health_status: stdio_status.to_string(),
            last_error: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    fn is_sensitive_name(value: &str) -> bool {
        let value = value.to_ascii_lowercase();
        [
            "token",
            "secret",
            "password",
            "passwd",
            "api-key",
            "apikey",
            "authorization",
        ]
        .iter()
        .any(|term| value.contains(term))
    }

    fn validate_args(args: &[String]) -> Result<(), String> {
        for argument in args {
            if !Self::is_sensitive_name(argument) {
                continue;
            }
            return Err(format!(
                "Credential-bearing argument '{}' is not permitted; pass credentials through env or headers using a secret:// reference.",
                argument
            ));
        }
        Ok(())
    }

    fn validate_endpoint(endpoint: &str) -> Result<(), String> {
        let parsed = reqwest::Url::parse(endpoint)
            .map_err(|error| format!("Invalid remote MCP endpoint: {}", error))?;
        if parsed.scheme() != "https" {
            return Err("Remote MCP endpoints must use HTTPS.".to_string());
        }
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1")
            || host.ends_with(".localhost")
        {
            return Err("Remote MCP endpoints cannot target loopback hosts.".to_string());
        }
        let authority = endpoint
            .split_once("://")
            .map(|(_, rest)| rest.split('/').next().unwrap_or(rest))
            .unwrap_or_default();
        if authority.contains('@') {
            return Err("Endpoint URLs must not embed username/password credentials.".to_string());
        }
        if let Some((_, query)) = endpoint.split_once('?') {
            for parameter in query.split('&') {
                let (name, value) = parameter.split_once('=').unwrap_or((parameter, ""));
                if Self::is_sensitive_name(name) {
                    let _ = value;
                    return Err(format!(
                        "Credential-bearing endpoint parameter '{}' is not permitted; use a secret:// Authorization header.",
                        name
                    ));
                }
            }
        }
        Ok(())
    }

    fn parse_server_config(raw: &str) -> Result<Vec<McpServerRecord>, String> {
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|error| format!("Invalid JSON: {}", error))?;
        let servers = value
            .get("mcpServers")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| "Configuration must contain a mcpServers object.".to_string())?;
        if servers.is_empty() {
            return Err("Configuration must contain at least one MCP server.".to_string());
        }
        let mut records = Vec::new();
        for (name, config) in servers {
            let command = config
                .get("command")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let endpoint = config
                .get("serverUrl")
                .or_else(|| config.get("url"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let transport = if command.is_some() {
                "stdio"
            } else if endpoint.is_some() {
                "http"
            } else {
                return Err(format!(
                    "Server '{}' must define command or serverUrl.",
                    name
                ));
            };
            let args: Vec<String> = config
                .get("args")
                .and_then(serde_json::Value::as_array)
                .map(|args| {
                    args.iter()
                        .filter_map(|arg| arg.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            Self::validate_args(&args)?;
            if let Some(endpoint) = endpoint.as_deref() {
                Self::validate_endpoint(endpoint)?;
            }

            let protected_config = |key: &str| -> Result<String, String> {
                let Some(values) = config.get(key).and_then(serde_json::Value::as_object) else {
                    return Ok("{}".to_string());
                };
                for (item_name, value) in values {
                    let value = value.as_str().ok_or_else(|| {
                        format!("{} value {} must be a secret reference.", key, item_name)
                    })?;
                    let sensitive_header = key == "headers"
                        && matches!(
                            item_name.to_ascii_lowercase().as_str(),
                            "authorization"
                                | "proxy-authorization"
                                | "x-api-key"
                                | "api-key"
                                | "cookie"
                                | "set-cookie"
                        );
                    if (key == "env" || sensitive_header) && !value.starts_with("secret://") {
                        return Err(format!(
                            "{} value {} must use secret:// reference; raw credentials are not stored.",
                            key, item_name
                        ));
                    }
                }
                Ok(serde_json::Value::Object(values.clone()).to_string())
            };
            let stdio_status =
                crate::infrastructure::mcp::server::McpServer::stdio_runtime_status();
            let now = chrono::Utc::now().to_rfc3339();
            records.push(McpServerRecord {
                id: format!("mcp-server-{}", name),
                name: name.clone(),
                transport: transport.to_string(),
                command,
                args,
                endpoint,
                env_secret_refs: protected_config("env")?,
                header_secret_refs: protected_config("headers")?,
                source_kind: "manual_config".to_string(),
                is_enabled: transport != "stdio" || stdio_status != "blocked_until_isolated",
                health_status: if transport == "stdio" {
                    stdio_status.to_string()
                } else {
                    "configured".to_string()
                },
                last_error: None,
                created_at: now.clone(),
                updated_at: now,
            });
        }
        Ok(records)
    }
}

impl Panel for McpMarketplacePanel {
    fn panel_name(&self) -> &'static str {
        "MCP Servers"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for McpMarketplacePanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for McpMarketplacePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let state = crate::AppState::global(cx);
        let db = state.db.clone();
        let actor_id = state.current_actor_id.clone();
        let servers = db.list_mcp_servers().unwrap_or_default();
        let tools = db.list_mcp_tools().unwrap_or_default();
        let selected_tool_ids = db
            .list_capability_selections("default", "")
            .unwrap_or_default()
            .into_iter()
            .filter(|selection| selection.enabled && selection.capability_kind == "mcp_tool")
            .map(|selection| selection.capability_id)
            .collect::<HashSet<_>>();

        let mut installed = v_flex()
            .w_full()
            .border_1()
            .border_color(theme.border)
            .rounded_md();
        if servers.is_empty() {
            installed = installed.child(
                div()
                    .p_4()
                    .text_color(theme.muted_foreground)
                    .child("No MCP servers configured."),
            );
        } else {
            for server in servers {
                let server_tools = tools
                    .iter()
                    .filter(|tool| tool.server_id.as_deref() == Some(&server.id))
                    .count();
                let enabled = server.is_enabled;
                let server_for_toggle = server.clone();
                let server_toggle_id = server.id.clone();
                let server_for_refresh = server.clone();
                let server_refresh_id = server.id.clone();
                installed = installed.child(
                    h_flex()
                        .w_full()
                        .justify_between()
                        .items_center()
                        .p_3()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            v_flex()
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(server.name),
                                )
                                .child(div().text_sm().text_color(theme.muted_foreground).child(
                                    format!(
                                        "{} | {} tool(s) | {}",
                                        server.transport, server_tools, server.health_status
                                    ),
                                )),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(if enabled {
                                            gpui::green()
                                        } else {
                                            theme.muted_foreground
                                        })
                                        .child(if enabled { "Enabled" } else { "Disabled" }),
                                )
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "refresh-server-{}",
                                        server_refresh_id
                                    )))
                                    .small()
                                    .label("Refresh Tools")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let state = crate::AppState::global(cx);
                                        let actor_id = state.current_actor_id.clone();
                                        let db = state.db.clone();
                                        let gateway =
                                            crate::application::orchestration::tool_gateway::ToolExecutionGateway::new(db.clone());
                                        let allowed = gateway
                                            .can_execute_interactively(&actor_id, "mcp_discover");
                                        gateway.record_interactive_decision(
                                            &actor_id,
                                            "mcp_discover",
                                            allowed,
                                        );
                                        if !allowed {
                                            this.config_status =
                                                Some("Permission denied for MCP discovery.".to_string());
                                            cx.notify();
                                            return;
                                        }
                                        this.config_status =
                                            Some(format!("Refreshing '{}'...", server_for_refresh.name));
                                        cx.notify();
                                        let view = cx.entity().clone();
                                        let db = db.clone();
                                        let server = server_for_refresh.clone();
                                        cx.spawn(async move |_, cx| {
                                            let outcome =
                                                match crate::infrastructure::mcp::server::McpServer::discover_tools(&server).await {
                                                    Ok(discovered) => {
                                                        let discovered_ids = discovered
                                                            .iter()
                                                            .map(|tool| tool.id.clone())
                                                            .collect::<HashSet<_>>();
                                                        for mut old_tool in db
                                                            .list_mcp_tools()
                                                            .unwrap_or_default()
                                                            .into_iter()
                                                            .filter(|tool| tool.server_id.as_deref() == Some(&server.id))
                                                        {
                                                            if !discovered_ids.contains(&old_tool.id) {
                                                                old_tool.is_active = false;
                                                                let _ = db.upsert_mcp_tool(&old_tool);
                                                            }
                                                        }
                                                        for tool in &discovered {
                                                            let _ = db.upsert_mcp_tool(tool);
                                                        }
                                                        let mut updated_server = server.clone();
                                                        updated_server.health_status = "available".to_string();
                                                        updated_server.last_error = None;
                                                        updated_server.updated_at =
                                                            chrono::Utc::now().to_rfc3339();
                                                        let _ = db.upsert_mcp_server(&updated_server);
                                                        format!(
                                                            "Discovered {} tool(s) from '{}'.",
                                                            discovered.len(),
                                                            server.name
                                                        )
                                                    }
                                                    Err(error) => {
                                                        let mut updated_server = server.clone();
                                                        updated_server.health_status = "error".to_string();
                                                        updated_server.last_error =
                                                            Some(error.to_string());
                                                        updated_server.updated_at =
                                                            chrono::Utc::now().to_rfc3339();
                                                        let _ = db.upsert_mcp_server(&updated_server);
                                                        format!(
                                                            "Unable to refresh '{}': {}",
                                                            server.name, error
                                                        )
                                                    }
                                                };
                                            let _ = cx.update(|cx| {
                                                let _ = view.update(cx, |panel, cx| {
                                                    panel.config_status = Some(outcome);
                                                    cx.notify();
                                                });
                                            });
                                        })
                                        .detach();
                                    })),
                                )
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "toggle-server-{}",
                                        server_toggle_id
                                    )))
                                    .small()
                                    .label(if enabled { "Disable" } else { "Enable" })
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        let mut server = server_for_toggle.clone();
                                        server.is_enabled = !enabled;
                                        server.updated_at = chrono::Utc::now().to_rfc3339();
                                        let _ = crate::AppState::global(cx)
                                            .db
                                            .upsert_mcp_server(&server);
                                        cx.notify();
                                    })),
                                ),
                        ),
                );
            }
        }

        let mut catalog = v_flex()
            .w_full()
            .border_1()
            .border_color(theme.border)
            .rounded_md();
        for entry in Self::catalog_entries() {
            let entry_for_add = entry.clone();
            catalog = catalog.child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .p_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(entry.name),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(entry.description),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(format!(
                                        "{} {}",
                                        entry.command,
                                        entry.args.join(" ")
                                    )),
                            ),
                    )
                    .child(
                        Button::new(gpui::SharedString::from(format!(
                            "add-mcp-catalog-{}",
                            entry.id
                        )))
                        .small()
                        .label("Add")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let outcome = match Self::catalog_record(&entry_for_add) {
                                Ok(server) => {
                                    let status = server.health_status.clone();
                                    match crate::AppState::global(cx).db.upsert_mcp_server(&server)
                                    {
                                        Ok(()) => format!(
                                            "Added '{}' from catalog with status '{}'. Store required secrets, then refresh tools.",
                                            server.name, status
                                        ),
                                        Err(error) => {
                                            format!("Unable to add catalog server: {}", error)
                                        }
                                    }
                                }
                                Err(error) => error,
                            };
                            this.config_status = Some(outcome);
                            cx.notify();
                        })),
                    ),
            );
        }

        let mut tool_table = v_flex()
            .w_full()
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(
                h_flex()
                    .w_full()
                    .bg(theme.secondary)
                    .p(px(10.))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(div().w(px(220.)).font_bold().child("Tool"))
                    .child(div().w(px(180.)).font_bold().child("Server"))
                    .child(div().flex_1().font_bold().child("Description"))
                    .child(div().w(px(130.)).font_bold().child("LLM Context")),
            );
        if tools.is_empty() {
            tool_table = tool_table.child(div().p_4().child("No discovered tools."));
        } else {
            for tool in tools {
                let is_selected = selected_tool_ids.contains(&tool.id);
                let tool_id = tool.id.clone();
                let server_label = tool
                    .server_id
                    .clone()
                    .unwrap_or_else(|| "unassigned".to_string());
                tool_table = tool_table.child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .p(px(10.))
                        .border_b_1()
                        .border_color(theme.border)
                        .child(div().w(px(220.)).child(tool.name))
                        .child(
                            div()
                                .w(px(180.))
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(server_label),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_color(theme.muted_foreground)
                                .child(tool.description),
                        )
                        .child(
                            Button::new(gpui::SharedString::from(format!("select-{}", tool_id)))
                                .small()
                                .label(if is_selected { "Selected" } else { "Enable" })
                                .on_click(cx.listener(move |_this, _, _, cx| {
                                    let state = crate::AppState::global(cx);
                                    let now = chrono::Utc::now().to_rfc3339();
                                    let _ = state.db.upsert_capability_selection(
                                        &crate::core::models::CapabilitySelectionRecord {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            scope_kind: "default".to_string(),
                                            scope_id: String::new(),
                                            capability_kind: "mcp_tool".to_string(),
                                            capability_id: tool_id.clone(),
                                            enabled: !is_selected,
                                            selected_by: Some(state.current_actor_id.clone()),
                                            created_at: now.clone(),
                                            updated_at: now,
                                        },
                                    );
                                    cx.notify();
                                })),
                        ),
                );
            }
        }

        let config_status = self.config_status.clone();
        let config_muted_foreground = theme.muted_foreground;
        v_flex()
            .size_full()
            .bg(theme.background)
            .p_6()
            .gap_4()
            .overflow_y_scrollbar()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Installed MCP Servers"),
                    )
                    .child(
                        Button::new("store-mcp-secret")
                            .label("Store Secret")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let view = cx.entity().clone();
                                let reference_input = cx.new(|cx| {
                                    InputState::new(window, cx).placeholder("secret reference name")
                                });
                                let value_input = cx.new(|cx| {
                                    InputState::new(window, cx).placeholder("secret value")
                                });
                                let reference_for_dialog = reference_input.clone();
                                let value_for_dialog = value_input.clone();
                                window.open_dialog(cx, move |dialog, _window, _cx| {
                                    let reference_for_footer = reference_for_dialog.clone();
                                    let value_for_footer = value_for_dialog.clone();
                                    let view_for_footer = view.clone();
                                    dialog
                                        .title("Store MCP Secret")
                                        .w(px(500.))
                                        .child(
                                            v_form()
                                                .gap(px(12.))
                                                .py(px(8.))
                                                .child(
                                                    field()
                                                        .label("Reference")
                                                        .child(Input::new(&reference_for_dialog)),
                                                )
                                                .child(
                                                    field()
                                                        .label("Secret Value")
                                                        .child(Input::new(&value_for_dialog)),
                                                ),
                                        )
                                        .footer(move |_, _, _, _| {
                                            let reference = reference_for_footer.clone();
                                            let value = value_for_footer.clone();
                                            let view = view_for_footer.clone();
                                            vec![
                                                Button::new("cancel-mcp-secret")
                                                    .label("Cancel")
                                                    .on_click(|_, window, cx| window.close_dialog(cx))
                                                    .into_any_element(),
                                                Button::new("save-mcp-secret")
                                                    .primary()
                                                    .label("Save")
                                                    .on_click(move |_, window, cx| {
                                                        let reference =
                                                            reference.read(cx).text().to_string();
                                                        let value = value.read(cx).text().to_string();
                                                        if reference.trim().is_empty()
                                                            || value.is_empty()
                                                        {
                                                            let _ = view.update(cx, |panel, cx| {
                                                                panel.config_status = Some(
                                                                    "Secret reference and value are required."
                                                                        .to_string(),
                                                                );
                                                                cx.notify();
                                                            });
                                                            window.close_dialog(cx);
                                                            return;
                                                        }
                                                        window.close_dialog(cx);
                                                        let view = view.clone();
                                                        cx.spawn(async move |cx| {
                                                            let outcome = match crate::infrastructure::security::keychain::Keychain::new()
                                                                .await
                                                            {
                                                                Ok(keychain) => match keychain
                                                                    .set_secret(
                                                                        "agentforge-mcp",
                                                                        reference.trim(),
                                                                        &value,
                                                                    )
                                                                    .await
                                                                {
                                                                    Ok(()) => format!(
                                                                        "Stored secret://{} in OS credential storage.",
                                                                        reference.trim()
                                                                    ),
                                                                    Err(error) => format!(
                                                                        "Unable to store secret: {}",
                                                                        error
                                                                    ),
                                                                },
                                                                Err(error) => format!(
                                                                    "Unable to access OS credential storage: {}",
                                                                    error
                                                                ),
                                                            };
                                                            let _ = cx.update(|cx| {
                                                                let _ = view.update(cx, |panel, cx| {
                                                                    panel.config_status =
                                                                        Some(outcome);
                                                                    cx.notify();
                                                                });
                                                            });
                                                        })
                                                        .detach();
                                                    })
                                                    .into_any_element(),
                                            ]
                                        })
                                });
                                this.config_status = None;
                            })),
                    )
                    .child(
                        Button::new("open-mcp-config")
                            .primary()
                            .label("Open MCP Config")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let view = cx.entity().clone();
                                let config_input = cx.new(|cx| {
                                    let mut input =
                                        InputState::new(window, cx).placeholder("MCP JSON configuration");
                                    input.replace(
                                        "{\"mcpServers\":{\"github\":{\"command\":\"npx\",\"args\":[\"-y\",\"@modelcontextprotocol/server-github\"],\"env\":{\"GITHUB_TOKEN\":\"secret://github-token\"}}}}".to_string(),
                                        window,
                                        cx,
                                    );
                                    input
                                });
                                let config_input_for_dialog = config_input.clone();
                                window.open_dialog(cx, move |dialog, _window, _cx| {
                                    let config_input_for_footer = config_input_for_dialog.clone();
                                    let view_for_footer = view.clone();
                                    dialog
                                        .title("Configure MCP Server")
                                        .w(px(820.))
                                        .child(
                                            v_form()
                                                .gap(px(12.))
                                                .py(px(8.))
                                                .child(
                                                    field()
                                                        .label("Configuration JSON")
                                                        .child(Input::new(&config_input_for_dialog)),
                                                )
                                                .child(field().label("Credential policy").child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(config_muted_foreground)
                                                        .child("Use command/args for local stdio or serverUrl for remote. Credentials are accepted only through secret:// environment or header entries. Local stdio requires AGENTFORGE_MCP_SANDBOX_WRAPPER or remains blocked."),
                                                )),
                                        )
                                        .footer(move |_, _, _, _| {
                                            let input = config_input_for_footer.clone();
                                            let view = view_for_footer.clone();
                                            vec![
                                                Button::new("cancel-mcp-config")
                                                    .label("Cancel")
                                                    .on_click(|_, window, cx| window.close_dialog(cx))
                                                    .into_any_element(),
                                                Button::new("save-mcp-config")
                                                    .primary()
                                                    .label("Save")
                                                    .on_click(move |_, window, cx| {
                                                        let raw = input.read(cx).text().to_string();
                                                        let outcome = match Self::parse_server_config(&raw) {
                                                            Ok(servers) => {
                                                                let db = crate::AppState::global(cx).db.clone();
                                                                let mut failures = Vec::new();
                                                                for server in &servers {
                                                                    if let Err(error) = db.upsert_mcp_server(server) {
                                                                        failures.push(format!("{}: {}", server.name, error));
                                                                    }
                                                                }
                                                                if failures.is_empty() {
                                                                    format!("Configured {} server(s). Refresh each server to discover tools.", servers.len())
                                                                } else {
                                                                    format!("Unable to save server configuration: {}", failures.join("; "))
                                                                }
                                                            }
                                                            Err(error) => error,
                                                        };
                                                        let _ = view.update(cx, |panel, cx| {
                                                            panel.config_status = Some(outcome);
                                                            cx.notify();
                                                        });
                                                        window.close_dialog(cx);
                                                    })
                                                    .into_any_element(),
                                            ]
                                        })
                                });
                                this.config_status = None;
                            })),
                    ),
            )
            .child(installed)
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("MCP Catalog"),
            )
            .child(catalog)
            .when_some(config_status, |view, status| {
                view.child(
                    div()
                        .p_3()
                        .border_1()
                        .border_color(theme.border)
                        .text_sm()
                        .child(status),
                )
            })
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Tools Available To Chat"),
            )
            .child(tool_table)
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(format!(
                        "Selections are stored for future LLM contexts by actor {}. Direct process execution is disabled on this surface.",
                        actor_id
                    )),
            )
    }
}

impl EventEmitter<PanelEvent> for McpMarketplacePanel {}

#[cfg(test)]
mod tests {
    use super::McpMarketplacePanel;

    #[test]
    fn configuration_imports_multiple_servers() {
        let servers = McpMarketplacePanel::parse_server_config(
            r#"{"mcpServers":{"local":{"command":"node","args":["server.js"]},"remote":{"serverUrl":"https://mcp.example.test/api","headers":{"Authorization":"secret://remote-token"}}}}"#,
        )
        .expect("configuration should parse");
        assert_eq!(servers.len(), 2);
    }

    #[test]
    fn configuration_rejects_credentials_in_args_or_url() {
        let args = McpMarketplacePanel::parse_server_config(
            r#"{"mcpServers":{"bad":{"command":"node","args":["server.js","--token=raw-value"]}}}"#,
        );
        assert!(args.is_err());
        let url = McpMarketplacePanel::parse_server_config(
            r#"{"mcpServers":{"bad":{"serverUrl":"https://mcp.example.test/api?token=raw-value"}}}"#,
        );
        assert!(url.is_err());
    }
}
