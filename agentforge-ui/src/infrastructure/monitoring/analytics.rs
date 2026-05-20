use gpui::{div, prelude::*, App, IntoElement};
use gpui_component::ActiveTheme as _;

#[derive(Clone, Debug)]
pub struct AnalyticEntry {
    pub name: String,
    pub calls: usize,
    pub latency: f32,
}

pub struct AnalyticsPanel {
    pub entries: Vec<AnalyticEntry>,
}

impl AnalyticsPanel {
    pub fn new(entries: Vec<AnalyticEntry>) -> Self {
        Self { entries }
    }

    pub fn render(self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();

        let mut list = div().flex().flex_col().gap_2();

        // Header
        list = list.child(
            div()
                .flex()
                .w_full()
                .border_b_1()
                .border_color(theme.border)
                .pb_2()
                .mb_2()
                .child(
                    div()
                        .w_1_2()
                        .text_color(theme.muted_foreground)
                        .child("Agent / Tool"),
                )
                .child(
                    div()
                        .w_1_4()
                        .text_color(theme.muted_foreground)
                        .child("Calls"),
                )
                .child(
                    div()
                        .w_1_4()
                        .text_color(theme.muted_foreground)
                        .child("Avg Latency"),
                ),
        );

        // Entries
        for entry in &self.entries {
            list = list.child(
                div()
                    .flex()
                    .w_full()
                    .items_center()
                    .child(
                        div()
                            .w_1_2()
                            .text_color(theme.foreground)
                            .child(entry.name.clone()),
                    )
                    .child(
                        div()
                            .w_1_4()
                            .text_color(theme.foreground)
                            .child(entry.calls.to_string()),
                    )
                    .child(
                        div()
                            .w_1_4()
                            .text_color(theme.foreground)
                            .child(format!("{:.1}ms", entry.latency)),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .p_4()
            .bg(theme.secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                div()
                    .text_color(theme.foreground)
                    .text_size(gpui::px(16.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .mb_4()
                    .child("Top Analytics"),
            )
            .child(list)
    }
}
