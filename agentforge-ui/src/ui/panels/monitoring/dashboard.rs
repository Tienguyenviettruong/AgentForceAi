use gpui::{div, prelude::*, App, IntoElement};
use gpui_component::ActiveTheme as _;

use super::charts::{BarChart, ChartData};
use crate::infrastructure::monitoring::{
    activity::{ActivityFeed, ActivityItem},
    agent_health::{AgentHealth, AgentHealthPanel},
    analytics::{AnalyticEntry, AnalyticsPanel},
    metrics::{MetricCard, MetricValue},
};

pub struct MonitoringDashboard {
    // We would typically store state here and update it from a backend service.
    // For now, we'll initialize it with some mock data for demonstration.
    metrics: Vec<MetricValue>,
    activities: Vec<ActivityItem>,
    agents: Vec<AgentHealth>,
    analytics: Vec<AnalyticEntry>,
    chart_data: Vec<ChartData>,
}

impl Default for MonitoringDashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitoringDashboard {
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
            activities: Vec::new(),
            agents: Vec::new(),
            analytics: Vec::new(),
            chart_data: Vec::new(),
        }
    }

    pub fn render(&self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();

        let mut metrics_row = div().flex().gap_4().w_full();
        for metric in &self.metrics {
            metrics_row = metrics_row.child(
                div()
                    .flex_1()
                    .child(MetricCard::new(metric.clone()).render(cx)),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap_6()
            .p_6()
            .w_full()
            .h_full()
            .bg(theme.background)
            .child(
                div()
                    .text_color(theme.foreground)
                    .text_size(gpui::px(24.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Monitoring Dashboard"),
            )
            .child(metrics_row)
            .child(
                div()
                    .flex()
                    .gap_6()
                    .w_full()
                    .child(
                        div().flex_1().child(
                            BarChart::new(
                                "Task Executions (7 Days)",
                                self.chart_data
                                    .iter()
                                    .map(|c| ChartData {
                                        label: c.label.clone(),
                                        value: c.value,
                                    })
                                    .collect(),
                            )
                            .render(cx),
                        ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(AnalyticsPanel::new(self.analytics.clone()).render(cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_6()
                    .w_full()
                    .child(
                        div()
                            .flex_1()
                            .child(AgentHealthPanel::new(self.agents.clone()).render(cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(ActivityFeed::new(self.activities.clone()).render(cx)),
                    ),
            )
    }
}
