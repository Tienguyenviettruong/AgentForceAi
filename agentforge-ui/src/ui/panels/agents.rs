use crate::db::Agent;
use chrono::Utc;
use gpui::{
    div, px, App, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    dock::{Panel, PanelEvent, TitleStyle},
    h_flex,
    switch::Switch,
    v_flex, ActiveTheme as _, IconName, Sizable,
};

pub struct AgentsPanel {
    focus_handle: FocusHandle,
    agents: Vec<Agent>,
}

fn single_line_display(value: impl AsRef<str>, fallback: &str) -> String {
    let text = value
        .as_ref()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        fallback.to_string()
    } else {
        text
    }
}

impl AgentsPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        let agents = db.list_agents().unwrap_or_default();

        Self {
            focus_handle: cx.focus_handle(),
            agents,
        }
    }

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let db = crate::AppState::global(cx).db.clone();
        if let Ok(agents) = db.list_agents() {
            self.agents = agents;
        }
        cx.notify();
    }
}

impl Panel for AgentsPanel {
    fn panel_name(&self) -> &'static str {
        "Agents"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for AgentsPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AgentsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let total_agents = self.agents.len();
        let online_agents = self
            .agents
            .iter()
            .filter(|agent| agent.status.to_lowercase() != "offline")
            .count();
        let provider_count = self
            .agents
            .iter()
            .map(|agent| agent.provider.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .len();

        let header = h_flex()
            .w_full()
            .h(px(72.))
            .px(px(24.))
            .items_center()
            .justify_between()
            .border_b(px(1.))
            .border_color(theme.border)
            .child(
                v_flex()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.foreground)
                            .child("Agent Management"),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(theme.secondary)
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{} agents", total_agents)),
                            )
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(gpui::green().opacity(0.12))
                                    .text_size(px(12.))
                                    .text_color(gpui::green())
                                    .child(format!("{} online", online_agents)),
                            )
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(theme.secondary)
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{} providers", provider_count)),
                            ),
                    ),
            )
            .child(
                Button::new("create-agent")
                    .primary()
                    .icon(IconName::Plus)
                    .label("Create Agent")
                    .on_click(cx.listener(|_this, _, window, cx| {
                        let db = crate::AppState::global(cx).db.clone();
                        crate::ui::components::dialogs::open_new_agent_dialog(
                            db,
                            cx.entity().clone(),
                            window,
                            cx,
                            |view: &mut AgentsPanel, cx: &mut Context<AgentsPanel>| view.reload(cx),
                        );
                    })),
            );

        if self.agents.is_empty() {
            v_flex()
                .size_full()
                .bg(theme.background)
                .child(header)
                .child(
                    v_flex()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .gap(px(16.))
                        .child(
                            div()
                                .w(px(64.))
                                .h(px(64.))
                                .rounded_full()
                                .bg(theme.secondary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(theme.muted_foreground)
                                .child(IconName::Bot),
                        )
                        .child(
                            div()
                                .text_size(px(16.))
                                .text_color(theme.muted_foreground)
                                .child("No agents exist."),
                        )
                        .child(
                            Button::new("create-agent-empty")
                                .primary()
                                .icon(IconName::Plus)
                                .label("Create Agent")
                                .on_click(cx.listener(|_this, _, window, cx| {
                                    let db = crate::AppState::global(cx).db.clone();
                                    crate::ui::components::dialogs::open_new_agent_dialog(
                                        db,
                                        cx.entity().clone(),
                                        window,
                                        cx,
                                        |view: &mut AgentsPanel, cx: &mut Context<AgentsPanel>| {
                                            view.reload(cx)
                                        },
                                    );
                                })),
                        ),
                )
        } else {
            v_flex()
                .size_full()
                .bg(theme.background)
                .child(header)
                .child(
                    div()
                        .flex_1()
                        .id("agents-scroll")
                        .overflow_y_scroll()
                        .p(px(24.))
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .items_start()
                        .content_start()
                        .gap(px(16.))
                        .children(self.agents.iter().map(|agent| {

                            let agent_name = single_line_display(&agent.name, "Unnamed agent");
                            let status_label = single_line_display(&agent.status, "offline");
                            let provider = single_line_display(&agent.provider, "unconfigured");
                            let role = single_line_display(agent.profile_position(), "No role");
                            let details = single_line_display(agent
                                .profile_details()
                                .unwrap_or_else(|| "No profile details".to_string()), "No profile details");
                            let is_offline = agent.status.to_lowercase() == "offline";
                            let is_online = !is_offline;
                            let status_color = if is_offline { gpui::red() } else { gpui::green() };

                            v_flex()
                                .w(px(336.))
                                .h(px(168.))
                                .p(px(16.))
                                .border(px(1.))
                                .border_color(theme.border)
                                .rounded(px(8.))
                                .bg(theme.secondary.opacity(0.22))
                                .hover(|style| {
                                    style
                                        .bg(theme.secondary.opacity(0.34))
                                        .border_color(theme.primary.opacity(0.45))
                                })
                                .justify_between()
                                .child(
                                    h_flex()
                                        .w_full()
                                        .items_start()
                                        .justify_between()
                                        .child(
                                            h_flex()
                                                .gap(px(12.))
                                                .min_w_0()
                                                .child(
                                                    div()
                                                        .w(px(44.))
                                                        .h(px(44.))
                                                        .rounded_full()
                                                        .bg(theme.primary.opacity(0.12))
                                                        .text_color(theme.primary)
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .child(IconName::Bot)
                                                )
                                                .child(
                                                    v_flex()
                                                        .min_w_0()
                                                        .gap(px(4.))
                                                        .child(
                                                            h_flex()
                                                                .min_w_0()
                                                                .gap(px(8.))
                                                                .child(
                                                                    div()
                                                                        .min_w_0()
                                                                        .truncate()
                                                                        .font_weight(gpui::FontWeight::BOLD)
                                                                        .text_size(px(15.))
                                                                        .text_color(theme.foreground)
                                                                        .child(agent_name)
                                                                )
                                                                .child(
                                                                    div()
                                                                        .flex_none()
                                                                        .px(px(7.))
                                                                        .py(px(2.))
                                                                        .rounded(px(4.))
                                                                        .bg(status_color.opacity(0.12))
                                                                        .text_color(status_color)
                                                                        .text_size(px(11.))
                                                                        .child(status_label)
                                                                )
                                                        )
                                                        .child(
                                                            div()
                                                                .truncate()
                                                                .text_size(px(13.))
                                                                .text_color(theme.muted_foreground)
                                                                .child(role)
                                                        )
                                                )
                                        )
                                        .child(
                                            h_flex()
                                                .flex_none()
                                                .gap(px(4.))
                                                .items_center()
                                                .child(
                                                    Switch::new(gpui::SharedString::from(format!(
                                                        "status-{}",
                                                        agent.id
                                                    )))
                                                        .checked(is_online)
                                                        .tooltip(if is_online {
                                                            "Set offline"
                                                        } else {
                                                            "Set online"
                                                        })
                                                        .on_click({
                                                            let agent_to_update = agent.clone();
                                                            let view = cx.entity().clone();
                                                            move |checked: &bool, _window, cx| {
                                                                let db = crate::AppState::global(cx).db.clone();
                                                                let mut updated_agent = agent_to_update.clone();
                                                                updated_agent.status = if *checked {
                                                                    "online".to_string()
                                                                } else {
                                                                    "offline".to_string()
                                                                };
                                                                updated_agent.updated_at = Utc::now().to_rfc3339();
                                                                let _ = db.insert_agent(&updated_agent);
                                                                let _ = view.update(cx, |this, cx| this.reload(cx));
                                                            }
                                                        }),
                                                )
                                                .child(Button::new(gpui::SharedString::from(format!("edit-{}", agent.id))).ghost().small().compact().icon(IconName::Settings)
                                                    .on_click({
                                                        let agent_clone = agent.clone();
                                                        let view = cx.entity().clone();
                                                        cx.listener(move |_this, _, window, cx| {
                                                            let db = crate::AppState::global(cx).db.clone();
                                                            crate::ui::components::dialogs::open_edit_agent_dialog(
                                                                db,
                                                                agent_clone.clone(),
                                                                view.clone(),
                                                                window,
                                                                cx,
                                                                |this: &mut Self, cx| {
                                                                    this.reload(cx);
                                                                },
                                                            );
                                                        })
                                                    }))
                                                .child(Button::new(gpui::SharedString::from(format!("delete-{}", agent.id))).ghost().small().compact().icon(IconName::Delete)
                                                    .on_click({
                                                        let agent_id = agent.id.clone();
                                                        cx.listener(move |this, _, _, cx| {
                                                            let db = crate::AppState::global(cx).db.clone();
                                                            let _ = db.delete_agent(&agent_id);
                                                            this.reload(cx);
                                                        })
                                                    }))
                                        )
                                )
                                .child(
                                    v_flex()
                                        .w_full()
                                        .gap(px(8.))
                                        .child(
                                            h_flex()
                                                .justify_between()
                                                .gap(px(12.))
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(theme.muted_foreground)
                                                        .child("Provider")
                                                )
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .truncate()
                                                        .text_size(px(12.))
                                                        .text_color(theme.foreground)
                                                        .child(provider)
                                                )
                                        )
                                        .child(div().w_full().h(px(1.)).bg(theme.border.opacity(0.65)))
                                        .child(
                                            h_flex()
                                                .justify_between()
                                                .gap(px(12.))
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(theme.muted_foreground)
                                                        .child("Details")
                                                )
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .truncate()
                                                        .text_size(px(12.))
                                                        .text_color(theme.foreground)
                                                        .child(details)
                                                )
                                        )
                                )
                        }))
                )
        }
    }
}

impl EventEmitter<PanelEvent> for AgentsPanel {}
