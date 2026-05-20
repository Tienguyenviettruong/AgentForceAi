use gpui::{
    div, px, App, Context, EventEmitter, InteractiveElement, IntoElement, ParentElement as _,
    Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{h_flex, ActiveTheme as _, Icon, IconName};

pub enum StatusBarEvent {
    OpenMonitoring,
    OpenAgents,
}

pub struct StatusBar {
    status_text: SharedString,
}

impl EventEmitter<StatusBarEvent> for StatusBar {}

impl StatusBar {
    pub fn new(_cx: &mut App) -> Self {
        Self {
            status_text: "Ready".into(),
        }
    }

    fn provider_status(cx: &mut Context<Self>) -> (SharedString, gpui::Hsla) {
        let providers = match crate::AppState::global(cx).db.list_providers() {
            Ok(providers) => providers,
            Err(_) => return ("Provider Status Unavailable".into(), gpui::red()),
        };

        if providers.is_empty() {
            return ("No Provider".into(), cx.theme().muted_foreground);
        }

        let available_count = providers
            .iter()
            .filter(|provider| {
                matches!(
                    provider.status.to_lowercase().as_str(),
                    "available" | "online" | "active" | "healthy"
                )
            })
            .count();

        if available_count == providers.len() {
            (
                format!(
                    "{} Provider{}",
                    available_count,
                    if available_count == 1 { "" } else { "s" }
                )
                .into(),
                gpui::green(),
            )
        } else if available_count > 0 {
            (
                format!("{} / {} Providers", available_count, providers.len()).into(),
                gpui::yellow(),
            )
        } else {
            ("Providers Offline".into(), gpui::red())
        }
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (provider_status_text, provider_status_color) = Self::provider_status(cx);
        let theme = cx.theme();

        div()
            .w_full()
            .h(px(24.))
            .flex()
            .items_center()
            .justify_between()
            .px(px(8.))
            .bg(theme.background)
            .border_t(px(1.))
            .border_color(theme.border)
            .text_color(theme.muted_foreground)
            .text_size(px(12.))
            .child(
                div().flex().items_center().gap(px(8.)).child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(IconName::Check)
                        .child(self.status_text.clone()),
                ),
            )
            .child(
                h_flex()
                    .gap(px(12.))
                    .child(
                        div()
                            .id("monitoring-btn")
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.emit(StatusBarEvent::OpenMonitoring);
                            }))
                            .child(Icon::empty().path("icons/chart.svg").size_4()),
                    )
                    .child(
                        div()
                            .id("agents-btn")
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.emit(StatusBarEvent::OpenAgents);
                            }))
                            .child(IconName::Bot),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(IconName::Globe)
                            .text_color(provider_status_color)
                            .child(provider_status_text),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(IconName::Bell)
                            .child("0"),
                    ),
            )
    }
}
