use crate::AppState;
use gpui::{
    div, px, App, Context, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window,
};
use gpui_component::{
    button::Button,
    dock::{Panel, PanelEvent, TitleStyle},
    h_flex, v_flex, ActiveTheme as _, Icon, IconName, Sizable,
};

pub struct ProfilePanel {
    focus_handle: FocusHandle,
}

impl ProfilePanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn stat_card(
        &self,
        label: &'static str,
        value: impl Into<String>,
        icon: IconName,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .min_w(px(150.))
            .flex_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .p_3()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(28.))
                            .h(px(28.))
                            .rounded_md()
                            .bg(theme.primary.opacity(0.12))
                            .text_color(theme.primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(icon).size(px(16.))),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.muted_foreground)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.foreground)
                                    .child(value.into()),
                            ),
                    ),
            )
    }

    fn info_row(
        &self,
        label: &'static str,
        value: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        h_flex()
            .w_full()
            .items_start()
            .justify_between()
            .gap_3()
            .py_2()
            .border_b_1()
            .border_color(theme.border.opacity(0.45))
            .child(
                div()
                    .w(px(120.))
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(12.))
                    .text_color(theme.foreground)
                    .child(value.into()),
            )
    }

    fn open_page(page: &'static str, cx: &mut Context<Self>) {
        let active_panel = AppState::global(cx).active_panel.clone();
        active_panel.update(cx, |panel, cx| {
            *panel = page.to_string();
            cx.notify();
        });
    }
}

impl Panel for ProfilePanel {
    fn panel_name(&self) -> &'static str {
        "Profile"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for ProfilePanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProfilePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let state = AppState::global(cx);
        let db = state.db.clone();
        let actor_id = state.current_actor_id.clone();
        let teams = db.list_teams().unwrap_or_default();
        let agents = db.list_agents().unwrap_or_default();
        let providers = db.list_providers().unwrap_or_default();
        let recent_workspaces = db.get_recent_workspaces().unwrap_or_default();
        let workspace = recent_workspaces
            .first()
            .cloned()
            .or_else(|| db.get_setting("workspace_local-user").ok().flatten())
            .unwrap_or_else(|| "No workspace configured".to_string());
        let mode = state
            .mode_manager
            .lock()
            .map(|manager| manager.current_mode().label().to_string())
            .unwrap_or_else(|_| "Unavailable".to_string());
        let theme_name = db
            .get_setting("theme")
            .ok()
            .flatten()
            .unwrap_or_else(|| "Default".to_string());
        let provider_ready = providers
            .iter()
            .filter(|provider| {
                provider
                    .api_key_ref
                    .as_deref()
                    .is_some_and(|key| !key.is_empty())
            })
            .count();

        div()
            .size_full()
            .bg(theme.background)
            .overflow_hidden()
            .child(
                v_flex()
                    .size_full()
                    .p_6()
                    .gap_5()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .w(px(46.))
                                            .h(px(46.))
                                            .rounded_full()
                                            .bg(theme.primary.opacity(0.14))
                                            .text_color(theme.primary)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(Icon::new(IconName::CircleUser).size(px(24.))),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_size(px(20.))
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .text_color(theme.foreground)
                                                    .child("Local Profile"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(actor_id.clone()),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("profile-open-settings")
                                            .small()
                                            .label("Settings")
                                            .on_click(cx.listener(|_, _, _window, cx| {
                                                Self::open_page("settings", cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("profile-open-teams")
                                            .small()
                                            .label("Teams")
                                            .on_click(cx.listener(|_, _, _window, cx| {
                                                Self::open_page("teams", cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("profile-open-orchestration")
                                            .small()
                                            .label("Orchestration")
                                            .on_click(cx.listener(|_, _, _window, cx| {
                                                Self::open_page("orchestration", cx);
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .gap_3()
                            .flex_wrap()
                            .child(self.stat_card("Teams", teams.len().to_string(), IconName::Building2, cx))
                            .child(self.stat_card("Agents", agents.len().to_string(), IconName::Bot, cx))
                            .child(self.stat_card(
                                "Providers ready",
                                format!("{}/{}", provider_ready, providers.len()),
                                IconName::Globe,
                                cx,
                            ))
                            .child(self.stat_card("Mode", mode.clone(), IconName::Settings2, cx)),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .items_start()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.background)
                                    .p_4()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.foreground)
                                            .child("Account Context"),
                                    )
                                    .child(self.info_row("Actor", actor_id, cx))
                                    .child(self.info_row("Workspace", workspace, cx))
                                    .child(self.info_row("Theme", theme_name, cx))
                                    .child(self.info_row("Mode", mode, cx)),
                            )
                            .child(
                                v_flex()
                                    .w(px(320.))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.background)
                                    .p_4()
                                    .gap_3()
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.foreground)
                                            .child("Profile Actions"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child("Use this area for local identity, workspace, provider readiness, and agent/team entry points."),
                                    )
                                    .child(
                                        Button::new("profile-manage-providers")
                                            .small()
                                            .label("Manage Providers")
                                            .on_click(cx.listener(|_, _, _window, cx| {
                                                Self::open_page("settings", cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("profile-view-agents")
                                            .small()
                                            .label("View Agents")
                                            .on_click(cx.listener(|_, _, _window, cx| {
                                                Self::open_page("agents", cx);
                                            })),
                                    ),
                            ),
                    ),
            )
    }
}

impl EventEmitter<PanelEvent> for ProfilePanel {}
