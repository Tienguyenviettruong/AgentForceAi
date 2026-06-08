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

fn render_usage_chart(
    title: &str,
    data: Vec<(&'static str, usize)>,
    cx: &Context<UsageAnalytics>,
) -> impl IntoElement {
    let theme = cx.theme().clone();
    let max_value = data.iter().map(|(_, value)| *value).max().unwrap_or(0) as f32;
    let mut bars = h_flex().items_end().gap_3().h(gpui::px(160.)).w_full();

    for (label, value) in data {
        let height = if max_value > 0.0 {
            ((value as f32 / max_value) * 130.0).max(6.0)
        } else {
            6.0
        };
        bars = bars.child(
            v_flex()
                .h_full()
                .flex_1()
                .items_center()
                .justify_end()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(value.to_string()),
                )
                .child(
                    div()
                        .w_full()
                        .max_w(gpui::px(48.))
                        .h(gpui::px(height))
                        .rounded_sm()
                        .bg(theme.primary),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(label),
                ),
        );
    }

    v_flex()
        .w_full()
        .p_4()
        .gap_3()
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .bg(theme.secondary)
        .child(div().font_bold().child(title.to_string()))
        .child(bars)
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
            .child(render_usage_chart(
                "Usage Summary",
                vec![
                    ("tasks", self.total_tasks),
                    ("tokens", self.total_tokens),
                    ("agents", self.active_agents),
                ],
                cx,
            ))
    }
}
