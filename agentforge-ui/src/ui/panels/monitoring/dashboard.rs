use gpui::{div, prelude::*, App, IntoElement};
use gpui_component::ActiveTheme as _;

use super::charts::{BarChart, ChartData, PieChart};
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::monitoring::{
    activity::{ActivityFeed, ActivityItem, ActivityType},
    metrics::{MetricCard, MetricValue},
};
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
        let events = db.list_recent_run_events(None, 60).unwrap_or_default();
        let tokens_per_agent = db.get_total_tokens_per_agent().unwrap_or_default();
        let token_total: usize = tokens_per_agent.iter().map(|(_, tokens)| *tokens).sum();
        let active_count = runs
            .iter()
            .filter(|run| {
                matches!(
                    run.status.as_str(),
                    "running" | "dispatched" | "waiting_approval"
                )
            })
            .count();
        let completed_count = runs.iter().filter(|run| run.status == "completed").count();
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
        let mut persisted_by_day = std::collections::BTreeMap::<String, usize>::new();
        for run in &runs {
            let day = run.created_at.chars().take(10).collect::<String>();
            *persisted_by_day.entry(day).or_default() += 1;
        }
        let chart_data = persisted_by_day
            .into_iter()
            .rev()
            .take(12)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|(label, value)| ChartData {
                label,
                value: value as f32,
            })
            .collect::<Vec<_>>();
        let mut status_counts = std::collections::BTreeMap::<String, usize>::new();
        for run in &runs {
            *status_counts.entry(run.status.clone()).or_default() += 1;
        }
        let run_status_data = status_counts
            .into_iter()
            .map(|(label, value)| ChartData {
                label,
                value: value as f32,
            })
            .collect::<Vec<_>>();
        let token_chart_data = tokens_per_agent
            .iter()
            .take(8)
            .map(|(agent, tokens)| ChartData {
                label: agent.clone(),
                value: *tokens as f32,
            })
            .collect::<Vec<_>>();
        let activities = events
            .into_iter()
            .map(|event| ActivityItem {
                message: format!(
                    "{} - {}",
                    event.event_type,
                    event.payload.unwrap_or_default()
                ),
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
            .gap_5()
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
                    .gap_5()
                    .w_full()
                    .child(
                        div()
                            .flex_1()
                            .child(BarChart::new("Recent Persisted Runs", chart_data).render(cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(PieChart::new("Run Status", run_status_data).render(cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_5()
                    .w_full()
                    .child(
                        div()
                            .flex_1()
                            .child(ActivityFeed::new(activities).render(cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(BarChart::new("Tokens by Agent", token_chart_data).render(cx)),
                    ),
            )
    }
}
