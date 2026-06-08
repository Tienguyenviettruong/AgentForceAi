use gpui::{canvas, div, point, prelude::*, px, App, IntoElement};
use gpui_component::ActiveTheme as _;

#[derive(Clone)]
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
        let palette = chart_palette();

        let max_val = self.data.iter().map(|d| d.value).fold(0.0f32, f32::max);

        let mut bars = div().flex().items_end().gap_2().h(px(160.0)).w_full();

        for (idx, item) in self.data.iter().enumerate() {
            let height_pct = if max_val > 0.0 {
                item.value / max_val
            } else {
                0.0
            };

            let height_px = if item.value > 0.0 {
                (height_pct * 128.0).max(8.0)
            } else {
                0.0
            };
            let color = palette[idx % palette.len()];

            bars = bars.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .h_full()
                    .flex_1()
                    .gap_1()
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .text_size(px(11.0))
                            .child(format!("{:.0}", item.value)),
                    )
                    .child(
                        div()
                            .w_full()
                            .max_w(px(44.0))
                            .h(px(height_px))
                            .bg(color)
                            .rounded_t_sm(),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .text_align(gpui::TextAlign::Center)
                            .text_color(theme.muted_foreground)
                            .text_size(px(11.0))
                            .child(compact_chart_label(&item.label, 12)),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .p_4()
            .h(px(256.0))
            .bg(theme.secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                div()
                    .text_color(theme.foreground)
                    .text_size(px(16.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .mb_4()
                    .child(self.title.clone()),
            )
            .child(bars)
    }
}

pub struct PieChart {
    pub title: String,
    pub data: Vec<ChartData>,
}

impl PieChart {
    pub fn new(title: impl Into<String>, data: Vec<ChartData>) -> Self {
        Self {
            title: title.into(),
            data,
        }
    }

    pub fn render(self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        let palette = chart_palette();
        let total = self
            .data
            .iter()
            .map(|item| item.value.max(0.0))
            .sum::<f32>();
        let data_for_canvas = self.data.clone();
        let palette_for_canvas = palette.clone();

        let mut legend = div().flex().flex_col().gap_2().min_w(px(150.0));
        if self.data.is_empty() || total <= 0.0 {
            legend = legend.child(
                div()
                    .text_color(theme.muted_foreground)
                    .text_size(px(12.0))
                    .child("No status data"),
            );
        } else {
            for (idx, item) in self.data.iter().enumerate() {
                let color = palette[idx % palette.len()];
                let pct = item.value / total * 100.0;
                legend = legend.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .min_w_0()
                        .child(div().w(px(10.0)).h(px(10.0)).rounded_full().bg(color))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(theme.foreground)
                                .text_size(px(12.0))
                                .child(compact_chart_label(&item.label, 18)),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_color(theme.muted_foreground)
                                .text_size(px(12.0))
                                .child(format!("{:.0} ({:.0}%)", item.value, pct)),
                        ),
                );
            }
        }

        div()
            .flex()
            .flex_col()
            .p_4()
            .h(px(256.0))
            .bg(theme.secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                div()
                    .text_color(theme.foreground)
                    .text_size(px(16.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .mb_4()
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(
                        div().w(px(170.0)).h(px(170.0)).child(
                            canvas(
                                move |_bounds, _window, _cx| {},
                                move |bounds, _, window, _cx| {
                                    if data_for_canvas.is_empty() || total <= 0.0 {
                                        return;
                                    }
                                    let width: f32 = bounds.size.width.into();
                                    let height: f32 = bounds.size.height.into();
                                    let radius = width.min(height) * 0.43;
                                    let center = point(
                                        bounds.origin.x + px(width / 2.0),
                                        bounds.origin.y + px(height / 2.0),
                                    );
                                    let mut start = -std::f32::consts::FRAC_PI_2;
                                    for (idx, item) in data_for_canvas.iter().enumerate() {
                                        let value = item.value.max(0.0);
                                        if value <= 0.0 {
                                            continue;
                                        }
                                        let sweep = std::f32::consts::TAU * (value / total);
                                        let end = start + sweep;
                                        let steps = ((sweep / std::f32::consts::TAU) * 72.0)
                                            .ceil()
                                            .max(3.0)
                                            as usize;
                                        let mut builder = gpui::PathBuilder::fill();
                                        builder.move_to(center);
                                        for step in 0..=steps {
                                            let t = step as f32 / steps as f32;
                                            let angle = start + (end - start) * t;
                                            builder.line_to(point(
                                                center.x + px(angle.cos() * radius),
                                                center.y + px(angle.sin() * radius),
                                            ));
                                        }
                                        builder.close();
                                        if let Ok(path) = builder.build() {
                                            window.paint_path(
                                                path,
                                                palette_for_canvas[idx % palette_for_canvas.len()],
                                            );
                                        }
                                        start = end;
                                    }
                                },
                            )
                            .size_full(),
                        ),
                    )
                    .child(legend),
            )
    }
}

fn chart_palette() -> Vec<gpui::Hsla> {
    vec![
        gpui::Hsla::from(gpui::rgb(0x2563eb)),
        gpui::Hsla::from(gpui::rgb(0x10b981)),
        gpui::Hsla::from(gpui::rgb(0xf59e0b)),
        gpui::Hsla::from(gpui::rgb(0xef4444)),
        gpui::Hsla::from(gpui::rgb(0x8b5cf6)),
        gpui::Hsla::from(gpui::rgb(0x06b6d4)),
        gpui::Hsla::from(gpui::rgb(0xec4899)),
        gpui::Hsla::from(gpui::rgb(0x64748b)),
    ]
}

fn compact_chart_label(label: &str, max_chars: usize) -> String {
    let trimmed = label.trim();
    if trimmed.len() >= 10
        && trimmed.as_bytes().get(4) == Some(&b'-')
        && trimmed.as_bytes().get(7) == Some(&b'-')
    {
        return trimmed.chars().skip(5).take(5).collect();
    }

    let count = trimmed.chars().count();
    if count <= max_chars {
        return trimmed.to_string();
    }

    trimmed.chars().take(max_chars).collect()
}
