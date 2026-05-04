use gpui::EventEmitter;
use gpui::{div, px, App, Context, Focusable, IntoElement, ParentElement, Render, Styled, Window};
use gpui_component::dock::PanelEvent;
use gpui_component::dock::{Panel, TitleStyle};
use gpui_component::{
    h_flex,
    v_flex,
    theme::ActiveTheme,
};

pub struct McpMarketplacePanel {
    focus_handle: gpui::FocusHandle,
}

impl McpMarketplacePanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
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
                                    .child(div().w(px(90.)).font_weight(gpui::FontWeight::BOLD).child("Status")),
                            )
                            .children(tools.into_iter().map(|t| {
                                let status = if t.is_active { "active" } else { "inactive" };
                                h_flex()
                                    .w_full()
                                    .p(px(10.))
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .child(div().w(px(220.)).child(t.name))
                                    .child(div().w(px(90.)).child(t.version))
                                    .child(div().flex_1().text_color(theme.muted_foreground).child(t.description))
                                    .child(div().w(px(90.)).child(status))
                            })),
                    )
            )
    }
}

impl EventEmitter<PanelEvent> for McpMarketplacePanel {}
