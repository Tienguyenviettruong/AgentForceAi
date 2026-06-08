use crate::application::orchestration::modes::OperatingMode;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, Context, EventEmitter, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dock::{Panel, PanelEvent, TitleStyle};
use gpui_component::input::{Input, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::scroll::ScrollableElement;
use gpui_component::theme::ActiveTheme;
use gpui_component::{
    form::{field, v_form},
    h_flex, v_flex, Sizable, WindowExt,
};

pub struct OrchestrationPanel {
    focus_handle: gpui::FocusHandle,
    active_tab: String,
}

impl OrchestrationPanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            active_tab: "Dashboard".to_string(),
        }
    }
}

impl Panel for OrchestrationPanel {
    fn panel_name(&self) -> &'static str {
        "Orchestration"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for OrchestrationPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OrchestrationPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .p_4()
            .child(
                h_flex()
                    .gap_2()
                    .child(self.render_tab("Dashboard", cx))
                    .child(self.render_tab("Tracking", cx))
                    .child(self.render_tab("Logs", cx))
                    .child(self.render_tab("Governance", cx))
                    .child(self.render_tab("Collaboration", cx))
                    .child(self.render_tab("Learning", cx))
                    .child(self.render_tab("Context", cx))
                    .child(self.render_tab("Artifacts", cx))
                    .child(self.render_tab("Mode Transition", cx)),
            )
            .child(
                v_flex().flex_1().w_full().overflow_hidden().child(
                    div().size_full().overflow_y_scrollbar().child(
                        match self.active_tab.as_str() {
                            "Dashboard" => self.render_dashboard(cx).into_any_element(),
                            "Tracking" => self.render_tracking(cx).into_any_element(),
                            "Logs" => self.render_logs(cx).into_any_element(),
                            "Governance" => self.render_governance(cx).into_any_element(),
                            "Collaboration" => self.render_collaboration(cx).into_any_element(),
                            "Learning" => self.render_learning(cx).into_any_element(),
                            "Context" => self.render_context_inspector(cx).into_any_element(),
                            "Artifacts" => self.render_artifacts(cx).into_any_element(),
                            "Mode Transition" => self.render_mode_transition(cx).into_any_element(),
                            _ => div().child("Unknown Tab").into_any_element(),
                        },
                    ),
                ),
            )
    }
}

