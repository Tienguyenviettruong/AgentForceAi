use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::WindowExt;
use gpui_component::{h_flex, v_flex, ActiveTheme as _, IconName};

use super::TeamWorkspacePanel;

impl TeamWorkspacePanel {
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
            .filter(|t| t.assignee_id.as_deref() == Some(&agent.id))
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
                    let status_icon = match t.status.as_str() {
                        "completed" => IconName::CircleCheck,
                        "failed" => IconName::CircleX,
                        "in_progress" => IconName::LoaderCircle,
                        _ => IconName::Asterisk,
                    };
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
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_size(px(13.))
                                            .child(
                                                t.id.split(':').next_back().unwrap_or(&t.id).to_string(),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .gap(px(4.))
                                            .text_color(status_color)
                                            .child(status_icon),
                                    ),
                            )
                            .when_some(t.payload.clone(), |d, payload| {
                                d.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(payload),
                                )
                            }),
                    );
                }
                d.child(tasks_list)
            })
    }

    pub(crate) fn render_group_header(
        &self,
        icon_color: gpui::Hsla,
        title: &str,
        is_expanded: bool,
        theme: &gpui_component::Theme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let title_clone = title.to_string();
        div()
            .flex()
            .flex_col()
            .border_t(px(1.))
            .border_b(px(1.))
            .border_color(theme.border)
            .bg(theme.secondary.opacity(0.3))
            .child(
                h_flex()
                    .id(SharedString::from(title.to_string()))
                    .justify_between()
                    .py(px(8.))
                    .px(px(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.expanded_groups.contains(&title_clone) {
                            this.expanded_groups.remove(&title_clone);
                        } else {
                            this.expanded_groups.insert(title_clone.clone());
                        }
                        cx.notify();
                    }))
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(if is_expanded {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    }),
                            )
                            .child(
                                div()
                                    .w(px(20.))
                                    .h(px(20.))
                                    .rounded_sm()
                                    .bg(icon_color.opacity(0.1))
                                    .text_color(icon_color)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(IconName::User),
                            )
                            .child(
                                div()
                                    .overflow_x_hidden()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_size(px(14.))
                                    .child(title.to_string()),
                            ),
                    ),
            )
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
            db
                .list_tasks_for_instance(instance_id)
                .unwrap_or_default()
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
                    .pl(px(16.))
                    // .py(px(2.))
                    .pr(px(8.))
                    .border_b(px(1.))
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
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
                                                        if db.create_instance(&new_id, &team_id, None, Some("running")).is_ok() {
                                                            this.reload(cx);
                                                            this.selected_instance_id = Some(new_id.clone());
                                                            this.selected_team_id = None;
                                                            this.start_team_bus_subscription(new_id.clone(), cx);
                                                            let history_len = this.chat_histories.get(&new_id).map(|h| h.len()).unwrap_or(0);
                                                            this.chat_list_state = gpui::ListState::new(history_len, gpui::ListAlignment::Bottom, px(200.));
                                                            cx.notify();
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
                    .w_full()
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
                        d.child(
                            self.render_group_header(
                                gpui::blue(),
                                "All Agents",
                                self.expanded_groups.contains("All Agents"),
                                theme,
                                cx,
                            ),
                        )
                        .when(
                            self.expanded_groups.contains("All Agents"),
                            |d| {
                                let mut list = div().flex().flex_col();
                                for agent in &agents {
                                    list = list.child(self.render_member_item(agent, &tasks, cx));
                                }
                                d.child(list)
                            },
                        )
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
                                let status_icon = match t.status.as_str() {
                                    "completed" => IconName::CircleCheck,
                                    "failed" => IconName::CircleX,
                                    "in_progress" => IconName::LoaderCircle,
                                    _ => IconName::Asterisk,
                                };
                                let assignee_name = if let Some(aid) = &t.assignee_id {
                                    agents.iter().find(|a| &a.id == aid).map(|a| a.name.clone()).unwrap_or_else(|| "Unknown".to_string())
                                } else {
                                    "Unassigned".to_string()
                                };
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
                                                .justify_between()
                                                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_size(px(14.)).child(t.id.split(':').next_back().unwrap_or(&t.id).to_string()))
                                                .child(
                                                    h_flex()
                                                        .gap(px(4.))
                                                        .text_color(status_color)
                                                        .child(status_icon)
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
                                        .when_some(t.payload.clone(), |d, payload| {
                                            if let Ok(dag_task) = serde_json::from_str::<crate::application::orchestration::core::DagTask>(&payload) {
                                                d.child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(theme.muted_foreground)
                                                        .child(
                                                            v_flex()
                                                                .gap_1()
                                                                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_color(theme.foreground).child(dag_task.name))
                                                                .child(div().child(dag_task.description))
                                                        )
                                                )
                                            } else {
                                                d.child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(theme.muted_foreground)
                                                        .child(payload)
                                                )
                                            }
                                        })
                                );
                            }
                        }
                        d.child(list)
                    })
            )
    }
}
