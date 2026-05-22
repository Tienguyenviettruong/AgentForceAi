use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, Context, EventEmitter, Focusable, IntoElement, ParentElement, Render, Styled,
    Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dock::{Panel, PanelEvent, TitleStyle};
use gpui_component::theme::ActiveTheme;
use gpui_component::{h_flex, v_flex, WindowExt};

pub struct OrchestrationPanel {
    focus_handle: gpui::FocusHandle,
    active_tab: String,
    current_mode: String,
}

impl OrchestrationPanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            active_tab: "Dashboard".to_string(),
            current_mode: "Human Interaction".to_string(),
        }
    }
}

impl Panel for OrchestrationPanel {
    fn panel_name(&self) -> &'static str {
        "Orchestration"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for OrchestrationPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OrchestrationPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .p_4()
            .child(
                h_flex()
                    .gap_2()
                    .child(self.render_tab("Dashboard", cx))
                    .child(self.render_tab("Tracking", cx))
                    .child(self.render_tab("Logs", cx))
                    .child(self.render_tab("Governance", cx))
                    .child(self.render_tab("Mode Transition", cx)),
            )
            .child(v_flex().flex_1().w_full().overflow_hidden().child(
                match self.active_tab.as_str() {
                    "Dashboard" => self.render_dashboard(cx).into_any_element(),
                    "Tracking" => self.render_tracking(cx).into_any_element(),
                    "Logs" => self.render_logs(cx).into_any_element(),
                    "Governance" => self.render_governance(cx).into_any_element(),
                    "Mode Transition" => self.render_mode_transition(cx).into_any_element(),
                    _ => div().child("Unknown Tab").into_any_element(),
                },
            ))
    }
}

