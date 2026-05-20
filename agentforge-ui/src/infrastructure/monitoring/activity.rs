use gpui::{div, prelude::*, App, IntoElement};
use gpui_component::{ActiveTheme as _, IconName};

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

        let mut list = div().flex().flex_col().gap_3();

        for item in &self.activities {
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
                    .items_start()
                    .gap_3()
                    .child(div().text_color(color).mt(gpui::px(2.0)).child(icon))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_color(theme.foreground)
                                    .text_size(gpui::px(14.0))
                                    .child(item.message.clone()),
                            )
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .text_size(gpui::px(12.0))
                                    .child(item.timestamp.clone()),
                            ),
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
                    .child("Recent Activity"),
            )
            .child(list)
    }
}
