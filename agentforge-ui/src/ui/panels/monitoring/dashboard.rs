use gpui::{div, prelude::*, App, IntoElement};
use gpui_component::ActiveTheme as _;

use super::charts::{BarChart, ChartData};
use crate::infrastructure::monitoring::{
    activity::{ActivityFeed, ActivityItem, ActivityType},
    metrics::{MetricCard, MetricValue},
};
use crate::core::traits::database::DatabasePort;
use std::sync::Arc;

pub struct MonitoringDashboard;

impl Default for MonitoringDashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitoringDashboard {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, db: &Arc<dyn DatabasePort>, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        let runs = db.list_recent_orchestration_runs(200).unwrap_or_default();
        let events = db.list_recent_run_events(None, 20).unwrap_or_default();
        let token_total: usize = db
            .get_total_tokens_per_agent()
            .unwrap_or_default()
            .into_iter()
            .map(|(_, tokens)| tokens)
            .sum();
        let active_count = runs
            .iter()
            .filter(|run| matches!(run.status.as_str(), "running" | "dispatched" | "waiting_approval"))
            .count();
        let completed_count = runs
            .iter()
            .filter(|run| run.status == "completed")
            .count();
        let metrics = vec![
            MetricValue {
                label: "Persisted Runs".to_string(),
                value: runs.len().to_string(),
                change: None,
            },
            MetricValue {
                label: "Active / Waiting".to_string(),
                value: active_count.to_string(),
                change: None,
            },
            MetricValue {
                label: "Completed".to_string(),
                value: completed_count.to_string(),
                change: None,
            },
            MetricValue {
                label: "Recorded Tokens".to_string(),
                value: token_total.to_string(),
                change: None,
            },
        ];
        let chart_data = runs
            .iter()
            .take(12)
            .rev()
            .map(|run| ChartData {
                label: run.created_at.chars().take(10).collect(),
                value: 1.0,
            })
            .collect::<Vec<_>>();
        let activities = events
            .into_iter()
            .map(|event| ActivityItem {
                message: format!("{} - {}", event.event_type, event.payload.unwrap_or_default()),
                timestamp: event.created_at,
                activity_type: if event.event_type.contains("failed")
                    || event.event_type.contains("denied")
                {
                    ActivityType::Error
                } else if event.event_type.contains("waiting")
                    || event.event_type.contains("approval")
                {
                    ActivityType::Warning
                } else if event.event_type.contains("completed")
                    || event.event_type.contains("succeeded")
                {
                    ActivityType::Success
                } else {
                    ActivityType::Info
                },
            })
            .collect::<Vec<_>>();

        let mut metrics_row = div().flex().gap_4().w_full();
        for metric in &metrics {
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
                            BarChart::new("Recent Persisted Runs", chart_data).render(cx),
                        ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(ActivityFeed::new(activities).render(cx)),
                    ),
            )
    }
}