impl OrchestrationPanel {
    fn render_tab(&self, name: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.active_tab == name;
        let tab_name = name.to_string();

        Button::new(name)
            .label(name.to_string())
            .when(is_active, |b| b.primary())
            .when(!is_active, |b| b.ghost())
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_tab = tab_name.clone();
                cx.notify();
            }))
    }

    fn render_dashboard(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        v_flex()
            .size_full()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Orchestration Dashboard"),
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .p_4()
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Active Runs"),
                            )
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("0"),
                            ),
                    )
                    .child(
                        v_flex()
                            .p_4()
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Success Rate"),
                            )
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("0%"),
                            ),
                    )
                    .child(
                        v_flex()
                            .p_4()
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Agents Utilized"),
                            )
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("0"),
                            ),
                    )
                    .child(
                        v_flex()
                            .p_4()
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Tasks Processed"),
                            )
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("0"),
                            ),
                    ),
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .mt_4()
                    .child("Recent Orchestration Runs"),
            )
            .child(v_flex().gap_2().children(vec![].into_iter().map(
                |(name, mode, status, time): (&str, &str, &str, &str)| {
                    h_flex()
                        .justify_between()
                        .p_3()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            v_flex()
                                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(name))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(mode),
                                ),
                        )
                        .child(
                            v_flex().items_end().child(div().child(status)).child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(time),
                            ),
                        )
                        .into_any_element()
                },
            )))
    }

    fn render_tracking(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        v_flex().size_full().gap_4().child(
            v_flex()
                .flex_1()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.secondary.opacity(0.3))
                .gap_3()
                .child(
                    h_flex()
                        .gap_4()
                        .p_2()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .w(px(200.))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child("Task Name"),
                        )
                        .child(
                            div()
                                .w(px(100.))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child("Agent"),
                        )
                        .child(
                            div()
                                .w(px(100.))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child("Status"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .font_weight(gpui::FontWeight::BOLD)
                                .child("Timeline / Progress"),
                        ),
                )
                .child(self.render_gantt_row(
                    "Task 1: Requirements Analysis",
                    "Product Owner",
                    "Completed",
                    0.0,
                    0.4,
                    theme.success,
                    cx,
                ))
                .child(self.render_gantt_row(
                    "Task 2: UI Design & Mockup",
                    "UX Designer",
                    "Running",
                    0.4,
                    0.3,
                    theme.primary,
                    cx,
                ))
                .child(self.render_gantt_row(
                    "Task 3: Backend API Setup",
                    "Software Engineer",
                    "Pending",
                    0.7,
                    0.3,
                    theme.border,
                    cx,
                )),
        )
    }

    fn render_gantt_row(
        &self,
        task: &str,
        agent: &str,
        status: &str,
        start_pct: f32,
        width_pct: f32,
        color: gpui::Hsla,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        h_flex()
            .gap_4()
            .p_2()
            .items_center()
            .child(div().w(px(200.)).child(task.to_string()))
            .child(div().w(px(100.)).child(agent.to_string()))
            .child(div().w(px(100.)).text_sm().child(status.to_string()))
            .child(
                div()
                    .flex_1()
                    .h(px(20.))
                    .rounded_sm()
                    .bg(theme.secondary)
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(gpui::relative(start_pct))
                            .w(gpui::relative(width_pct))
                            .rounded_sm()
                            .bg(color),
                    ),
            )
    }

    fn render_logs(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        v_flex()
            .size_full()
            .gap_4()
            .child(
                v_flex()
                    .flex_1()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .gap_1()
                    .children(vec![].into_iter().map(|(msg, color): (&str, gpui::Hsla)| {
                        div()
                            .text_sm()
                            .font_family("Courier New")
                            .text_color(color)
                            .child(msg.to_string())
                    })),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("export-logs").label("Export Logs").ghost())
                    .child(Button::new("clear-logs").label("Clear Logs")),
            )
    }

    fn render_governance(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        v_flex().size_full().gap_4().child(
            v_flex().flex_1().gap_4().children(vec![].into_iter().map(
                |(title, desc, enabled): (&str, &str, bool)| {
                    h_flex()
                        .p_4()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.secondary)
                        .justify_between()
                        .items_center()
                        .child(
                            v_flex()
                                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(title))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(desc),
                                ),
                        )
                        .child(div().w(px(40.)).h(px(20.)).rounded_full().bg(if enabled {
                            theme.primary
                        } else {
                            theme.muted_foreground
                        }))
                },
            )),
        )
    }

    fn render_setting_row(&self, label: &str, value: &str, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        h_flex()
            .justify_between()
            .items_center()
            .p_2()
            .border_b_1()
            .border_color(theme.border)
            .child(div().child(label.to_string()))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.primary)
                    .child(value.to_string()),
            )
    }

    fn render_mode_transition(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        v_flex().size_full().gap_4().child(
            v_flex()
                .flex_1()
                .gap_4()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.background)
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("Trigger Manual Transition"),
                        )
                        .child(
                            div()
                                .text_color(theme.primary)
                                .child(format!("Current Mode: {}", self.current_mode)),
                        ),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("btn-transition-autonomous")
                                .primary()
                                .label("Autonomous Mode")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_transition_dialog("Autonomous", window, cx);
                                })),
                        )
                        .child(
                            Button::new("btn-transition-human")
                                .label("Human Interaction")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_transition_dialog("Human Interaction", window, cx);
                                })),
                        ),
                )
                .child(
                    v_flex()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .p_4()
                        .child(div().mb_2().child("Safety Constraints"))
                        .child(self.render_setting_row("Safety Check Delay", "500ms", cx))
                        .child(self.render_setting_row("Max Cost Limit", "$10.00 / hour", cx))
                        .child(self.render_setting_row("Auto-Fallback", "Enabled", cx)),
                ),
        )
    }

    fn open_transition_dialog(
        &self,
        target_mode: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().clone();
        let target_mode_owned = target_mode.to_string();

        window.open_dialog(cx, move |dialog, _window, cx| {
            let view_save = view.clone();
            let target_mode_save = target_mode_owned.clone();
            let theme = cx.theme().clone();

            dialog
                .title("Mode Transition Safety Check")
                .w(px(500.))
                .child(
                    v_flex()
                        .gap_4()
                        .py_4()
                        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(format!("Target Mode: {}", target_mode_save)))
                        .child(div().text_sm().child("The following safety checks must pass before transitioning:"))
                        .child(
                            v_flex().gap_2()
                                .child(h_flex().gap_2().items_center()
                                    .child(div().w_4().h_4().rounded_full().bg(theme.primary))
                                    .child(div().text_sm().child("No blocking task dependencies"))
                                )
                                .child(h_flex().gap_2().items_center()
                                    .child(div().w_4().h_4().rounded_full().bg(theme.primary))
                                    .child(div().text_sm().child("Agents in idle/ready state"))
                                )
                                .child(h_flex().gap_2().items_center()
                                    .child(div().w_4().h_4().rounded_full().bg(theme.primary))
                                    .child(div().text_sm().child("Governance policy validated"))
                                )
                                .child(h_flex().gap_2().items_center()
                                    .child(div().w_4().h_4().rounded_full().bg(theme.accent))
                                    .child(div().text_sm().child("Token budget warning: 80% used"))
                                )
                        )
                        .child(
                            div().p_3().rounded_md().bg(theme.primary.opacity(0.2)).border_1().border_color(theme.primary)
                                .child(div().text_sm().text_color(theme.primary).child("Warning: Transitioning to Autonomous mode will automatically consume token budget without human approval."))
                        )
                )
                .footer({
                    let view_save = view_save.clone();
                    let target_mode_save = target_mode_save.clone();

                    move |_, _, _, _| {
                        let view_save2 = view_save.clone();
                        let target_mode_save2 = target_mode_save.clone();

                        vec![
                            Button::new("cancel-transition")
                                .label("Cancel")
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                })
                                .into_any_element(),
                            Button::new("confirm-transition")
                                .primary()
                                .label("Confirm Transition")
                                .on_click({
                                    let view_save3 = view_save2.clone();
                                    let target_mode_save3 = target_mode_save2.clone();

                                    move |_ev, window, cx| {
                                        view_save3.update(cx, |this, cx| {
                                            this.current_mode = target_mode_save3.clone();
                                            cx.notify();
                                        });
                                        window.close_dialog(cx);
                                        use gpui_component::notification::NotificationType;
                                        window.push_notification(
                                            (NotificationType::Success, "Mode transition successful."),
                                            cx,
                                        );
                                    }
                                })
                                .into_any_element(),
                        ]
                    }
                })
        });
    }
}

impl EventEmitter<PanelEvent> for OrchestrationPanel {}
