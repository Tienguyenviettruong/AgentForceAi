use gpui::{div, prelude::*, App, IntoElement};
use gpui_component::ActiveTheme as _;

pub struct ChartData {
    pub label: String,
    pub value: f32,
}

pub struct BarChart {
    pub title: String,
    pub data: Vec<ChartData>,
}

impl BarChart {
    pub fn new(title: impl Into<String>, data: Vec<ChartData>) -> Self {
        Self {
            title: title.into(),
            data,
        }
    }

    pub fn render(self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();

        let max_val = self.data.iter().map(|d| d.value).fold(0.0f32, f32::max);

        let mut bars = div().flex().items_end().gap_2().h(gpui::px(150.0)).w_full();

        for item in &self.data {
            let height_pct = if max_val > 0.0 {
                item.value / max_val
            } else {
                0.0
            };

            // Map height pct to pixel height (max 150px)
            let height_px = height_pct * 150.0;

            bars = bars.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .h_full()
                    .flex_1()
                    .child(
                        div()
                            .w_full()
                            .h(gpui::px(height_px))
                            .bg(theme.accent)
                            .rounded_t_sm(), // tooltips need an ID to track state, but for a simple render we skip it or use gpui_component::tooltip if we can
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
                    .child(self.title.clone()),
            )
            .child(bars)
    }
}
