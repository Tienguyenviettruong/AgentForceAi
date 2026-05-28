use gpui::{div, App, Context, IntoElement, ParentElement, Render, Styled, Window};
use gpui_component::StyledExt;
use gpui_component::{h_flex, theme::ActiveTheme, v_flex};

#[allow(dead_code)]
pub struct UsageAnalytics {
    focus_handle: gpui::FocusHandle,
    total_tasks: usize,
    total_tokens: usize,
    active_agents: usize,
}

impl UsageAnalytics {
    pub fn new(cx: &mut App) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        let total_tasks = db.get_total_tasks_completed().unwrap_or(0);
        let total_tokens = db.get_total_daily_tokens().unwrap_or(0);
        let active_agents = db.get_active_agents_count().unwrap_or(0);

        Self {
            focus_handle: cx.focus_handle(),
            total_tasks,
            total_tokens,
            active_agents,
        }
    }
}

impl Render for UsageAnalytics {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .bg(theme.background)
            .child(
                h_flex().w_full().items_center().child(
                    div()
                        .text_xl()
                        .text_color(theme.foreground)
                        .child("Usage Analytics"),
                ),
            )
            .child(
                v_flex().w_full().gap_4().child(
                    h_flex()
                        .w_full()
                        .gap_4()
                        .child(
                            v_flex()
                                .p_4()
                                .border_1()
                                .border_color(theme.border)
                                .rounded_md()
                                .bg(theme.secondary)
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child("Total Tasks Completed"),
                                )
                                .child(
                                    div()
                                        .text_2xl()
                                        .font_bold()
                                        .child(self.total_tasks.to_string())
                                        .mt_2(),
                                ),
                        )
                        .child(
                            v_flex()
                                .p_4()
                                .border_1()
                                .border_color(theme.border)
                                .rounded_md()
                                .bg(theme.secondary)
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child("Tokens Today"),
                                )
                                .child(
                                    div()
                                        .text_2xl()
                                        .font_bold()
                                        .child(self.total_tokens.to_string())
                                        .mt_2(),
                                ),
                        )
                        .child(
                            v_flex()
                                .p_4()
                                .border_1()
                                .border_color(theme.border)
                                .rounded_md()
                                .bg(theme.secondary)
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child("Active Agents"),
                                )
                                .child(
                                    div()
                                        .text_2xl()
                                        .font_bold()
                                        .child(self.active_agents.to_string())
                                        .mt_2(),
                                ),
                        ),
                ),
            )
    }
}
