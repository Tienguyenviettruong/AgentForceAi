use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, Animation, AnimationExt, Context, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::spinner::Spinner;
use gpui_component::WindowExt;
use gpui_component::{h_flex, ActiveTheme as _, IconName};
use std::time::Duration;

use super::TeamWorkspacePanel;

fn normalized_route_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn task_payload_value(task: &crate::tasks::shared_task_list::Task) -> Option<serde_json::Value> {
    task.payload
        .as_deref()
        .and_then(|payload| serde_json::from_str::<serde_json::Value>(payload).ok())
}

fn task_payload_string(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|field| field.as_str())
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .unwrap_or("")
        .to_string()
}

fn title_from_description(description: &str) -> Option<String> {
    description
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| {
            line.trim_start_matches(|ch: char| {
                ch.is_ascii_digit() || ch == '.' || ch == ')' || ch == '-'
            })
            .trim()
            .chars()
            .take(96)
            .collect::<String>()
        })
        .filter(|title| !title.is_empty())
}

impl TeamWorkspacePanel {
    fn task_matches_agent(
        task: &crate::tasks::shared_task_list::Task,
        agent: &crate::db::Agent,
    ) -> bool {
        if task.assignee_id.as_deref() == Some(&agent.id) {
            return true;
        }
        if task.assignee_id.is_some() {
            return false;
        }

        let Some(value) = task_payload_value(task) else {
            return false;
        };
        let route = task_payload_string(&value, "role");
        let route = if route.is_empty() {
            task_payload_string(&value, "name")
        } else {
            route
        };
        !route.is_empty()
            && normalized_route_key(&route) == normalized_route_key(&agent.routing_role())
    }

    fn task_display_parts(task: &crate::tasks::shared_task_list::Task) -> (String, String) {
        let Some(value) = task_payload_value(task) else {
            return ("Untitled task".to_string(), String::new());
        };

        let description = task_payload_string(&value, "description");
        let title = task_payload_string(&value, "title");
        if !title.is_empty() {
            return (title, description);
        }

        let name = task_payload_string(&value, "name");
        let role = task_payload_string(&value, "role");
        let name_looks_like_role = !name.is_empty()
            && (normalized_route_key(&name) == normalized_route_key(&role)
                || matches!(
                    normalized_route_key(&name).as_str(),
                    "coordinator" | "pm" | "ba" | "dev" | "developer" | "engineer"
                ));
        if name_looks_like_role {
            if let Some(title) = title_from_description(&description) {
                return (title, description);
            }
        }

        if !name.is_empty() {
            return (name, description);
        }
        if let Some(title) = title_from_description(&description) {
            return (title, description);
        }
        ("Untitled task".to_string(), description)
    }

    fn render_task_status_indicator(status: &str, color: gpui::Hsla) -> gpui::AnyElement {
        match status {
            "completed" => div()
                .text_color(color)
                .child(IconName::CircleCheck)
                .into_any_element(),
            "failed" => div()
                .text_color(color)
                .child(IconName::CircleX)
                .into_any_element(),
            "in_progress" => Spinner::new().color(color).into_any_element(),
            _ => div()
                .text_color(color)
                .child(IconName::Asterisk)
                .into_any_element(),
        }
    }

    fn render_task_title(title: String, text_size: f32) -> gpui::AnyElement {
        let char_count = title.chars().count();
        let should_marquee = char_count > 34;
        let title_text = div()
            .min_w_0()
            .whitespace_nowrap()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_size(px(text_size))
            .child(title);

        div()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .child(if should_marquee {
                let travel =
                    ((char_count.saturating_sub(34) as f32) * text_size * 0.55).clamp(36.0, 180.0);
                title_text
                    .with_animation(
                        "task-title-marquee",
                        Animation::new(Duration::from_secs_f64(7.0)).repeat(),
                        move |this, delta| {
                            let offset = if delta < 0.18 {
                                0.0
                            } else if delta > 0.88 {
                                0.0
                            } else if delta < 0.53 {
                                travel * ((delta - 0.18) / 0.35)
                            } else {
                                travel * (1.0 - ((delta - 0.53) / 0.35))
                            };
                            this.ml(-px(offset))
                        },
                    )
                    .into_any_element()
            } else {
                title_text.truncate().into_any_element()
            })
            .into_any_element()
    }

