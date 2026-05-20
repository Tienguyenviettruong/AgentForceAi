use gpui::{div, App, Context, Focusable, IntoElement, ParentElement, Render, Styled, Window};
use gpui_component::StyledExt;
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    theme::ActiveTheme,
    v_flex,
};

pub struct UsageAnalytics {
    focus_handle: gpui::FocusHandle,
    db: std::sync::Arc<dyn crate::core::traits::database::DatabasePort>,
    total_tasks: usize,
    active_agents: usize,
}

impl UsageAnalytics {
    pub fn new(cx: &mut App) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        let total_tasks = db.get_total_tasks_completed().unwrap_or(0);
        let active_agents = db.get_active_agents_count().unwrap_or(0);

        Self {
            focus_handle: cx.focus_handle(),
            db,
            total_tasks,
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
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_xl()
                            .text_color(theme.foreground)
                            .child("Usage Analytics")
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("btn-export-csv").label("Export CSV"))
                            .child(
                                Button::new("btn-generate-report")
                                    .primary()
                                    .label("Generate Custom Report")
                                    .on_click(|_, _, _| {})
                            )
                    )
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_4()
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .child("Analyze your platform usage, agent efficiency, and generate custom reports.")
                    )
                    // Metrics Row
                    .child(
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
                                    .child(div().text_sm().text_color(theme.muted_foreground).child("Total Tasks Completed"))
                                    .child(div().text_2xl().font_bold().child(self.total_tasks.to_string()).mt_2())
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child("+12% from last week")
                                            .mt_1()
                                    )
                            )
                            .child(
                                v_flex()
                                    .p_4()
                                    .border_1()
                                    .border_color(theme.border)
                                    .rounded_md()
                                    .bg(theme.secondary)
                                    .flex_1()
                                    .child(div().text_sm().text_color(theme.muted_foreground).child("Average Resolution Time"))
                                    .child(div().text_2xl().font_bold().child("4m 12s").mt_2())
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child("-5% from last week")
                                            .mt_1()
                                    )
                            )
                            .child(
                                v_flex()
                                    .p_4()
                                    .border_1()
                                    .border_color(theme.border)
                                    .rounded_md()
                                    .bg(theme.secondary)
                                    .flex_1()
                                    .child(div().text_sm().text_color(theme.muted_foreground).child("Active Agents"))
                                    .child(div().text_2xl().font_bold().child(self.active_agents.to_string()).mt_2())
                            )
                    )
            )
    }
}
