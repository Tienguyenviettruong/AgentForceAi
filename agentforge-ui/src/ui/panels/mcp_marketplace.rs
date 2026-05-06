use gpui::EventEmitter;
use gpui::{div, px, App, AppContext, Context, Focusable, IntoElement, ParentElement, Render, Styled, Window};
use gpui::prelude::FluentBuilder;
use gpui_component::dock::PanelEvent;
use gpui_component::dock::{Panel, TitleStyle};
use gpui_component::{
    h_flex,
    input::{Input, InputState},
    button::Button,
    button::ButtonVariants,
    form::{field, v_form},
    v_flex,
    theme::ActiveTheme,
    WindowExt,
    Sizable,
    Disableable,
};
use std::collections::HashMap;
use std::process::Stdio;

pub struct McpMarketplacePanel {
    focus_handle: gpui::FocusHandle,
    outputs: HashMap<String, String>,
    running: HashMap<String, bool>,
}

impl McpMarketplacePanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            outputs: HashMap::new(),
            running: HashMap::new(),
        }
    }

    fn role_id() -> &'static str {
        "admin-role-123"
    }

    fn check_allowed(
        db: &std::sync::Arc<dyn crate::core::traits::database::DatabasePort>,
        tool_name: &str,
    ) -> bool {
        db.check_role_permission(Self::role_id(), &format!("mcp:execute:{}", tool_name))
            .unwrap_or(false)
    }

    fn execute_tool(
        &mut self,
        tool: crate::infrastructure::mcp::registry::McpTool,
        payload: String,
        cx: &mut Context<Self>,
    ) {
        if self.running.get(&tool.id).copied().unwrap_or(false) {
            return;
        }
        self.running.insert(tool.id.clone(), true);
        cx.notify();

        let view = cx.entity().clone();
        let db = crate::AppState::global(cx).db.clone();
        let allowed = Self::check_allowed(&db, &tool.name);
        cx.spawn(async move |_, cx| {
            let result = if !allowed {
                "Permission denied.".to_string()
            } else {
                let output = std::process::Command::new(&tool.command)
                    .args(tool.args.clone())
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .and_then(|mut child| {
                        if let Some(mut stdin) = child.stdin.take() {
                            use std::io::Write;
                            let _ = stdin.write_all(payload.as_bytes());
                        }
                        child.wait_with_output()
                    });

                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        let mut s = String::new();
                        s.push_str(&format!("exit_code={}\n", out.status.code().unwrap_or(-1)));
                        if !stdout.trim().is_empty() {
                            s.push_str("\nSTDOUT:\n");
                            s.push_str(&stdout);
                        }
                        if !stderr.trim().is_empty() {
                            s.push_str("\nSTDERR:\n");
                            s.push_str(&stderr);
                        }
                        if stdout.trim().is_empty() && stderr.trim().is_empty() {
                            s.push_str("\n(no output)\n");
                        }
                        s
                    }
                    Err(e) => format!("Failed to run tool: {}", e),
                }
            };

            let _ = cx.update(|cx| {
                let _ = view.update(cx, |this: &mut Self, cx| {
                    this.outputs.insert(tool.id.clone(), result);
                    this.running.insert(tool.id.clone(), false);
                    cx.notify();
                });
            });
        })
        .detach();
    }
}

impl Panel for McpMarketplacePanel {
    fn panel_name(&self) -> &'static str {
        "MCP Marketplace"
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
        let bg = theme.background;
        let db = crate::AppState::global(cx).db.clone();
        let tools = db.list_mcp_tools().unwrap_or_default();
        
