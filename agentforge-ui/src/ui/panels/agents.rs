use crate::core::traits::database::DatabasePort;
use gpui::{
    div, px, App, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window, StatefulInteractiveElement,
};
use gpui_component::{
    dock::{Panel, PanelEvent, TitleStyle},
    ActiveTheme as _, button::{Button, ButtonVariants}, IconName, h_flex, v_flex,
};
use crate::db::Agent;

pub struct AgentsPanel {
    focus_handle: FocusHandle,
    agents: Vec<Agent>,
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

        let header = h_flex()
            .w_full()
            .h(px(56.))
            .px(px(24.))
            .items_center()
            .justify_end()
            .border_b(px(1.))
            .border_color(theme.border)
            .child(
                Button::new("create-agent")
                    .primary()
                    .icon(IconName::Plus)
                    .label("Create Agent")
                    .on_click(cx.listener(|_this, _, window, cx| {
                        let db = crate::AppState::global(cx).db.clone();
                        crate::ui::components::dialogs::open_new_agent_dialog(db, cx.entity().clone(), window, cx, |view: &mut AgentsPanel, cx: &mut Context<AgentsPanel>| view.reload(cx));
                    }))
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
                                .child(IconName::Bot)
                        )
                        .child(div().text_size(px(16.)).text_color(theme.muted_foreground).child("No agents exist."))
                        .child(
                            Button::new("create-agent-empty")
                                .primary()
                                .icon(IconName::Plus)
                                .label("Create Agent")
                                .on_click(cx.listener(|_this, _, window, cx| {
                                    let db = crate::AppState::global(cx).db.clone();
                                    crate::ui::components::dialogs::open_new_agent_dialog(db, cx.entity().clone(), window, cx, |view: &mut AgentsPanel, cx: &mut Context<AgentsPanel>| view.reload(cx));
                                }))
                        )
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
                        .flex_col()
                        .gap(px(12.))
                        .children(self.agents.iter().map(|agent| {
                            
                            // Parse config for role and details
                            let mut role = "Unassigned".to_string();
                            if let Some(config_str) = &agent.config {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(config_str) {
                                    if let Some(r) = val.get("role").and_then(|v| v.as_str()) {
                                        role = r.to_string();
                                    }
                                }
                            }

                            h_flex()
                                .w_full()
                                .p(px(16.))
                                .border(px(1.))
                                .border_color(theme.border)
                                .rounded_lg()
                                .bg(theme.secondary.opacity(0.3))
                                .justify_between()
                                .child(
                                    h_flex()
                                        .gap(px(16.))
                                        .child(
                                            div()
                                                .w(px(48.))
                                                .h(px(48.))
                                                .rounded_full()
                                                .bg(theme.primary.opacity(0.1))
                                                .text_color(theme.primary)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .child(IconName::Bot)
                                        )
                                        .child(
                                            v_flex()
                                                .gap(px(4.))
                                                .child(
                                                    h_flex()
                                                        .gap(px(8.))
                                                        .child(div().font_weight(gpui::FontWeight::BOLD).text_size(px(16.)).child(agent.name.clone()))
                                                        .child(
                                                            div()
                                                                .px(px(8.))
                                                                .py(px(2.))
                                                                .rounded_full()
                                                                .bg(if agent.status == "offline" { gpui::red().opacity(0.1) } else { gpui::green().opacity(0.1) })
                                                                .text_color(if agent.status == "offline" { gpui::red() } else { gpui::green() })
                                                                .text_size(px(12.))
                                                                .child(agent.status.clone())
                                                        )
                                                )
                                                .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(role))
                                                .child(div().text_size(px(12.)).text_color(theme.muted_foreground.opacity(0.7)).child(format!("Provider: {}", agent.provider)))
                                        )
                                )
                                .child(
                                    h_flex()
                                        .gap(px(8.))
                                        .child(Button::new(gpui::SharedString::from(format!("edit-{}", agent.id))).ghost().icon(IconName::Settings)
                                            .on_click({
                                                let agent_clone = agent.clone();
                                                let view = cx.entity().clone();
                                                cx.listener(move |this, _, window, cx| {
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
                                        .child(Button::new(gpui::SharedString::from(format!("delete-{}", agent.id))).ghost().icon(IconName::Delete)
                                            .on_click({
                                                let agent_id = agent.id.clone();
                                                cx.listener(move |this, _, _, cx| {
                                                    let db = crate::AppState::global(cx).db.clone();
                                                    let _ = db.delete_agent(&agent_id);
                                                    this.reload(cx);
                                                })
                                            }))
                                )
                        }))
                )
        }
    }
}

impl EventEmitter<PanelEvent> for AgentsPanel {}