    fn task_assignee_name(
        task: &crate::tasks::shared_task_list::Task,
        agents: &[crate::db::Agent],
    ) -> String {
        if let Some(aid) = &task.assignee_id {
            return agents
                .iter()
                .find(|agent| &agent.id == aid)
                .map(|agent| agent.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());
        }

        agents
            .iter()
            .find(|agent| Self::task_matches_agent(task, agent))
            .map(|agent| format!("{} (inferred)", agent.name))
            .unwrap_or_else(|| "Unassigned".to_string())
    }

    fn task_requested_role(task: &crate::tasks::shared_task_list::Task) -> Option<String> {
        let value = task_payload_value(task)?;
        let role = task_payload_string(&value, "role");
        if !role.is_empty() {
            return Some(role);
        }
        let requested_role = task_payload_string(&value, "requested_role");
        if !requested_role.is_empty() {
            return Some(requested_role);
        }
        None
    }

    fn task_assignment_warning(
        task: &crate::tasks::shared_task_list::Task,
        agents: &[crate::db::Agent],
    ) -> Option<String> {
        if task.assignee_id.is_some()
            || agents
                .iter()
                .any(|agent| Self::task_matches_agent(task, agent))
        {
            return None;
        }
        Self::task_requested_role(task).map(|role| {
            format!(
                "Requested role: {} - No matching online agent in this instance",
                role
            )
        })
    }

    pub(crate) fn render_member_item(
        &self,
        agent: &crate::db::Agent,
        tasks: &[crate::tasks::shared_task_list::Task],
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        let status = if agent.status.is_empty() {
            "Idle"
        } else {
            &agent.status
        };
        let is_completed = status.to_lowercase() == "completed";

        let agent_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| Self::task_matches_agent(t, agent))
            .collect();
        let agent_tasks_key = format!(
            "agent-tasks-{}-{}",
            self.selected_instance_id.as_deref().unwrap_or("none"),
            agent.id
        );
        let tasks_expanded = self.expanded_groups.contains(&agent_tasks_key);

