use gpui::{div, prelude::*, App, IntoElement};
use gpui_component::{ActiveTheme as _, IconName};

#[derive(Clone, Debug, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Offline,
}

#[derive(Clone, Debug)]
pub struct AgentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub cpu_usage: f32,
    pub memory_usage: f32,
}

pub struct AgentHealthPanel {
    pub agents: Vec<AgentHealth>,
}

impl AgentHealthPanel {
    pub fn new(agents: Vec<AgentHealth>) -> Self {
        Self { agents }
    }

    pub fn render(self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();

        let mut list = div().flex().flex_col().gap_2();

        for agent in &self.agents {
            let (status_color, status_text) = match agent.status {
                HealthStatus::Healthy => (gpui::Hsla::from(gpui::rgb(0x10b981)), "Healthy"),
                HealthStatus::Degraded => (gpui::Hsla::from(gpui::rgb(0xf59e0b)), "Degraded"),
                HealthStatus::Offline => (gpui::Hsla::from(gpui::rgb(0xef4444)), "Offline"),
            };

            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_3()
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w(gpui::px(8.0))
                                    .h(gpui::px(8.0))
                                    .rounded_full()
                                    .bg(status_color),
                            )
                            .child(
                                div()
                                    .text_color(theme.foreground)
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(agent.name.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .text_color(theme.muted_foreground)
                            .text_size(gpui::px(12.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(IconName::SquareTerminal)
                                    .child(format!("{:.1}%", agent.cpu_usage)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(IconName::Inbox)
                                    .child(format!("{:.1} MB", agent.memory_usage)),
                            )
                            .child(
                                div()
                                    .w(gpui::px(60.0))
                                    .flex()
                                    .justify_end()
                                    .text_color(status_color)
                                    .child(status_text),
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
                    .child("Agent Health"),
            )
            .child(list)
    }
}