        v_flex()
            .size_full()
            .bg(theme.background)
            .child(
                v_flex()
                    .flex_1()
                    .p_6()
                    .gap_4()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(format!("Available MCP Tools ({})", tools.len())),
                    )
                    .child(
                        v_flex()
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
                                    .child(div().w(px(220.)).font_weight(gpui::FontWeight::BOLD).child("Name"))
                                    .child(div().w(px(90.)).font_weight(gpui::FontWeight::BOLD).child("Version"))
                                    .child(div().flex_1().font_weight(gpui::FontWeight::BOLD).child("Description"))
                                    .child(div().w(px(110.)).font_weight(gpui::FontWeight::BOLD).child("Permission"))
                                    .child(div().w(px(140.)).font_weight(gpui::FontWeight::BOLD).child("Actions")),
                            )
                            .children(tools.into_iter().map(|t| {
                                let allowed = Self::check_allowed(&db, &t.name);
                                let perm_label = if allowed { "allowed" } else { "denied" };
                                let tool_id = t.id.clone();
                                let has_output = self.outputs.contains_key(&tool_id);
                                let is_running = self.running.get(&tool_id).copied().unwrap_or(false);
                                let tool = t.clone();
                                h_flex()
                                    .w_full()
                                    .p(px(10.))
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .child(div().w(px(220.)).child(t.name))
                                    .child(div().w(px(90.)).child(t.version))
                                    .child(div().flex_1().text_color(theme.muted_foreground).child(t.description))
                                    .child(div().w(px(110.)).child(perm_label))
                                    .child(
                                        h_flex()
                                            .w(px(140.))
                                            .gap(px(8.))
                                            .justify_end()
                                            .child(
                                                Button::new(gpui::SharedString::from(format!("run-{}", tool_id)))
                                                    .small()
                                                    .label(if is_running { "Running..." } else { "Run" })
                                                    .disabled(!allowed || is_running)
                                                    .on_click(cx.listener(move |_, _, window, cx| {
                                                        let view = cx.entity().clone();
                                                        let payload_input = cx.new(|cx| {
                                                            let mut st = InputState::new(window, cx).placeholder("JSON payload (optional)");
                                                            st.replace("{}".to_string(), window, cx);
                                                            st
                                                        });
                                                        let tool2 = tool.clone();
                                                        let payload_input2 = payload_input.clone();
                                                        window.open_dialog(cx, {
                                                            let view2 = view.clone();
                                                            let tool3 = tool2.clone();
                                                            let payload_input3 = payload_input2.clone();
                                                            move |dialog, _window, _cx| {
                                                                let payload_input4 = payload_input3.clone();
                                                                let view3 = view2.clone();
                                                                let tool4 = tool3.clone();
                                                                dialog
                                                                    .title("Run MCP Tool")
                                                                    .w(px(520.))
                                                                    .child(
                                                                        v_form()
                                                                            .gap(px(12.))
                                                                            .py(px(8.))
                                                                            .child(field().label("Tool").child(div().child(tool4.name.clone())))
                                                                            .child(field().label("Command").child(div().child(format!("{} {}", tool4.command, tool4.args.join(" ")))))
                                                                            .child(field().label("Payload").child(Input::new(&payload_input4))),
                                                                    )
                                                                    .footer(move |_, _, _, _| {
                                                                        let view4 = view3.clone();
                                                                        let tool5 = tool4.clone();
                                                                        let payload_input5 = payload_input4.clone();
                                                                        vec![
                                                                            Button::new("cancel-run-mcp")
                                                                                .label("Cancel")
                                                                                .on_click(|_, window, cx| window.close_dialog(cx))
                                                                                .into_any_element(),
                                                                            Button::new("exec-run-mcp")
                                                                                .primary()
                                                                                .label("Execute")
                                                                                .on_click(move |_, window, cx| {
                                                                                    let payload = payload_input5.read(cx).text().to_string();
                                                                                    view4.update(cx, |this: &mut Self, cx| {
                                                                                        this.execute_tool(tool5.clone(), payload, cx);
                                                                                    });
                                                                                    window.close_dialog(cx);
                                                                                })
                                                                                .into_any_element(),
                                                                        ]
                                                                    })
                                                            }
                                                        });
                                                    }))
                                            )
                                            .when(has_output, |c| {
                                                c.child(
                                                    Button::new(gpui::SharedString::from(format!("out-{}", tool_id)))
                                                        .small()
                                                        .label("Output")
                                                        .on_click(cx.listener({
                                                            let tool_id = tool_id.clone();
                                                            move |this, _, window, cx| {
                                                                let output = std::sync::Arc::new(
                                                                    this.outputs.get(&tool_id).cloned().unwrap_or_default(),
                                                                );
                                                                window.open_dialog(cx, {
                                                                    let output = output.clone();
                                                                    move |dialog, _window, _cx| {
                                                                        dialog
                                                                            .title("MCP Output")
                                                                            .w(px(720.))
                                                                            .child(div().p_4().bg(bg).child(div().text_sm().child((*output).clone())))
                                                                            .footer(|_, _, _, _| {
                                                                                vec![Button::new("close-out").label("Close").on_click(|_, window, cx| window.close_dialog(cx)).into_any_element()]
                                                                            })
                                                                    }
                                                                });
                                                            }
                                                        })),
                                                )
                                            })
                                    )
                            })),
                    )
            )
    }
}

impl EventEmitter<PanelEvent> for McpMarketplacePanel {}
