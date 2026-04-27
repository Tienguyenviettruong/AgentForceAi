use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{h_flex, ActiveTheme as _, IconName};

use super::TeamWorkspacePanel;

impl TeamWorkspacePanel {
    pub(crate) fn render_template_view(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let team = self
            .selected_team_id
            .as_ref()
            .and_then(|id| self.teams.iter().find(|t| t.id == *id));

        let title = team
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Select a template".to_string());

        let agents = if let Some(team) = team {
            self.team_service.get_team_agents(&team.id).unwrap_or_default()
        } else {
            vec![]
        };

        div()
            .h_full()
            .w_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child(
                div()
                    .p(px(16.))
                    .border_b(px(1.))
                    .border_color(theme.border)
                    .child(
                        h_flex().justify_between().items_start().child(
                            h_flex()
                                .gap(px(8.))
                                .items_start()
                                .child(
                                    div()
                                        .mt(px(4.))
                                        .text_color(theme.primary)
                                        .child(IconName::Inbox),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .child(
                                            div()
                                                .font_weight(gpui::FontWeight::BOLD)
                                                .text_size(px(18.))
                                                .child(title),
                                        )
                                        .child(
                                            h_flex()
                                                .mt(px(4.))
                                                .gap(px(4.))
                                                .text_size(px(13.))
                                                .text_color(theme.muted_foreground)
                                                .child(format!(
                                                    "{} template members",
                                                    agents.len()
                                                )),
                                        ),
                                ),
                        ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .id("template-members-scroll")
                    .overflow_y_scroll()
                    .p(px(16.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .children(agents.iter().map(|agent| {
                                div()
                                    .p(px(12.))
                                    .border(px(1.))
                                    .border_color(theme.border)
                                    .rounded_lg()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_size(px(14.))
                                                    .child(agent.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(agent.provider.clone()),
                                            ),
                                    )
                                    .when(agent.system_prompt.is_some(), |d| {
                                        d.child(
                                            div()
                                                .text_size(px(13.))
                                                .text_color(theme.muted_foreground)
                                                .child(
                                                    agent.system_prompt.as_ref().unwrap().clone(),
                                                ),
                                        )
                                    })
                            })),
                    ),
            )
    }

    pub(crate) fn render_instance(
        &self,
        instance: &crate::db::Instance,
        is_selected: bool,
        theme: &gpui_component::Theme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let id_str = instance.id.clone();

        // Use a generic dot color based on state, defaulting to blue
        let dot_color = match instance.state.as_deref() {
            Some("failed") | Some("error") => gpui::red(),
            Some("running") => gpui::green(),
            Some("paused") => gpui::Hsla::from(gpui::rgb(0xffa500)),
            _ => gpui::blue(),
        };

        let title = instance.name.clone();
        let desc = instance
            .config
            .as_deref()
            .unwrap_or("No configuration provided");
        let time = &instance.created_at;
        let status = instance.state.as_deref().unwrap_or("Initializing");

        div()
            .id(SharedString::from(id_str.clone()))
            .flex()
            .flex_col()
            .gap(px(4.))
            .py(px(6.))
            .px(px(12.))
            .rounded_lg()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_instance_id = Some(id_str.clone());
                this.selected_team_id = None;
                this.cross_team_target_instance_id = None;
                this.start_team_bus_subscription(id_str.clone(), cx);

                let mut sessions = this
                    .team_service
                    .list_sessions_for_instance(&id_str)
                    .unwrap_or_default();
                if sessions.is_empty() {
                    let agent_id = this
                        .team_service
                        .get_instance_agents(&id_str)
                        .ok()
                        .and_then(|ids| ids.first().cloned());
                    if let Some(agent_id) = agent_id {
                        if this
                            .team_service
                            .create_session_for_instance(&id_str, &agent_id)
                            .is_ok()
                        {
                            sessions = this
                                .team_service
                                .list_sessions_for_instance(&id_str)
                                .unwrap_or_default();
                        }
                    }
                }
                this.sessions_for_instance = sessions.clone();
                let session_id = sessions.first().map(|s| s.id.clone());
                this.selected_session_id = session_id.clone();
                if let Some(session_id) = session_id {
                    this.instance_active_session
                        .insert(id_str.clone(), session_id.clone());
                    let db = crate::AppState::global(cx).db.clone();
                    let msgs = db
                        .get_conversation_turns(&session_id)
                        .unwrap_or_default();
                    this.chat_histories.insert(session_id.clone(), msgs);
                    let history_len = this
                        .chat_histories
                        .get(&session_id)
                        .map(|h| h.len())
                        .unwrap_or(0);
                    this.chat_list_state =
                        gpui::ListState::new(history_len, gpui::ListAlignment::Bottom, px(200.));
                }

                let key = format!("workspace_{}", id_str);
                let db = crate::AppState::global(cx).db.clone();
                if let Ok(Some(path)) = db.get_setting(&key) {
                    this.workspace_path = Some(path);
                } else {
                    this.workspace_path = None;
                }

                cx.notify();
            }))
            .border(px(1.))
            .border_color(if is_selected {
                theme.primary.opacity(0.3)
            } else {
                theme.transparent
            })
            .bg(if is_selected {
                theme.primary.opacity(0.05)
            } else {
                theme.transparent
            })
            .hover(|s| s.bg(theme.secondary))
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(dot_color))
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_size(px(14.))
                                    .child(title),
                            ),
                    )
                    .child(
                        div()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded_md()
                            .bg(if is_selected {
                                theme.primary.opacity(0.2)
                            } else {
                                theme.secondary
                            })
                            .text_color(if is_selected {
                                theme.primary
                            } else {
                                theme.muted_foreground
                            })
                            .text_size(px(12.))
                            .child(status.to_string()),
                    ),
            )
            .child(
                div()
                    .pl(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(desc.to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground.opacity(0.5))
                            .child(time.to_string()),
                    ),
            )
    }

    pub(crate) fn render_template(
        &self,
        team: &crate::db::Team,
        theme: &gpui_component::Theme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let id_str = team.id.clone();
        let is_selected = self.selected_team_id.as_deref() == Some(id_str.as_str());

        div()
            .id(SharedString::from(id_str.clone()))
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(12.))
            .rounded_lg()
            .cursor_pointer()
            .border(px(1.))
            .border_color(if is_selected {
                theme.primary.opacity(0.3)
            } else {
                theme.transparent
            })
            .bg(if is_selected {
                theme.primary.opacity(0.05)
            } else {
                theme.transparent
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_team_id = Some(id_str.clone());
                this.selected_instance_id = None;
                cx.notify();
            }))
            .hover(|s| s.bg(theme.secondary))
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_size(px(14.))
                            .child(team.name.clone()),
                    )
                    .child(
                        h_flex()
                            .child(
                                h_flex()
                                    .child(
                                        div()
                                            .w(px(12.))
                                            .h(px(12.))
                                            .rounded_full()
                                            .bg(gpui::blue()),
                                    )
                                    .child(
                                        div()
                                            .w(px(12.))
                                            .h(px(12.))
                                            .rounded_full()
                                            .bg(gpui::green())
                                            .ml(px(-4.)),
                                    )
                                    .child(
                                        div()
                                            .w(px(12.))
                                            .h(px(12.))
                                            .rounded_full()
                                            .bg(gpui::Hsla::from(gpui::rgb(0xffa500)))
                                            .ml(px(-4.)),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child("+1")
                                    .ml(px(4.)),
                            ),
                    ),
            )
            .when(team.description.is_some(), |d| {
                d.child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(team.description.as_deref().unwrap().to_string()),
                )
            })
    }

    pub(crate) fn render_teams_column(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .h_full()
            .w_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            // .border_r(px(1.))
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                div()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(16.))
                    .border_b(px(1.))
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .child(
                                gpui::svg()
                                    .path("teams.svg")
                                    .size(px(16.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_size(px(14.))
                                    .child("AGENT TEAMS"),
                            ),
                    )
                    .child(
                        Button::new("new-instance")
                            .ghost()
                            .tooltip("New Instance")
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|_this, _, window, cx| {
                                let db = crate::AppState::global(cx).db.clone();
                                crate::ui::components::dialogs::open_new_instance_dialog(
                                    db,
                                    cx.entity().clone(),
                                    window,
                                    cx,
                                    |view: &mut crate::ui::panels::team_workspace::TeamWorkspacePanel, cx: &mut Context<crate::ui::panels::team_workspace::TeamWorkspacePanel>| view.reload(cx),
                                );
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .id("instances-scroll")
                    .overflow_scroll()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                h_flex()
                                    .id("instances-header")
                                    .justify_between()
                                    .px(px(8.))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.instances_expanded = !this.instances_expanded;
                                        cx.notify();
                                    }))
                                    .child(
                                        h_flex()
                                            .gap(px(6.))
                                            .text_color(theme.muted_foreground)
                                            .child(if self.instances_expanded {
                                                IconName::ChevronDown
                                            } else {
                                                IconName::ChevronRight
                                            })
                                            .child(IconName::Inbox)
                                            .child(div().text_size(px(13.)).child(format!(
                                                "Instances ({})",
                                                self.instances.len()
                                            ))),
                                    ),
                            )
                            .when(self.instances_expanded, |d| {
                                let mut list = div().flex().flex_col().gap(px(4.));
                                for instance in &self.instances {
                                    let is_selected = self.selected_instance_id.as_deref()
                                        == Some(instance.id.as_str());
                                    list = list.child(self.render_instance(
                                        instance,
                                        is_selected,
                                        theme,
                                        cx,
                                    ));
                                }
                                d.child(list)
                            }),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .px(px(16.))
                    .child(div().h(px(1.)).bg(theme.border).w_full())
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child("Modules")
                            .px(px(16.)),
                    )
                    .child(div().h(px(1.)).bg(theme.border).w_full()),
            )
            .child(
                div()
                    .flex_1()
                    .id("templates-scroll")
                    .overflow_scroll()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                h_flex()
                                    .id("templates-header")
                                    .gap(px(6.))
                                    .px(px(8.))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.templates_expanded = !this.templates_expanded;
                                        cx.notify();
                                    }))
                                    .text_color(theme.muted_foreground)
                                    .child(if self.templates_expanded {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    })
                                    .child(IconName::Inbox)
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .child(format!("Templates ({})", self.teams.len())),
                                    ),
                            )
                            .when(self.templates_expanded, |d| {
                                let mut list = div().flex().flex_col().gap(px(4.));
                                for team in &self.teams {
                                    list = list.child(self.render_template(team, theme, cx));
                                }
                                d.child(list)
                            }),
                    ),
            )
            .child(
                div()
                    .p(px(8.))
                    .border_t(px(1.))
                    .border_color(theme.border)
                    .child(
                        div()
                            .id("new-team-btn")
                            .w_full()
                            .py(px(4.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(6.))
                            .border(px(1.))
                            .border_color(theme.border)
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.secondary))
                            .text_color(theme.muted_foreground)
                            .child(IconName::Plus)
                            .child(div().text_size(px(16.)).child("New Team"))
                            .on_click(cx.listener(|_this, _, window, cx| {
                                let db = crate::AppState::global(cx).db.clone();
                                    crate::ui::components::dialogs::open_new_team_dialog(
                                        db,
                                    cx.entity().clone(),
                                    window,
                                    cx,
                                    |view: &mut crate::ui::panels::team_workspace::TeamWorkspacePanel, cx: &mut Context<crate::ui::panels::team_workspace::TeamWorkspacePanel>| view.reload(cx),
                                );
                            })),
                    ),
            )
    }
}
