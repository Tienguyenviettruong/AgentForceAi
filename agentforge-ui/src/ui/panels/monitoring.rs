pub mod charts;
pub mod dashboard;
pub mod token_dashboard;

use gpui::EventEmitter;
use gpui::{
    div, prelude::*, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement,
    Render, Styled, Window,
};
use gpui_component::dock::PanelEvent;
use gpui_component::dock::{Panel, TitleStyle};
use gpui_component::scroll::ScrollableElement;
use gpui_component::{h_flex, v_flex, ActiveTheme};

use crate::monitoring::usage_analytics::UsageAnalytics;
use crate::ui::panels::monitoring::dashboard::MonitoringDashboard;
use crate::ui::panels::monitoring::token_dashboard::TokenDashboard;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MonitoringTab {
    Dashboard,
    TokenUsage,
    UsageAnalytics,
}

pub struct MonitoringPanel {
    focus_handle: gpui::FocusHandle,
    active_tab: MonitoringTab,
    dashboard: MonitoringDashboard,
    token_dashboard: Entity<TokenDashboard>,
    usage_analytics: Entity<UsageAnalytics>,
}

impl MonitoringPanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            active_tab: MonitoringTab::Dashboard,
            dashboard: MonitoringDashboard::new(),
            token_dashboard: cx.new(|cx| TokenDashboard::new(cx)),
            usage_analytics: cx.new(|cx| UsageAnalytics::new(cx)),
        }
    }
}

impl Panel for MonitoringPanel {
    fn panel_name(&self) -> &'static str {
        "Monitoring & Analytics"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for MonitoringPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MonitoringPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let tabs = h_flex()
            .w_full()
            .border_b_1()
            .border_color(theme.border)
            .gap_4()
            .p_4()
            .child(
                div()
                    .id("tab_dashboard")
                    .cursor_pointer()
                    .text_color(if self.active_tab == MonitoringTab::Dashboard {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    })
                    .font_weight(if self.active_tab == MonitoringTab::Dashboard {
                        gpui::FontWeight::BOLD
                    } else {
                        gpui::FontWeight::NORMAL
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.active_tab = MonitoringTab::Dashboard;
                        cx.notify();
                    }))
                    .child("System Health"),
            )
            .child(
                div()
                    .id("tab_token_usage")
                    .cursor_pointer()
                    .text_color(if self.active_tab == MonitoringTab::TokenUsage {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    })
                    .font_weight(if self.active_tab == MonitoringTab::TokenUsage {
                        gpui::FontWeight::BOLD
                    } else {
                        gpui::FontWeight::NORMAL
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.active_tab = MonitoringTab::TokenUsage;
                        cx.notify();
                    }))
                    .child("Token Usage"),
            )
            .child(
                div()
                    .id("tab_usage_analytics")
                    .cursor_pointer()
                    .text_color(if self.active_tab == MonitoringTab::UsageAnalytics {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    })
                    .font_weight(if self.active_tab == MonitoringTab::UsageAnalytics {
                        gpui::FontWeight::BOLD
                    } else {
                        gpui::FontWeight::NORMAL
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.active_tab = MonitoringTab::UsageAnalytics;
                        cx.notify();
                    }))
                    .child("Usage Analytics"),
            );

        let content = match self.active_tab {
            MonitoringTab::Dashboard => {
                let db = crate::AppState::global(cx).db.clone();
                div()
                    .size_full()
                    .child(self.dashboard.render(&db, cx))
                    .into_any_element()
            }
            MonitoringTab::TokenUsage => div()
                .size_full()
                .child(self.token_dashboard.clone())
                .into_any_element(),
            MonitoringTab::UsageAnalytics => div()
                .size_full()
                .child(self.usage_analytics.clone())
                .into_any_element(),
        };

        v_flex().size_full().bg(theme.background).child(tabs).child(
            div()
                .flex_1()
                .overflow_hidden()
                .child(div().size_full().overflow_y_scrollbar().child(content)),
        )
    }
}

impl EventEmitter<PanelEvent> for MonitoringPanel {}