        div()
            .flex()
            .flex_col()
            .border_b(px(1.))
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .p(px(16.))
                    .hover(|s| s.bg(theme.secondary))
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                h_flex().gap(px(8.)).child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_size(px(14.))
                                        .child(agent.name.clone()),
                                ),
                            )
                            .child(
                                h_flex()
                                    .gap(px(4.))
                                    .text_color(if is_completed {
                                        gpui::green()
                                    } else {
                                        theme.primary
                                    })
                                    .child(
                                        div()
                                            .w(px(12.))
                                            .h(px(12.))
                                            .rounded_full()
                                            .border(px(2.))
                                            .border_color(if is_completed {
                                                gpui::green()
                                            } else {
                                                theme.primary
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(
                                                if is_completed {
                                                    gpui::green()
                                                } else {
                                                    theme.transparent
                                                },
                                            )),
                                    )
                                    .child(div().text_size(px(12.)).child(status.to_string())),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap(px(6.))
                                    .text_color(theme.muted_foreground)
                                    .child(IconName::Bot)
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .child(format!("@ {}", agent.provider)),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .id(SharedString::from(agent_tasks_key.clone()))
                                    .gap(px(4.))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.expanded_groups.contains(&agent_tasks_key) {
                                            this.expanded_groups.remove(&agent_tasks_key);
                                        } else {
                                            this.expanded_groups.insert(agent_tasks_key.clone());
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(format!("{} tasks", agent_tasks.len())),
                                    )
                                    .child(div().text_color(theme.muted_foreground).child(
                                        if tasks_expanded {
                                            IconName::ChevronDown
                                        } else {
                                            IconName::ChevronRight
                                        },
                                    )),
                            ),
                    ),
            )
            .when(tasks_expanded && !agent_tasks.is_empty(), |d| {
                let mut tasks_list = div().flex().flex_col().bg(theme.secondary.opacity(0.1));
                for t in agent_tasks {
                    let status_color = match t.status.as_str() {
                        "completed" => gpui::green(),
                        "failed" => gpui::red(),
                        "in_progress" => gpui::blue(),
                        _ => theme.muted_foreground,
                    };
                    let (task_title, task_desc) = Self::task_display_parts(t);
                    tasks_list = tasks_list.child(
                        div()
                            .p(px(12.))
                            .pl(px(32.))
                            .border_t(px(1.))
                            .border_color(theme.border)
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                h_flex()
                                    .w_full()
                                    .min_w_0()
                                    .justify_between()
                                    .child(Self::render_task_title(task_title, 13.))
                                    .child(
                                        h_flex()
                                            .flex_none()
                                            .gap(px(4.))
                                            .text_color(status_color)
                                            .child(Self::render_task_status_indicator(
                                                t.status.as_str(),
                                                status_color,
                                            )),
                                    ),
                            )
                            .when(!task_desc.is_empty(), |d| {
                                d.child(
                                    div()
                                        .overflow_hidden()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(task_desc),
                                )
                            }),
                    );
                }
                d.child(tasks_list)
            })
    }

    pub(crate) fn render_members_column(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let active_team_id = self.selected_team_id.clone().or_else(|| {
            self.selected_instance_id.as_ref().and_then(|iid| {
                self.instances
                    .iter()
                    .find(|i| i.id == *iid)
                    .map(|i| i.team_id.clone())
            })
        });

        let active_team = active_team_id
            .as_ref()
            .and_then(|id| self.teams.iter().find(|t| t.id == *id));

        let title = active_team
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Select a team".to_string());

        let db = crate::AppState::global(cx).db.clone();
        let agents = if let Some(team) = active_team {
            let agent_ids = if let Some(instance_id) = &self.selected_instance_id {
                db.get_instance_agents(instance_id).unwrap_or_default()
            } else {
                db.get_team_agents(&team.id).unwrap_or_default()
            };
            if !agent_ids.is_empty() {
                agent_ids
                    .iter()
                    .filter_map(|id| db.get_agent(id).ok().flatten())
                    .collect::<Vec<_>>()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let tasks = if let Some(instance_id) = &self.selected_instance_id {
            db.list_tasks_for_instance(instance_id).unwrap_or_default()
        } else {
            vec![]
        };

        let container = div()
            .h_full()
            .w_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            // .border_r(px(1.))
            .border_color(theme.border)
            .bg(theme.background);

        container
            // Header
            .child(
                div()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .pl(px(16.))
                    .pr(px(8.))
                    .border_b(px(1.))
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
                                    .min_w_0()
                                    .gap(px(8.))
                                    .items_center()
                                    .child(
                                        div()
                                            .text_color(theme.primary)
                                            .child(IconName::User),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(
                                                div()
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .text_size(px(14.))
                                                    .child(title),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(format!("{} members", agents.len()))
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .flex_none()
                                    .gap(px(8.))
                                    .when(self.selected_instance_id.is_none(), |d| {
                                        d.child(
                                            Button::new("run-team")
                                                .label("Run Instance")
                                                .icon(IconName::ArrowRight)
                                                .primary()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    if let Some(team_id) = this.selected_team_id.clone() {
                                                        let db = crate::AppState::global(cx).db.clone();
                                                        let new_id = format!("inst-{}", uuid::Uuid::new_v4().simple());
                                                        if db.create_instance(&new_id, "New Instance", &team_id, None, Some("initializing")).is_ok() {
                                                            let initialized = db
                                                                .get_instance_agents(&new_id)
                                                                .map(|agents| !agents.is_empty())
                                                                .unwrap_or(false);
                                                            let next_state = if initialized { "running" } else { "failed" };
                                                            let _ = db.update_instance_state(&new_id, next_state);
                                                            this.selected_instance_id = Some(new_id.clone());
                                                            this.selected_team_id = None;
                                                            this.start_team_bus_subscription(new_id.clone(), cx);
                                                            this.reload(cx);
                                                        }
                                                    }
                                                }))
                                        )
                                    })
                                    .child(
                                        Button::new("manage-team")
                                            .tooltip("Manage Team")
                                            .icon(IconName::Settings)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let active_team_id = this.selected_team_id.clone().or_else(|| {
                                                    this.selected_instance_id.as_ref().and_then(|iid| {
                                                        this.instances.iter().find(|i| i.id == *iid).map(|i| i.team_id.clone())
                                                    })
                                                });
                                                if let Some(team_id) = active_team_id {
                                                    let db = crate::AppState::global(cx).db.clone();
                                                    crate::ui::components::dialogs::open_manage_team_dialog(
                                                        db,
                                                        cx.entity().clone(),
                                                        team_id,
                                                        window,
                                                        cx,
                                                        |view: &mut crate::ui::panels::team_workspace::TeamWorkspacePanel, cx: &mut Context<crate::ui::panels::team_workspace::TeamWorkspacePanel>| view.reload(cx),
                                                    );
                                                } else {
                                                    use gpui_component::notification::NotificationType;
                                                    window.push_notification(
                                                        (NotificationType::Warning, "Please select a team first."),
                                                        cx,
                                                    );
                                                }
                                            }))
                                    )
                            ),
                    ),
            )
            // Tabs
            .child(
                h_flex()
                    .h(px(36.))
                    .w_full()
                    .items_center()
                    .border_b(px(1.))
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .id("tab-members")
                            .flex_1()
                            .justify_center()
                            .gap(px(6.))
                            .py(px(8.))
                            .border_b(px(2.))
                            .border_color(if self.members_active_tab == 0 { theme.primary } else { theme.transparent })
                            .text_color(if self.members_active_tab == 0 { theme.foreground } else { theme.muted_foreground })
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.members_active_tab = 0;
                                cx.notify();
                            }))
                            .child(IconName::User)
                            .child(div().text_size(px(13.)).child("Members"))
                            .child(
                                div()
                                    .px(px(6.))
                                    .bg(theme.secondary)
                                    .rounded_md()
                                    .text_size(px(11.))
                                    .child(agents.len().to_string()),
                            ),
                    )
                    .child(
                        h_flex()
                            .id("tab-tasks")
                            .flex_1()
                            .justify_center()
                            .gap(px(6.))
                            .py(px(8.))
                            .border_b(px(2.))
                            .border_color(if self.members_active_tab == 1 { theme.primary } else { theme.transparent })
                            .text_color(if self.members_active_tab == 1 { theme.foreground } else { theme.muted_foreground })
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.members_active_tab = 1;
                                cx.notify();
                            }))
                            .child(IconName::Check)
                            .child(div().text_size(px(13.)).child("Tasks"))
                            .child(
                                div()
                                    .px(px(6.))
                                    .bg(theme.secondary)
                                    .rounded_md()
                                    .text_size(px(11.))
                                    .child(tasks.len().to_string()),
                            ),
                    )
            )
            // Scroll area
            .child(
                div()
                    .flex_1()
                    .id("member-scroll")
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .when(self.members_active_tab == 0, |d| {
                        let mut list = div().flex().flex_col();
                        if agents.is_empty() {
                            list = list.child(
                                div()
                                    .p(px(16.))
                                    .text_color(theme.muted_foreground)
                                    .text_size(px(13.))
                                    .child("No agents in this team."),
                            );
                        } else {
                            for agent in &agents {
                                list = list.child(self.render_member_item(agent, &tasks, cx));
                            }
                        }
                        d.child(list)
                    })
                    .when(self.members_active_tab == 1, |d| {
                        let mut list = div().flex().flex_col().p(px(16.)).gap(px(12.));
                        if tasks.is_empty() {
                            list = list.child(
                                div()
                                    .p(px(16.))
                                    .text_color(theme.muted_foreground)
                                    .text_size(px(13.))
                                    .child("No tasks for this instance.")
                            );
                        } else {
                            for t in tasks {
                                let status_color = match t.status.as_str() {
                                    "completed" => gpui::green(),
                                    "failed" => gpui::red(),
                                    "in_progress" => gpui::blue(),
                                    _ => theme.muted_foreground,
                                };
                                let (task_title, task_desc) = Self::task_display_parts(&t);
                                let assignee_name = Self::task_assignee_name(&t, &agents);
                                let assignment_warning = Self::task_assignment_warning(&t, &agents);
                                list = list.child(
                                    div()
                                        .p(px(12.))
                                        .border(px(1.))
                                        .rounded_lg()
                                        .border_color(theme.border)
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.))
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .min_w_0()
                                                .justify_between()
                                                .child(
                                                    h_flex()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .gap(px(6.))
                                                        .items_center()
                                                        .child(Self::render_task_title(
                                                            task_title, 14.,
                                                        ))
                                                )
                                                .child(
                                                    h_flex()
                                                        .flex_none()
                                                        .gap(px(4.))
                                                        .text_color(status_color)
                                                        .child(Self::render_task_status_indicator(
                                                            t.status.as_str(),
                                                            status_color,
                                                        ))
                                                )
                                        )
                                        .child(
                                            h_flex()
                                                .justify_between()
                                                .child(
                                                    h_flex()
                                                        .gap(px(4.))
                                                        .text_color(theme.muted_foreground)
                                                        .child(IconName::User)
                                                        .child(div().text_size(px(12.)).child(assignee_name))
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(theme.muted_foreground)
                                                        .child(format!("Priority: {}", t.priority))
                                                )
                                        )
                                        .when_some(assignment_warning, |d, warning| {
                                            d.child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(gpui::red())
                                                    .child(warning),
                                            )
                                        })
                                        .when(!task_desc.is_empty(), |d| {
                                            d.child(
                                                div()
                                                    .overflow_hidden()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(task_desc),
                                            )
                                        })
                                );
                            }
                        }
                        d.child(list)
                    })
            )
    }
}
