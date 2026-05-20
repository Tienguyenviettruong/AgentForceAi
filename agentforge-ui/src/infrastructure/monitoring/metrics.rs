use gpui::{div, prelude::*, App, IntoElement};
use gpui_component::ActiveTheme as _;

#[derive(Clone, Debug)]
pub struct MetricValue {
    pub label: String,
    pub value: String,
    pub change: Option<f32>,
}

pub struct MetricCard {
    pub metric: MetricValue,
}

impl MetricCard {
    pub fn new(metric: MetricValue) -> Self {
        Self { metric }
    }

    pub fn render(self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();

        let change_el = if let Some(change) = self.metric.change {
            let (color, text) = if change >= 0.0 {
                (
                    gpui::Hsla::from(gpui::rgb(0x10b981)),
                    format!("+{:.1}%", change),
                )
            } else {
                (
                    gpui::Hsla::from(gpui::rgb(0xef4444)),
                    format!("{:.1}%", change),
                )
            };
            div()
                .text_color(color)
                .text_size(gpui::px(12.0))
                .child(text)
        } else {
            div()
        };

        div()
            .flex()
            .flex_col()
            .p_4()
            .bg(theme.secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .gap_2()
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .text_size(gpui::px(14.0))
                    .child(self.metric.label.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .text_color(theme.foreground)
                            .text_size(gpui::px(24.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(self.metric.value.clone()),
                    )
                    .child(change_el),
            )
    }
}
