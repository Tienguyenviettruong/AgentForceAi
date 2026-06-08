use gpui::{div, prelude::*, App, IntoElement};
use gpui_component::{scroll::ScrollableElement, ActiveTheme as _, IconName};

#[derive(Clone, Debug)]
pub enum ActivityType {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Clone, Debug)]
pub struct ActivityItem {
    pub message: String,
    pub timestamp: String,
    pub activity_type: ActivityType,
}

pub struct ActivityFeed {
    pub activities: Vec<ActivityItem>,
}

impl ActivityFeed {
    pub fn new(activities: Vec<ActivityItem>) -> Self {
        Self { activities }
    }

    pub fn render(self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();

        let mut list = div().flex().flex_col().gap_3().pr_2();

        for item in &self.activities {
            let message = compact_activity_text(&item.message, 140);
            let (icon, color) = match item.activity_type {
                ActivityType::Info => (IconName::Info, theme.accent),
                ActivityType::Warning => (
                    IconName::TriangleAlert,
                    gpui::Hsla::from(gpui::rgb(0xf59e0b)),
                ),
                ActivityType::Error => (IconName::CircleX, gpui::Hsla::from(gpui::rgb(0xef4444))),
                ActivityType::Success => {
                    (IconName::CircleCheck, gpui::Hsla::from(gpui::rgb(0x10b981)))
                }
            };

            list = list.child(
                div()
                    .flex()
                    .w_full()
                    .min_w_0()
                    .items_start()
                    .gap_3()
                    .child(
                        div()
                            .flex_none()
                            .text_color(color)
                            .mt(gpui::px(2.0))
                            .child(icon),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .text_color(theme.foreground)
                                    .text_size(gpui::px(14.0))
                                    .child(message),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .text_color(theme.muted_foreground)
                                    .text_size(gpui::px(12.0))
                                    .child(compact_timestamp(&item.timestamp)),
                            ),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .p_4()
            .h(gpui::px(256.0))
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
                    .child("Recent Activity"),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(gpui::px(0.0))
                    .overflow_y_scrollbar()
                    .child(list),
            )
    }
}

fn compact_activity_text(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut out = normalized
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

fn compact_timestamp(value: &str) -> String {
    value
        .split('T')
        .nth(1)
        .map(|time| time.chars().take(8).collect::<String>())
        .filter(|time| !time.is_empty())
        .unwrap_or_else(|| value.chars().take(19).collect())
}