impl OrchestrationPanel {
    fn render_tab(&self, name: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.active_tab == name;
        let tab_name = name.to_string();

        Button::new(name)
            .label(name.to_string())
            .when(is_active, |b| b.primary())
            .when(!is_active, |b| b.ghost())
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_tab = tab_name.clone();
                cx.notify();
            }))
    }

    fn count_labels(values: impl IntoIterator<Item = String>) -> Vec<(String, usize)> {
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        for value in values {
            *counts.entry(value).or_default() += 1;
        }
        counts.into_iter().collect()
    }

    fn render_bar_chart(
        &self,
        title: &str,
        data: Vec<(String, usize)>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let max_value = data.iter().map(|(_, value)| *value).max().unwrap_or(0) as f32;
        let mut bars = h_flex().items_end().gap_2().h(px(150.)).w_full();

        if data.is_empty() {
            bars = bars.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.muted_foreground)
                    .child("No data"),
            );
        } else {
            for (label, value) in data {
                let height = if max_value > 0.0 {
                    ((value as f32 / max_value) * 128.0).max(6.0)
                } else {
                    6.0
                };
                bars = bars.child(
                    v_flex()
                        .h_full()
                        .flex_1()
                        .items_center()
                        .justify_end()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(value.to_string()),
                        )
                        .child(
                            div()
                                .w_full()
                                .max_w(px(42.))
                                .h(px(height))
                                .rounded_sm()
                                .bg(theme.primary),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(label.chars().take(12).collect::<String>()),
                        ),
                );
            }
        }

        v_flex()
            .p_4()
            .gap_3()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary.opacity(0.35))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(title.to_string()),
            )
            .child(bars)
    }

    fn render_dashboard(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let db = crate::AppState::global(cx).db.clone();
        let total_tasks = db.get_total_tasks_count().unwrap_or(0);
        let active_agents = db.get_active_agents_count().unwrap_or(0);
        let runs = db.list_recent_orchestration_runs(100).unwrap_or_default();
        let task_status_chart = Self::count_labels(
            db.list_recent_tasks(100)
                .unwrap_or_default()
                .into_iter()
                .map(|task| task.status),
        );
        let run_status_chart = Self::count_labels(runs.iter().map(|run| run.status.clone()));
        let active_runs = runs
            .iter()
            .filter(|run| matches!(run.status.as_str(), "running" | "pending" | "dispatched"))
            .count();
        let terminal_runs: Vec<_> = runs
            .iter()
            .filter(|run| matches!(run.status.as_str(), "completed" | "failed" | "cancelled"))
            .collect();
        let successful_runs = terminal_runs
            .iter()
            .filter(|run| run.status == "completed")
            .count();
        let success_rate = if terminal_runs.is_empty() {
            0
        } else {
            successful_runs * 100 / terminal_runs.len()
        };
        let recent_runs: Vec<_> = runs.into_iter().take(5).collect();

        v_flex()
            .size_full()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Orchestration Dashboard"),
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .p_4()
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Active Runs"),
                            )
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(active_runs.to_string()),
                            ),
                    )
                    .child(
                        v_flex()
                            .p_4()
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Success Rate"),
                            )
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(format!("{}%", success_rate)),
                            ),
                    )
                    .child(
                        v_flex()
                            .p_4()
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Active Agents"),
                            )
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(active_agents.to_string()),
                            ),
                    )
                    .child(
                        v_flex()
                            .p_4()
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Tasks Processed"),
                            )
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(total_tasks.to_string()),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(div().flex_1().child(self.render_bar_chart(
                        "Run Status",
                        run_status_chart,
                        cx,
                    )))
                    .child(div().flex_1().child(self.render_bar_chart(
                        "Task Status",
                        task_status_chart,
                        cx,
                    ))),
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .mt_4()
                    .child("Recent Runs"),
            )
            .child(
                v_flex()
                    .gap_2()
                    .when(recent_runs.is_empty(), |list| {
                        list.child(
                            div()
                                .p_3()
                                .text_color(theme.muted_foreground)
                                .child("No persisted orchestration runs."),
                        )
                    })
                    .children(recent_runs.into_iter().map(|run| {
                        let run_id = run.id.clone();
                        let has_workflow = run.workflow_id.is_some();
                        let goal = run.goal;
                        let scope = format!(
                            "{} / run {}",
                            run.mode,
                            run_id.chars().take(8).collect::<String>()
                        );
                        let status = run.status;
                        let time = run.updated_at;
                        h_flex()
                            .justify_between()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .child(
                                v_flex()
                                    .child(
                                        div().font_weight(gpui::FontWeight::SEMIBOLD).child(goal),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(scope),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .items_end()
                                    .gap_1()
                                    .child(div().child(status))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(time),
                                    )
                                    .when(has_workflow, |actions| {
                                        let selected_run = run_id.clone();
                                        actions.child(
                                            Button::new(gpui::SharedString::from(format!(
                                                "open-run-iflow-{}",
                                                selected_run
                                            )))
                                            .small()
                                            .label("View iFlow")
                                            .on_click(cx.listener(move |_this, _, _, cx| {
                                                let db = crate::AppState::global(cx).db.clone();
                                                let selected = crate::AppState::global(cx)
                                                    .selected_iflow_run_id
                                                    .clone();
                                                let active_panel = crate::AppState::global(cx)
                                                    .active_panel
                                                    .clone();
                                                let _ = db.set_setting(
                                                    "iflow_selected_run_id",
                                                    &selected_run,
                                                );
                                                let selected_run_id = selected_run.clone();
                                                selected.update(cx, move |current, cx| {
                                                    *current = Some(selected_run_id);
                                                    cx.notify();
                                                });
                                                active_panel.update(cx, |page, cx| {
                                                    *page = "iflow_builder".to_string();
                                                    cx.notify();
                                                });
                                            })),
                                        )
                                    }),
                            )
                            .into_any_element()
                    })),
            )
    }

    fn render_tracking(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let tasks = crate::AppState::global(cx)
            .db
            .list_recent_tasks(100)
            .unwrap_or_default();
        let agents = crate::AppState::global(cx)
            .db
            .list_agents()
            .unwrap_or_default();
        let task_status_chart = Self::count_labels(tasks.iter().map(|task| task.status.clone()));

        let mut task_list = v_flex()
            .flex_1()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary.opacity(0.3))
            .gap_3()
            .child(
                h_flex()
                    .gap_4()
                    .p_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .w(px(200.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Task Name"),
                    )
                    .child(
                        div()
                            .w(px(100.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Agent"),
                    )
                    .child(
                        div()
                            .w(px(100.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Status"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Progress"),
                    ),
            );

        if tasks.is_empty() {
            task_list = task_list.child(
                div()
                    .p_3()
                    .text_color(theme.muted_foreground)
                    .child("No persisted tasks."),
            );
        } else {
            for task in tasks {
                let (width_pct, color) = match task.status.as_str() {
                    "completed" => (1.0, theme.success),
                    "in_progress" | "running" => (0.6, theme.primary),
                    "failed" => (1.0, theme.danger),
                    _ => (0.15, theme.border),
                };
                let task_name = match task.run_id.as_ref() {
                    Some(run_id) => format!(
                        "{} [run {}]",
                        Self::task_display_name(&task),
                        run_id.chars().take(8).collect::<String>()
                    ),
                    None => Self::task_display_name(&task),
                };
                let assignee_name = Self::task_assignee_display(&task, &agents);
                task_list = task_list.child(self.render_gantt_row(
                    &task_name,
                    &assignee_name,
                    &task.status,
                    0.0,
                    width_pct,
                    color,
                    cx,
                ));
            }
        }

        v_flex()
            .size_full()
            .gap_4()
            .child(self.render_bar_chart("Task Status Distribution", task_status_chart, cx))
            .child(task_list)
    }

    fn task_display_name(task: &crate::core::models::Task) -> String {
        let Some(payload) = task.payload.as_ref() else {
            return task.id.clone();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
            return task.id.clone();
        };
        let title = value
            .get("title")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(title) = title {
            return title.to_string();
        }
        let name = value
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("");
        let description = value
            .get("description")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("");
        if !name.is_empty()
            && !matches!(
                name.to_ascii_lowercase().as_str(),
                "coordinator" | "pm" | "ba" | "dev" | "developer" | "engineer"
            )
        {
            return name.to_string();
        }
        if !description.is_empty() {
            return description
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or(description)
                .trim_start_matches(|ch: char| {
                    ch.is_ascii_digit() || ch == '.' || ch == ')' || ch == '-'
                })
                .trim()
                .chars()
                .take(96)
                .collect();
        }
        task.id.clone()
    }

    fn task_assignee_display(
        task: &crate::core::models::Task,
        agents: &[crate::core::models::Agent],
    ) -> String {
        if let Some(agent_id) = task.assignee_id.as_ref() {
            return agents
                .iter()
                .find(|agent| &agent.id == agent_id)
                .map(|agent| agent.name.clone())
                .unwrap_or_else(|| agent_id.clone());
        }

        let route = task
            .payload
            .as_ref()
            .and_then(|payload| serde_json::from_str::<serde_json::Value>(payload).ok())
            .and_then(|value| {
                value
                    .get("role")
                    .and_then(|role| role.as_str())
                    .or_else(|| value.get("name").and_then(|name| name.as_str()))
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_default();
        let route_key = Self::normalize_label(&route);
        agents
            .iter()
            .find(|agent| Self::normalize_label(&agent.routing_role()) == route_key)
            .map(|agent| format!("{} (inferred)", agent.name))
            .unwrap_or_else(|| "Unassigned".to_string())
    }

    fn normalize_label(value: &str) -> String {
        value
            .trim()
            .to_lowercase()
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect()
    }

    fn render_gantt_row(
        &self,
        task: &str,
        agent: &str,
        status: &str,
        start_pct: f32,
        width_pct: f32,
        color: gpui::Hsla,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        h_flex()
            .gap_4()
            .p_2()
            .items_center()
            .child(div().w(px(200.)).child(task.to_string()))
            .child(div().w(px(100.)).child(agent.to_string()))
            .child(div().w(px(100.)).text_sm().child(status.to_string()))
            .child(
                div()
                    .flex_1()
                    .h(px(20.))
                    .rounded_sm()
                    .bg(theme.secondary)
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(gpui::relative(start_pct))
                            .w(gpui::relative(width_pct))
                            .rounded_sm()
                            .bg(color),
                    ),
            )
    }

    fn render_logs(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let events = crate::AppState::global(cx)
            .db
            .list_recent_run_events(None, 100)
            .unwrap_or_default();
        let event_type_chart = Self::count_labels(events.iter().map(|event| {
            if event.event_type.contains("failed") {
                "failed".to_string()
            } else if event.event_type.contains("approval") || event.event_type.contains("waiting")
            {
                "approval".to_string()
            } else if event.event_type.contains("completed") {
                "completed".to_string()
            } else {
                "other".to_string()
            }
        }));
        let mut event_list = v_flex()
            .flex_1()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .gap_1();

        if events.is_empty() {
            event_list = event_list.child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("No persisted run events."),
            );
        } else {
            for event in events {
                event_list =
                    event_list.child(div().text_sm().font_family("Courier New").child(format!(
                        "{} [{}] run={} actor={} {}",
                        event.created_at,
                        event.event_type,
                        event.run_id.chars().take(8).collect::<String>(),
                        event.actor_id.unwrap_or(event.actor_type),
                        event.payload.unwrap_or_default()
                    )));
            }
        }

        v_flex()
            .size_full()
            .gap_4()
            .child(self.render_bar_chart("Event Categories", event_type_chart, cx))
            .child(event_list)
    }

    fn render_governance(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let current_mode = {
            let manager = crate::AppState::global(cx).mode_manager.lock().unwrap();
            manager.current_mode().label().to_string()
        };
        let db = crate::AppState::global(cx).db.clone();
        let transitions = db
            .list_recent_mode_transitions(1000)
            .unwrap_or_default()
            .len();
        let pending_approvals = db.list_pending_approval_requests(1000).unwrap_or_default();
        let pending_count = pending_approvals.len();
        let mut approval_list = v_flex().gap_2();
        if pending_approvals.is_empty() {
            approval_list = approval_list.child(
                div()
                    .p_3()
                    .text_color(theme.muted_foreground)
                    .child("No sensitive operations awaiting approval."),
            );
        } else {
            for request in pending_approvals.into_iter().take(20) {
                let operation = request.operation.clone();
                let operation_label = operation.chars().take(90).collect::<String>();
                let approve_id = request.id.clone();
                let approve_run_id = request.run_id.clone();
                let reject_id = request.id.clone();
                let reject_run_id = request.run_id.clone();
                approval_list = approval_list.child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .p_3()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            v_flex().gap_1().child(div().child(operation_label)).child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(format!(
                                        "Run {} | Requested {}",
                                        request.run_id.chars().take(8).collect::<String>(),
                                        request.created_at
                                    )),
                            ),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "approve-{}",
                                        approve_id
                                    )))
                                    .small()
                                    .primary()
                                    .label("Approve")
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        let state = crate::AppState::global(cx);
                                        let db = state.db.clone();
                                        let team_bus = state.team_bus.clone();
                                        let runtime = state.tokio_runtime.clone();
                                        let actor_id = state.current_actor_id.clone();
                                        let _ = db.resolve_approval_request(
                                            &approve_id,
                                            "approved",
                                            Some(&actor_id),
                                            Some("Approved in Governance panel"),
                                        );
                                        let _ = db.resolve_waiting_tasks_for_run(
                                            &approve_run_id,
                                            "pending",
                                        );
                                        let _ = db.update_orchestration_run_status(
                                            &approve_run_id,
                                            "running",
                                            None,
                                        );
                                        let _ = db.insert_run_event(
                                            &crate::core::models::RunEventRecord {
                                                id: uuid::Uuid::new_v4().to_string(),
                                                run_id: approve_run_id.clone(),
                                                event_type: "tool_approval_approved".to_string(),
                                                actor_type: "user".to_string(),
                                                actor_id: Some(actor_id.clone()),
                                                task_id: None,
                                                payload: None,
                                                created_at: chrono::Utc::now().to_rfc3339(),
                                            },
                                        );
                                        let _ = db.insert_audit_log(
                                            &crate::infrastructure::security::audit::AuditEvent {
                                                timestamp: chrono::Utc::now(),
                                                action: "tool_approval_approved".to_string(),
                                                user_id: Some(actor_id),
                                                resource: approve_id.clone(),
                                                details: "Approved from Governance panel"
                                                    .to_string(),
                                            },
                                        );
                                        let _ = crate::application::iflow_engine::automation::IFlowAutomation::resume_approved_run(
                                            db.clone(),
                                            team_bus,
                                            runtime,
                                            &approve_run_id,
                                        );
                                        cx.notify();
                                    })),
                                )
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "reject-{}",
                                        reject_id
                                    )))
                                    .small()
                                    .label("Reject")
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        let state = crate::AppState::global(cx);
                                        let db = state.db.clone();
                                        let team_bus = state.team_bus.clone();
                                        let actor_id = state.current_actor_id.clone();
                                        let _ = db.resolve_approval_request(
                                            &reject_id,
                                            "rejected",
                                            Some(&actor_id),
                                            Some("Rejected in Governance panel"),
                                        );
                                        let _ = db.resolve_waiting_tasks_for_run(
                                            &reject_run_id,
                                            "failed",
                                        );
                                        let _ = db.update_orchestration_run_status(
                                            &reject_run_id,
                                            "failed",
                                            None,
                                        );
                                        let _ = db.insert_run_event(
                                            &crate::core::models::RunEventRecord {
                                                id: uuid::Uuid::new_v4().to_string(),
                                                run_id: reject_run_id.clone(),
                                                event_type: "tool_approval_rejected".to_string(),
                                                actor_type: "user".to_string(),
                                                actor_id: Some(actor_id.clone()),
                                                task_id: None,
                                                payload: None,
                                                created_at: chrono::Utc::now().to_rfc3339(),
                                            },
                                        );
                                        let _ = db.insert_audit_log(
                                            &crate::infrastructure::security::audit::AuditEvent {
                                                timestamp: chrono::Utc::now(),
                                                action: "tool_approval_rejected".to_string(),
                                                user_id: Some(actor_id),
                                                resource: reject_id.clone(),
                                                details: "Rejected from Governance panel"
                                                    .to_string(),
                                            },
                                        );
                                        let _ = crate::application::iflow_engine::automation::IFlowAutomation::reject_waiting_run(
                                            db.clone(),
                                            team_bus,
                                            &reject_run_id,
                                        );
                                        cx.notify();
                                    })),
                                ),
                        ),
                );
            }
        }

        v_flex()
            .size_full()
            .gap_4()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Governance Status"),
            )
            .child(
                v_flex()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .child(self.render_setting_row("Operating Mode", &current_mode, cx))
                    .child(self.render_setting_row(
                        "Persisted Transitions",
                        &transitions.to_string(),
                        cx,
                    ))
                    .child(self.render_setting_row(
                        "Pending Approvals",
                        &pending_count.to_string(),
                        cx,
                    ))
                    .child(self.render_setting_row(
                        "Run Traceability",
                        "Persistent execution spine active",
                        cx,
                    ))
                    .child(self.render_setting_row(
                        "Approval Enforcement",
                        "Gateway active for governed runtime tools",
                        cx,
                    )),
            )
            .child(self.render_bar_chart(
                "Governance Workload",
                vec![
                    ("transitions".to_string(), transitions),
                    ("approvals".to_string(), pending_count),
                ],
                cx,
            ))
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Pending Tool Approvals"),
            )
            .child(approval_list)
    }

    fn render_collaboration(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let db = crate::AppState::global(cx).db.clone();
        let cases = db.list_recent_collaboration_cases(50).unwrap_or_default();
        let escalations = db.list_pending_case_escalations(50).unwrap_or_default();
        let escalation_count = escalations.len();
        let mut list = v_flex()
            .flex_1()
            .gap_2()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(theme.border);
        if cases.is_empty() {
            list = list.child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("No governed collaboration cases."),
            );
        } else {
            for case_record in cases {
                let latest_readback = db
                    .list_case_readbacks(&case_record.id)
                    .unwrap_or_default()
                    .into_iter()
                    .last();
                let readback_label = latest_readback
                    .as_ref()
                    .map(|readback| readback.status.clone())
                    .unwrap_or_else(|| "missing".to_string());
                let pending_consensus = db
                    .list_case_consensus_records(&case_record.id)
                    .unwrap_or_default()
                    .into_iter()
                    .rev()
                    .find(|record| record.status == "proposed");
                let mut block = v_flex()
                    .gap_1()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(case_record.objective.clone()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.primary)
                                    .child(case_record.state.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "Case {} | Risk {} | Readback {} | {} -> {}",
                                case_record.id.chars().take(8).collect::<String>(),
                                case_record.risk_level,
                                readback_label,
                                case_record.owner_instance_id,
                                case_record.target_instance_id
                            )),
                    );
                if let Some(readback) =
                    latest_readback.filter(|readback| readback.status == "submitted")
                {
                    let case_id = case_record.id.clone();
                    let readback_id = readback.id.clone();
                    block = block.child(
                        Button::new(gpui::SharedString::from(format!(
                            "accept-readback-{}",
                            readback_id
                        )))
                        .small()
                        .primary()
                        .label("Accept Readback")
                        .on_click(cx.listener(move |_this, _, _, cx| {
                            let state = crate::AppState::global(cx);
                            let service = crate::application::orchestration::collaboration::CollaborationService::new(
                                state.db.clone(),
                            );
                            let _ = service.accept_readback(
                                &case_id,
                                &readback_id,
                                &state.current_actor_id,
                            );
                            cx.notify();
                        })),
                    );
                }
                if let Some(consensus) = pending_consensus {
                    let approve_case_id = case_record.id.clone();
                    let approve_consensus_id = consensus.id.clone();
                    let reject_case_id = case_record.id.clone();
                    let reject_consensus_id = consensus.id.clone();
                    block = block
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(format!("Consensus proposal: {}", consensus.proposal)),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "approve-consensus-{}",
                                        approve_consensus_id
                                    )))
                                    .small()
                                    .primary()
                                    .label("Approve Outcome")
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        let state = crate::AppState::global(cx);
                                        let service = crate::application::orchestration::collaboration::CollaborationService::new(
                                            state.db.clone(),
                                        );
                                        let _ = service.resolve_consensus(
                                            &approve_case_id,
                                            &approve_consensus_id,
                                            &state.current_actor_id,
                                            true,
                                            "Outcome approved by the authorized operator.",
                                        );
                                        cx.notify();
                                    })),
                                )
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "reject-consensus-{}",
                                        reject_consensus_id
                                    )))
                                    .small()
                                    .label("Request Changes")
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        let state = crate::AppState::global(cx);
                                        let service = crate::application::orchestration::collaboration::CollaborationService::new(
                                            state.db.clone(),
                                        );
                                        let _ = service.resolve_consensus(
                                            &reject_case_id,
                                            &reject_consensus_id,
                                            &state.current_actor_id,
                                            false,
                                            "Outcome returned for changes by the authorized operator.",
                                        );
                                        cx.notify();
                                    })),
                                ),
                        );
                }
                list = list.child(block);
            }
        }
        let mut escalation_list = v_flex().gap_2();
        for escalation in escalations {
            let resume_id = escalation.id.clone();
            let cancel_id = escalation.id.clone();
            escalation_list = escalation_list.child(
                h_flex()
                    .justify_between()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        div().text_sm().child(format!(
                            "{} [{}] {}",
                            escalation.case_id.chars().take(8).collect::<String>(),
                            escalation.severity,
                            escalation.reason
                        )),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "resume-escalation-{}",
                                    resume_id
                                )))
                                .small()
                                .primary()
                                .label("Resume Case")
                                .on_click(cx.listener(move |_this, _, _, cx| {
                                    let state = crate::AppState::global(cx);
                                    let service = crate::application::orchestration::collaboration::CollaborationService::new(
                                        state.db.clone(),
                                    );
                                    let _ = service.resolve_escalation(
                                        &resume_id,
                                        &state.current_actor_id,
                                        true,
                                        "Escalation reviewed; case authorized to resume.",
                                    );
                                    cx.notify();
                                })),
                            )
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "cancel-escalation-{}",
                                    cancel_id
                                )))
                                .small()
                                .label("Cancel Case")
                                .on_click(cx.listener(move |_this, _, _, cx| {
                                    let state = crate::AppState::global(cx);
                                    let service = crate::application::orchestration::collaboration::CollaborationService::new(
                                        state.db.clone(),
                                    );
                                    let _ = service.resolve_escalation(
                                        &cancel_id,
                                        &state.current_actor_id,
                                        false,
                                        "Escalation reviewed; case cancelled by operator.",
                                    );
                                    cx.notify();
                                })),
                            ),
                    ),
            );
        }

        v_flex()
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Collaboration Cases"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("{} open escalations", escalation_count)),
                    ),
            )
            .child(list)
            .when(escalation_count > 0, |panel| {
                panel
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Escalations Requiring Resolution"),
                    )
                    .child(escalation_list)
            })
    }

    fn open_lesson_validation_dialog(
        &mut self,
        feedback_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().clone();
        let instruction =
            cx.new(|cx| InputState::new(window, cx).placeholder("Validated operating instruction"));
        let scope_kind = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("global or instance");
            input.replace("global".to_string(), window, cx);
            input
        });
        let scope_id =
            cx.new(|cx| InputState::new(window, cx).placeholder("Instance ID, blank for global"));
        let instruction_dialog = instruction.clone();
        let scope_kind_dialog = scope_kind.clone();
        let scope_id_dialog = scope_id.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let instruction_footer = instruction_dialog.clone();
            let scope_kind_footer = scope_kind_dialog.clone();
            let scope_id_footer = scope_id_dialog.clone();
            let feedback_id_footer = feedback_id.clone();
            let view_footer = view.clone();
            dialog
                .title("Validate Feedback as Lesson")
                .w(px(620.))
                .child(
                    v_form()
                        .gap(px(12.))
                        .py(px(8.))
                        .child(field().label("Instruction").required(true).child(Input::new(
                            &instruction_dialog,
                        )))
                        .child(field().label("Scope Kind").required(true).child(Input::new(
                            &scope_kind_dialog,
                        )))
                        .child(field().label("Scope ID").child(Input::new(&scope_id_dialog))),
                )
                .footer(move |_, _, _, _| {
                    let instruction = instruction_footer.clone();
                    let scope_kind = scope_kind_footer.clone();
                    let scope_id = scope_id_footer.clone();
                    let feedback_id = feedback_id_footer.clone();
                    let view = view_footer.clone();
                    vec![
                        Button::new("cancel-learning-lesson")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx))
                            .into_any_element(),
                        Button::new("save-learning-lesson")
                            .primary()
                            .label("Validate Lesson")
                            .on_click(move |_, window, cx| {
                                let state = crate::AppState::global(cx);
                                let scope_kind_value = scope_kind.read(cx).text().to_string();
                                let scope_id_value = scope_id.read(cx).text().to_string();
                                let instruction_value = instruction.read(cx).text().to_string();
                                let service = crate::application::orchestration::learning::LearningService::new(
                                    state.db.clone(),
                                );
                                let result = service.validate_lesson(
                                    None,
                                    Some(&feedback_id),
                                    scope_kind_value.trim(),
                                    scope_id_value.trim(),
                                    instruction_value.trim(),
                                    &state.current_actor_id,
                                );
                                let notification = match result {
                                    Ok(_) => (NotificationType::Success, "Lesson validated."),
                                    Err(error) => {
                                        window.push_notification(
                                            (
                                                NotificationType::Error,
                                                gpui::SharedString::from(error.to_string()),
                                            ),
                                            cx,
                                        );
                                        window.close_dialog(cx);
                                        return;
                                    }
                                };
                                window.push_notification(notification, cx);
                                window.close_dialog(cx);
                                let _ = view.update(cx, |_, cx| cx.notify());
                            })
                            .into_any_element(),
                    ]
                })
        });
    }

    fn open_candidate_dialog(
        &mut self,
        lesson_id: String,
        candidate_kind: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().clone();
        let target_id =
            cx.new(|cx| InputState::new(window, cx).placeholder("Skill ID or workflow ID"));
        let definition = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Candidate definition JSON");
            if candidate_kind == "skill" {
                input.replace("{\"instructions\":\"\"}".to_string(), window, cx);
            }
            input
        });
        let risk = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("low, medium, high, critical");
            input.replace("medium".to_string(), window, cx);
            input
        });
        let target_dialog = target_id.clone();
        let definition_dialog = definition.clone();
        let risk_dialog = risk.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let target_footer = target_dialog.clone();
            let definition_footer = definition_dialog.clone();
            let risk_footer = risk_dialog.clone();
            let lesson_footer = lesson_id.clone();
            let view_footer = view.clone();
            dialog
                .title(format!("Create {} Candidate", candidate_kind))
                .w(px(720.))
                .child(
                    v_form()
                        .gap(px(12.))
                        .py(px(8.))
                        .child(field().label("Target ID").required(true).child(Input::new(
                            &target_dialog,
                        )))
                        .child(
                            field()
                                .label("Proposed Definition JSON")
                                .required(true)
                                .child(Input::new(&definition_dialog)),
                        )
                        .child(field().label("Risk Level").required(true).child(Input::new(
                            &risk_dialog,
                        ))),
                )
                .footer(move |_, _, _, _| {
                    let target = target_footer.clone();
                    let definition = definition_footer.clone();
                    let risk = risk_footer.clone();
                    let lesson = lesson_footer.clone();
                    let view = view_footer.clone();
                    vec![
                        Button::new("cancel-learning-candidate")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx))
                            .into_any_element(),
                        Button::new("save-learning-candidate")
                            .primary()
                            .label("Create Candidate")
                            .on_click(move |_, window, cx| {
                                let state = crate::AppState::global(cx);
                                let target_value = target.read(cx).text().to_string();
                                let definition_value = definition.read(cx).text().to_string();
                                let risk_value = risk.read(cx).text().to_string();
                                let service = crate::application::orchestration::learning::LearningService::new(
                                    state.db.clone(),
                                );
                                match service.create_candidate(
                                    candidate_kind,
                                    &lesson,
                                    target_value.trim(),
                                    definition_value.trim(),
                                    risk_value.trim(),
                                    &state.current_actor_id,
                                ) {
                                    Ok(_) => window.push_notification(
                                        (NotificationType::Success, "Candidate created."),
                                        cx,
                                    ),
                                    Err(error) => window.push_notification(
                                        (
                                            NotificationType::Error,
                                            gpui::SharedString::from(error.to_string()),
                                        ),
                                        cx,
                                    ),
                                }
                                window.close_dialog(cx);
                                let _ = view.update(cx, |_, cx| cx.notify());
                            })
                            .into_any_element(),
                    ]
                })
        });
    }

    fn open_benchmark_dialog(
        &mut self,
        candidate_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().clone();
        let suite = cx.new(|cx| InputState::new(window, cx).placeholder("Benchmark suite ID"));
        let score = cx.new(|cx| InputState::new(window, cx).placeholder("Score: 0.0 to 1.0"));
        let regressions = cx.new(|cx| InputState::new(window, cx).placeholder("Regression count"));
        let evidence = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Evidence JSON");
            input.replace(
                "{\"case_results\":[],\"safety_violations\":[],\"unauthorized_side_effects\":0}"
                    .to_string(),
                window,
                cx,
            );
            input
        });
        let suite_dialog = suite.clone();
        let score_dialog = score.clone();
        let regressions_dialog = regressions.clone();
        let evidence_dialog = evidence.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let suite_footer = suite_dialog.clone();
            let score_footer = score_dialog.clone();
            let regressions_footer = regressions_dialog.clone();
            let evidence_footer = evidence_dialog.clone();
            let candidate_footer = candidate_id.clone();
            let view_footer = view.clone();
            dialog
                .title("Record Benchmark Evidence")
                .w(px(760.))
                .child(
                    v_form()
                        .gap(px(12.))
                        .py(px(8.))
                        .child(field().label("Suite ID").required(true).child(Input::new(
                            &suite_dialog,
                        )))
                        .child(field().label("Aggregate Score").required(true).child(
                            Input::new(&score_dialog),
                        ))
                        .child(field().label("Regression Count").required(true).child(
                            Input::new(&regressions_dialog),
                        ))
                        .child(field().label("Evidence JSON").required(true).child(Input::new(
                            &evidence_dialog,
                        ))),
                )
                .footer(move |_, _, _, _| {
                    let suite = suite_footer.clone();
                    let score = score_footer.clone();
                    let regressions = regressions_footer.clone();
                    let evidence = evidence_footer.clone();
                    let candidate = candidate_footer.clone();
                    let view = view_footer.clone();
                    vec![
                        Button::new("cancel-learning-benchmark")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx))
                            .into_any_element(),
                        Button::new("save-learning-benchmark")
                            .primary()
                            .label("Record Benchmark")
                            .on_click(move |_, window, cx| {
                                let suite_value = suite.read(cx).text().to_string();
                                let score_value = score.read(cx).text().to_string();
                                let regressions_value = regressions.read(cx).text().to_string();
                                let evidence_value = evidence.read(cx).text().to_string();
                                let parsed_score = score_value.trim().parse::<f64>().unwrap_or(-1.0);
                                let parsed_regressions =
                                    regressions_value.trim().parse::<i64>().unwrap_or(-1);
                                let state = crate::AppState::global(cx);
                                let service = crate::application::orchestration::learning::LearningService::new(
                                    state.db.clone(),
                                );
                                match service.record_benchmark(
                                    &candidate,
                                    suite_value.trim(),
                                    parsed_score,
                                    parsed_regressions,
                                    evidence_value.trim(),
                                    &state.current_actor_id,
                                ) {
                                    Ok(_) => window.push_notification(
                                        (NotificationType::Success, "Benchmark recorded."),
                                        cx,
                                    ),
                                    Err(error) => window.push_notification(
                                        (
                                            NotificationType::Error,
                                            gpui::SharedString::from(error.to_string()),
                                        ),
                                        cx,
                                    ),
                                }
                                window.close_dialog(cx);
                                let _ = view.update(cx, |_, cx| cx.notify());
                            })
                            .into_any_element(),
                    ]
                })
        });
    }

    fn open_canary_dialog(
        &mut self,
        candidate_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().clone();
        let scope = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Bounded canary scope JSON");
            input.replace(
                "{\"risk_level\":\"low\",\"eligible_case_filter\":\"manual\"}".to_string(),
                window,
                cx,
            );
            input
        });
        let traffic = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Traffic percent");
            input.replace("1".to_string(), window, cx);
            input
        });
        let scope_dialog = scope.clone();
        let traffic_dialog = traffic.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let scope_footer = scope_dialog.clone();
            let traffic_footer = traffic_dialog.clone();
            let candidate_footer = candidate_id.clone();
            let view_footer = view.clone();
            dialog
                .title("Start Bounded Canary")
                .w(px(680.))
                .child(
                    v_form()
                        .gap(px(12.))
                        .py(px(8.))
                        .child(field().label("Scope JSON").required(true).child(Input::new(
                            &scope_dialog,
                        )))
                        .child(field().label("Traffic Percent").required(true).child(
                            Input::new(&traffic_dialog),
                        )),
                )
                .footer(move |_, _, _, _| {
                    let scope = scope_footer.clone();
                    let traffic = traffic_footer.clone();
                    let candidate = candidate_footer.clone();
                    let view = view_footer.clone();
                    vec![
                        Button::new("cancel-learning-canary")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx))
                            .into_any_element(),
                        Button::new("save-learning-canary")
                            .primary()
                            .label("Start Canary")
                            .on_click(move |_, window, cx| {
                                let scope_value = scope.read(cx).text().to_string();
                                let traffic_value = traffic.read(cx).text().to_string();
                                let percentage =
                                    traffic_value.trim().parse::<f64>().unwrap_or(-1.0);
                                let state = crate::AppState::global(cx);
                                let service = crate::application::orchestration::learning::LearningService::new(
                                    state.db.clone(),
                                );
                                match service.start_canary(
                                    &candidate,
                                    scope_value.trim(),
                                    percentage,
                                    &state.current_actor_id,
                                ) {
                                    Ok(_) => window.push_notification(
                                        (NotificationType::Success, "Canary started."),
                                        cx,
                                    ),
                                    Err(error) => window.push_notification(
                                        (
                                            NotificationType::Error,
                                            gpui::SharedString::from(error.to_string()),
                                        ),
                                        cx,
                                    ),
                                }
                                window.close_dialog(cx);
                                let _ = view.update(cx, |_, cx| cx.notify());
                            })
                            .into_any_element(),
                    ]
                })
        });
    }

    fn render_learning(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let db = crate::AppState::global(cx).db.clone();
        let candidates = db.list_recent_learning_candidates(50).unwrap_or_default();
        let lessons = db.list_active_lessons(None, None, 50).unwrap_or_default();
        let all_lessons = db.list_recent_lessons(50).unwrap_or_default();
        let pending_feedback: Vec<_> = db
            .list_recent_feedback_records(50)
            .unwrap_or_default()
            .into_iter()
            .filter(|feedback| feedback.validation_status == "quarantined")
            .collect();
        let evaluations = db.list_recent_run_evaluations(50).unwrap_or_default();
        let rollbacks = db.list_recent_rollback_records(50).unwrap_or_default();
        let mut benchmark_jobs_queued = 0usize;
        let mut benchmark_jobs_running = 0usize;
        let mut benchmark_jobs_failed = 0usize;
        let mut benchmark_runs_passed = 0usize;
        let mut benchmark_runs_failed = 0usize;
        let mut canaries_running = 0usize;
        let mut canary_observations_passed = 0usize;
        let mut canary_observations_failed = 0usize;
        for candidate in &candidates {
            for job in db
                .list_benchmark_runner_jobs_for_candidate(&candidate.id)
                .unwrap_or_default()
            {
                match job.status.as_str() {
                    "queued" => benchmark_jobs_queued += 1,
                    "running" => benchmark_jobs_running += 1,
                    "failed" => benchmark_jobs_failed += 1,
                    _ => {}
                }
            }
            for run in db
                .list_benchmark_runs_for_candidate(&candidate.id)
                .unwrap_or_default()
            {
                match run.status.as_str() {
                    "passed" => benchmark_runs_passed += 1,
                    "failed" => benchmark_runs_failed += 1,
                    _ => {}
                }
            }
            for deployment in db
                .list_canary_deployments_for_candidate(&candidate.id)
                .unwrap_or_default()
            {
                if deployment.status == "running" {
                    canaries_running += 1;
                }
                for observation in db
                    .list_canary_observations_for_deployment(&deployment.id)
                    .unwrap_or_default()
                {
                    match observation.verdict.as_str() {
                        "pass" => canary_observations_passed += 1,
                        "fail" => canary_observations_failed += 1,
                        _ => {}
                    }
                }
            }
        }
        let telemetry = h_flex()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Benchmark Runner"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{} queued | {} running | {} failed jobs",
                                benchmark_jobs_queued,
                                benchmark_jobs_running,
                                benchmark_jobs_failed
                            )),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{} passed | {} failed runs",
                                benchmark_runs_passed, benchmark_runs_failed
                            )),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Canary Telemetry"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("{} running deployments", canaries_running)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{} pass | {} fail observations",
                                canary_observations_passed, canary_observations_failed
                            )),
                    ),
            );
        let mut evidence_list = v_flex()
            .gap_2()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Quarantined Feedback"),
            );
        if pending_feedback.is_empty() {
            evidence_list = evidence_list.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No feedback awaits validation."),
            );
        } else {
            for feedback in pending_feedback.iter().take(10) {
                let feedback_id = feedback.id.clone();
                evidence_list = evidence_list.child(
                    h_flex()
                        .gap_3()
                        .justify_between()
                        .child(
                            v_flex()
                                .flex_1()
                                .child(div().text_sm().child(format!(
                                    "{}: {}",
                                    feedback.subject_kind, feedback.subject_id
                                )))
                                .child(
                                    div().text_sm().text_color(theme.muted_foreground).child(
                                        feedback.content.chars().take(180).collect::<String>(),
                                    ),
                                ),
                        )
                        .child(
                            Button::new(gpui::SharedString::from(format!(
                                "validate-feedback-{}",
                                feedback_id
                            )))
                            .small()
                            .label("Validate Lesson")
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.open_lesson_validation_dialog(
                                        feedback_id.clone(),
                                        window,
                                        cx,
                                    );
                                },
                            )),
                        ),
                );
            }
        }
        let validated_lessons: Vec<_> = all_lessons
            .iter()
            .filter(|lesson| lesson.status == "validated")
            .collect();
        let mut lesson_list = v_flex()
            .gap_2()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Validated Lessons Awaiting Candidate"),
            );
        if validated_lessons.is_empty() {
            lesson_list = lesson_list.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No validated lesson is waiting for versioning."),
            );
        } else {
            for lesson in validated_lessons.into_iter().take(10) {
                let lesson_for_skill = lesson.id.clone();
                let lesson_for_workflow = lesson.id.clone();
                lesson_list = lesson_list.child(
                    v_flex()
                        .gap_1()
                        .child(div().text_sm().child(format!(
                            "{} / {}: {}",
                            lesson.scope_kind, lesson.scope_id, lesson.instruction
                        )))
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "skill-candidate-{}",
                                        lesson_for_skill
                                    )))
                                    .small()
                                    .label("New Skill Candidate")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.open_candidate_dialog(
                                            lesson_for_skill.clone(),
                                            "skill",
                                            window,
                                            cx,
                                        );
                                    })),
                                )
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "workflow-candidate-{}",
                                        lesson_for_workflow
                                    )))
                                    .small()
                                    .label("New Workflow Candidate")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.open_candidate_dialog(
                                            lesson_for_workflow.clone(),
                                            "workflow",
                                            window,
                                            cx,
                                        );
                                    })),
                                ),
                        ),
                );
            }
        }
        let mut list = v_flex()
            .gap_2()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Candidate Versions"),
            );
        if candidates.is_empty() {
            list = list.child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("No governed learning candidates."),
            );
        } else {
            for candidate in candidates {
                let candidate_id = candidate.id.clone();
                let candidate_status = candidate.status.clone();
                let benchmarks = db
                    .list_benchmark_runs_for_candidate(&candidate.id)
                    .unwrap_or_default()
                    .len();
                let benchmark_jobs = db
                    .list_benchmark_runner_jobs_for_candidate(&candidate.id)
                    .unwrap_or_default()
                    .len();
                let canaries = db
                    .list_canary_deployments_for_candidate(&candidate.id)
                    .unwrap_or_default();
                let canary_count = canaries.len();
                let running_deployment_id = canaries
                    .iter()
                    .find(|deployment| deployment.status == "running")
                    .map(|deployment| deployment.id.clone());
                let mut candidate_block = v_flex()
                    .gap_1()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(format!(
                                "{}: {}",
                                candidate.candidate_kind, candidate.target_id
                            )))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.primary)
                                    .child(candidate_status.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "Risk {} | Benchmarks {} | Auto jobs {} | Canaries {} | Lesson {}",
                                candidate.risk_level,
                                benchmarks,
                                benchmark_jobs,
                                canary_count,
                                candidate.source_lesson_id
                            )),
                    );
                if matches!(candidate_status.as_str(), "draft" | "benchmark_failed") {
                    let benchmark_id = candidate_id.clone();
                    let auto_benchmark_id = candidate_id.clone();
                    candidate_block = candidate_block.child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "auto-benchmark-candidate-{}",
                                    auto_benchmark_id
                                )))
                                .small()
                                .primary()
                                .label("Run Auto Benchmark")
                                .on_click(cx.listener(move |_this, _, window, cx| {
                                    let state = crate::AppState::global(cx);
                                    let runner = crate::application::orchestration::benchmark_runner::BenchmarkRunner::new(state.db.clone());
                                    let actor_id = state.current_actor_id.clone();
                                    let candidate_id = auto_benchmark_id.clone();
                                    let view = cx.entity().clone();
                                    window.push_notification(
                                        (
                                            NotificationType::Success,
                                            "Automatic benchmark started.",
                                        ),
                                        cx,
                                    );
                                    cx.spawn(async move |_, cx| {
                                        let outcome = runner
                                            .run_candidate(&candidate_id, None, &actor_id)
                                            .await;
                                        let _ = cx.update(|cx| {
                                            let _ = view.update(cx, |_, cx| {
                                                cx.notify();
                                            });
                                        });
                                        outcome.ok();
                                    })
                                    .detach();
                                })),
                            )
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "benchmark-candidate-{}",
                                    benchmark_id
                                )))
                                .small()
                                .label("Record Benchmark")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_benchmark_dialog(benchmark_id.clone(), window, cx);
                                })),
                            ),
                    );
                }
                if candidate_status == "benchmark_passed" {
                    let canary_id = candidate_id.clone();
                    candidate_block = candidate_block.child(
                        Button::new(gpui::SharedString::from(format!(
                            "canary-candidate-{}",
                            canary_id
                        )))
                        .small()
                        .label("Start Canary")
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.open_canary_dialog(canary_id.clone(), window, cx);
                            },
                        )),
                    );
                }
                if candidate_status == "canary_running" {
                    if let Some(deployment_id) = running_deployment_id {
                        let passed_candidate_id = candidate_id.clone();
                        let failed_candidate_id = candidate_id.clone();
                        let passed_deployment_id = deployment_id.clone();
                        candidate_block = candidate_block.child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "pass-canary-{}",
                                        passed_candidate_id
                                    )))
                                    .small()
                                    .primary()
                                    .label("Pass Canary")
                                    .on_click(cx.listener(move |_this, _, window, cx| {
                                        let state = crate::AppState::global(cx);
                                        let service = crate::application::orchestration::learning::LearningService::new(state.db.clone());
                                        let outcome = service.record_canary_outcome(
                                            &passed_candidate_id,
                                            &passed_deployment_id,
                                            true,
                                            &state.current_actor_id,
                                        );
                                        match outcome {
                                            Ok(()) => window.push_notification((NotificationType::Success, "Canary passed."), cx),
                                            Err(error) => window.push_notification((NotificationType::Error, gpui::SharedString::from(error.to_string())), cx),
                                        }
                                        cx.notify();
                                    })),
                                )
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "fail-canary-{}",
                                        failed_candidate_id
                                    )))
                                    .small()
                                    .label("Fail Canary")
                                    .on_click(cx.listener(move |_this, _, window, cx| {
                                        let state = crate::AppState::global(cx);
                                        let service = crate::application::orchestration::learning::LearningService::new(state.db.clone());
                                        let outcome = service.record_canary_outcome(
                                            &failed_candidate_id,
                                            &deployment_id,
                                            false,
                                            &state.current_actor_id,
                                        );
                                        match outcome {
                                            Ok(()) => window.push_notification((NotificationType::Success, "Canary failed and promotion is blocked."), cx),
                                            Err(error) => window.push_notification((NotificationType::Error, gpui::SharedString::from(error.to_string())), cx),
                                        }
                                        cx.notify();
                                    })),
                                ),
                        );
                    }
                }
                if candidate_status == "canary_passed" {
                    let promote_id = candidate_id.clone();
                    candidate_block = candidate_block.child(
                        Button::new(gpui::SharedString::from(format!(
                            "promote-candidate-{}",
                            promote_id
                        )))
                        .small()
                        .primary()
                        .label("Promote")
                        .on_click(cx.listener(
                            move |_this, _, window, cx| {
                                let state = crate::AppState::global(cx);
                                let service =
                                crate::application::orchestration::learning::LearningService::new(
                                    state.db.clone(),
                                );
                                match service.promote(
                                    &promote_id,
                                    &state.current_actor_id,
                                    "Promoted by authorized operator after persisted canary pass.",
                                ) {
                                    Ok(_) => window.push_notification(
                                        (NotificationType::Success, "Candidate promoted."),
                                        cx,
                                    ),
                                    Err(error) => window.push_notification(
                                        (
                                            NotificationType::Error,
                                            gpui::SharedString::from(error.to_string()),
                                        ),
                                        cx,
                                    ),
                                }
                                cx.notify();
                            },
                        )),
                    );
                }
                if matches!(
                    candidate_status.as_str(),
                    "promoted" | "canary_running" | "canary_passed"
                ) {
                    let rollback_id = candidate_id.clone();
                    candidate_block = candidate_block.child(
                        Button::new(gpui::SharedString::from(format!(
                            "rollback-candidate-{}",
                            rollback_id
                        )))
                        .small()
                        .label("Rollback")
                        .on_click(cx.listener(
                            move |_this, _, window, cx| {
                                let state = crate::AppState::global(cx);
                                let service =
                                crate::application::orchestration::learning::LearningService::new(
                                    state.db.clone(),
                                );
                                match service.rollback(
                                    &rollback_id,
                                    None,
                                    &state.current_actor_id,
                                    "Rollback initiated by authorized operator.",
                                ) {
                                    Ok(_) => window.push_notification(
                                        (NotificationType::Success, "Candidate rolled back."),
                                        cx,
                                    ),
                                    Err(error) => window.push_notification(
                                        (
                                            NotificationType::Error,
                                            gpui::SharedString::from(error.to_string()),
                                        ),
                                        cx,
                                    ),
                                }
                                cx.notify();
                            },
                        )),
                    );
                }
                list = list.child(candidate_block);
            }
        }
        v_flex()
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Governed Learning"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{} active lessons | {} evaluations | {} rollbacks",
                                lessons.len(),
                                evaluations.len(),
                                rollbacks.len()
                            )),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_4()
                    .overflow_y_scrollbar()
                    .child(telemetry)
                    .child(evidence_list)
                    .child(lesson_list)
                    .child(list),
            )
    }

    fn render_context_inspector(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let db = crate::AppState::global(cx).db.clone();
        let snapshots = db
            .list_recent_llm_context_snapshots(None, 30)
            .unwrap_or_default();
        let mut snapshot_list = v_flex()
            .flex_1()
            .gap_3()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.background);

        if snapshots.is_empty() {
            snapshot_list = snapshot_list.child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("No persisted LLM request contexts."),
            );
        } else {
            for snapshot in snapshots {
                let sources = db
                    .list_llm_context_sources(&snapshot.id)
                    .unwrap_or_default();
                let run_label = snapshot
                    .run_id
                    .as_deref()
                    .map(|run_id| run_id.chars().take(8).collect::<String>())
                    .unwrap_or_else(|| "unbound".to_string());
                let hash_label = snapshot.context_hash.chars().take(12).collect::<String>();
                let mut source_list = v_flex().gap_1();
                for source in sources.iter().take(8) {
                    source_list = source_list.child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .font_family("Courier New")
                            .child(format!(
                                "{} | {} | {} chars | {}",
                                source.source_kind,
                                source.source_id,
                                source.character_count,
                                source.trust_level
                            )),
                    );
                }
                snapshot_list =
                    snapshot_list.child(
                        v_flex()
                            .gap_2()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(
                                        format!(
                                            "Run {} | Agent {} | {}",
                                            run_label,
                                            snapshot.agent_id,
                                            snapshot.mode.as_deref().unwrap_or("no-mode")
                                        ),
                                    ))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(snapshot.created_at),
                                    ),
                            )
                            .child(div().text_sm().child(format!(
                                "Request hash {} | {} chars | {} source(s)",
                                hash_label,
                                snapshot.character_count,
                                sources.len()
                            )))
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                format!("Capabilities: {}", snapshot.selected_capabilities_json),
                            ))
                            .child(source_list),
                    );
            }
        }

        v_flex()
            .size_full()
            .gap_4()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("LLM Context Inspector"),
            )
            .child(snapshot_list)
    }

    fn render_artifacts(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let db = crate::AppState::global(cx).db.clone();
        let mut artifacts = Vec::new();
        for run in db.list_recent_orchestration_runs(50).unwrap_or_default() {
            if let Ok(records) = db.list_artifacts_for_run(&run.id) {
                artifacts.extend(records);
            }
        }
        artifacts.sort_by(|left, right| right.created_at.cmp(&left.created_at));

        let mut list = v_flex()
            .flex_1()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .gap_2();
        if artifacts.is_empty() {
            list = list.child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("No persisted run artifacts."),
            );
        } else {
            for artifact in artifacts.into_iter().take(100) {
                let run_label = artifact
                    .run_id
                    .as_deref()
                    .map(|id| id.chars().take(8).collect::<String>())
                    .unwrap_or_else(|| "unbound".to_string());
                let hash_label = artifact.content_hash.chars().take(12).collect::<String>();
                list =
                    list.child(
                        v_flex()
                            .gap_1()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(artifact.artifact_kind),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(artifact.created_at),
                                    ),
                            )
                            .child(div().text_sm().child(artifact.path))
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                format!(
                                    "Run {} | Agent {} | Call {} | Hash {}",
                                    run_label,
                                    artifact.agent_id.as_deref().unwrap_or("none"),
                                    artifact.invocation_id.as_deref().unwrap_or("none"),
                                    hash_label
                                ),
                            )),
                    );
            }
        }

        v_flex()
            .size_full()
            .gap_4()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Run Artifacts"),
            )
            .child(list)
    }

    fn render_setting_row(&self, label: &str, value: &str, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        h_flex()
            .justify_between()
            .items_center()
            .p_2()
            .border_b_1()
            .border_color(theme.border)
            .child(div().child(label.to_string()))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.primary)
                    .child(value.to_string()),
            )
    }

    fn render_mode_transition(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let mode = crate::AppState::global(cx)
            .mode_manager
            .lock()
            .unwrap()
            .current_mode();
        let current_mode = mode.label().to_string();
        let mutation_policy = match mode {
            OperatingMode::HumanInteraction => "Approval for every mutation",
            OperatingMode::Supervision => "Approval for sensitive operations",
            OperatingMode::Autonomous => "Sensitive operations require explicit policy enablement",
        };

        v_flex().size_full().gap_4().child(
            v_flex()
                .flex_1()
                .gap_4()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.background)
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("Trigger Manual Transition"),
                        )
                        .child(
                            div()
                                .text_color(theme.primary)
                                .child(format!("Current Mode: {}", current_mode)),
                        ),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("btn-transition-autonomous")
                                .primary()
                                .label("Autonomous Mode")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_transition_dialog(
                                        OperatingMode::Autonomous,
                                        window,
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            Button::new("btn-transition-supervision")
                                .label("Supervision")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_transition_dialog(
                                        OperatingMode::Supervision,
                                        window,
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            Button::new("btn-transition-human")
                                .label("Human Interaction")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_transition_dialog(
                                        OperatingMode::HumanInteraction,
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
                .child(
                    v_flex()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .p_4()
                        .child(div().mb_2().child("Transition Recording"))
                        .child(self.render_setting_row("Shared Mode State", "Enabled", cx))
                        .child(self.render_setting_row("Mutation Policy", mutation_policy, cx))
                        .child(self.render_setting_row("Database Setting", "Persisted", cx))
                        .child(self.render_setting_row("Mode Transition History", "Persisted", cx)),
                ),
        )
    }

    fn open_transition_dialog(
        &self,
        target_mode: OperatingMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().clone();

        window.open_dialog(cx, move |dialog, _window, cx| {
            let view_save = view.clone();
            let target_mode_save = target_mode;
            let theme = cx.theme().clone();

            dialog
                .title("Mode Transition Safety Check")
                .w(px(500.))
                .child(
                    v_flex()
                        .gap_4()
                        .py_4()
                        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(format!("Target Mode: {}", target_mode_save.label())))
                        .child(div().text_sm().child("This updates the shared operating mode and records the change in persistent audit data."))
                        .child(
                            v_flex().gap_2()
                                .child(h_flex().gap_2().items_center()
                                    .child(div().w_4().h_4().rounded_full().bg(theme.primary))
                                    .child(div().text_sm().child("Mode state persisted to application settings"))
                                )
                                .child(h_flex().gap_2().items_center()
                                    .child(div().w_4().h_4().rounded_full().bg(theme.primary))
                                    .child(div().text_sm().child("Transition written to audit log"))
                                )
                                .child(h_flex().gap_2().items_center()
                                    .child(div().w_4().h_4().rounded_full().bg(theme.warning))
                                    .child(div().text_sm().child("Sensitive runtime tools require policy approval and respect run token budgets"))
                                )
                        )
                )
                .footer({
                    let view_save = view_save.clone();
                    let target_mode_save = target_mode_save.clone();

                    move |_, _, _, _| {
                        let view_save2 = view_save.clone();
                        let target_mode_save2 = target_mode_save.clone();

                        vec![
                            Button::new("cancel-transition")
                                .label("Cancel")
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                })
                                .into_any_element(),
                            Button::new("confirm-transition")
                                .primary()
                                .label("Confirm Transition")
                                .on_click({
                                    let view_save3 = view_save2.clone();
                                    let target_mode_save3 = target_mode_save2.clone();

                                    move |_ev, window, cx| {
                                        let db = crate::AppState::global(cx).db.clone();
                                        let (result, from_mode, changed) = {
                                            let mut manager = crate::AppState::global(cx)
                                                .mode_manager
                                                .lock()
                                                .unwrap();
                                            let from_mode = manager.current_mode();
                                            if from_mode == target_mode_save3 {
                                                (Ok(()), from_mode, false)
                                            } else {
                                                (
                                                    manager.transition_to(
                                                        target_mode_save3,
                                                        "User confirmed mode transition in orchestration panel",
                                                    ),
                                                    from_mode,
                                                    true,
                                                )
                                            }
                                        };
                                        if result.is_ok() && changed {
                                            let actor_id = crate::AppState::global(cx)
                                                .current_actor_id
                                                .clone();
                                            let _ = db.set_setting(
                                                "orchestration_mode",
                                                target_mode_save3.storage_value(),
                                            );
                                            let _ = db.insert_mode_transition(
                                                &crate::core::models::ModeTransitionRecord {
                                                    id: uuid::Uuid::new_v4().to_string(),
                                                    instance_id: None,
                                                    run_id: None,
                                                    actor_id: Some(actor_id.clone()),
                                                    from_mode: from_mode.storage_value().to_string(),
                                                    to_mode: target_mode_save3
                                                        .storage_value()
                                                        .to_string(),
                                                    reason: Some(
                                                        "User confirmed mode transition in orchestration panel"
                                                            .to_string(),
                                                    ),
                                                    policy_version: Some("phase4-gateway".to_string()),
                                                    created_at: chrono::Utc::now().to_rfc3339(),
                                                },
                                            );
                                            let _ = db.insert_audit_log(
                                                &crate::infrastructure::security::audit::AuditEvent {
                                                    timestamp: chrono::Utc::now(),
                                                    action: "mode_transition".to_string(),
                                                    user_id: Some(actor_id),
                                                    resource: "orchestration".to_string(),
                                                    details: format!(
                                                        "Mode changed to {} from orchestration panel",
                                                        target_mode_save3.label()
                                                    ),
                                                },
                                            );
                                            view_save3.update(cx, |_this, cx| cx.notify());
                                        }
                                        window.close_dialog(cx);
                                        use gpui_component::notification::NotificationType;
                                        window.push_notification(
                                            (NotificationType::Success, "Mode transition successful."),
                                            cx,
                                        );
                                    }
                                })
                                .into_any_element(),
                        ]
                    }
                })
        });
    }
}

impl EventEmitter<PanelEvent> for OrchestrationPanel {}
