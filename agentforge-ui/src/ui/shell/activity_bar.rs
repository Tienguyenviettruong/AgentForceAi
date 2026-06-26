use gpui::{
    div, px, App, ClickEvent, Context, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_component::{ActiveTheme as _, Icon, IconName};

pub enum ActivityBarEvent {
    Selected(SharedString),
}

pub struct ActivityBar {
    pub active_item: SharedString,
}

impl ActivityBar {
    pub fn new(_cx: &mut App) -> Self {
        Self {
            active_item: "skills".into(),
        }
    }

    fn render_icon(
        &mut self,
        id: &'static str,
        icon: impl IntoElement,
        is_active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let bg = if is_active {
            theme.accent.opacity(0.1)
        } else {
            theme.transparent
        };
        let fg = if is_active {
            theme.accent
        } else {
            theme.muted_foreground
        };

        div()
            .id(id)
            .w(px(48.))
            .h(px(48.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .bg(bg)
            .text_color(fg)
            .cursor_pointer()
            .hover(|style| style.bg(theme.secondary))
            .child(icon)
            .tooltip(move |window, cx| {
                let text = match id {
                    "teams" => "Teams",
                    "skills" => "Skills",
                    "knowledge" => "Knowledge",
                    "mcp_marketplace" => "MCP Marketplace",
                    "iflow_builder" => "iFlow Builder",
                    "research_notebook" => "Research Notebook",
                    "orchestration" => "Orchestration",
                    "settings" => "Settings",
                    "profile" => "Profile",
                    _ => id,
                };
                gpui_component::tooltip::Tooltip::new(text).build(window, cx)
            })
            .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                this.active_item = id.into();
                cx.emit(ActivityBarEvent::Selected(id.into()));
                cx.notify();
            }))
    }
}

impl EventEmitter<ActivityBarEvent> for ActivityBar {}

impl Render for ActivityBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .w(px(48.))
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_between()
            .py(px(8.))
            .bg(theme.background)
            .border_r(px(1.))
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .items_center()
                    // .child(
                    //     // Logo
                    //     div()
                    //         .w(px(40.))
                    //         .h(px(40.))
                    //         .flex()
                    //         .items_center()
                    //         .justify_center()
                    //         .mb(px(16.))
                    //         .text_color(theme.accent)
                    //         .child(IconName::Bot),
                    // )
                    .child(self.render_icon(
                        "teams",
                        Icon::empty().path("icons/teams.svg").size_4(),
                        self.active_item == "teams",
                        cx,
                    ))
                    .child(self.render_icon(
                        "skills",
                        Icon::empty().path("icons/skill.svg").size_4(),
                        self.active_item == "skills",
                        cx,
                    ))
                    .child(self.render_icon(
                        "knowledge",
                        Icon::empty().path("icons/brain.svg").size_4(),
                        self.active_item == "knowledge",
                        cx,
                    ))
                    .child(self.render_icon(
                        "mcp_marketplace",
                        Icon::empty().path("icons/Mcp.svg").size_4(),
                        self.active_item == "mcp_marketplace",
                        cx,
                    ))
                    .child(self.render_icon(
                        "iflow_builder",
                        Icon::empty().path("icons/flow.svg").size_4(),
                        self.active_item == "iflow_builder",
                        cx,
                    ))
                    .child(self.render_icon(
                        "research_notebook",
                        Icon::empty().path("icons/notebooklm.svg").size_4(),
                        self.active_item == "research_notebook",
                        cx,
                    ))
                    .child(self.render_icon(
                        "orchestration",
                        IconName::Settings2,
                        self.active_item == "orchestration",
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .items_center()
                    .child(self.render_icon(
                        "settings",
                        IconName::Settings,
                        self.active_item == "settings",
                        cx,
                    ))
                    .child(self.render_icon(
                        "profile",
                        IconName::CircleUser,
                        self.active_item == "profile",
                        cx,
                    )),
            )
    }
}
