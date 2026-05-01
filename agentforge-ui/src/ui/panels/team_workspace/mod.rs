use crate::core::traits::database::DatabasePort;
use gpui::AppContext;
use gpui::{
    div, px, Context, Entity, FocusHandle, IntoElement, ListAlignment, ListState, ParentElement,
    Render, Styled, Window,
};
use gpui_component::button::ButtonVariants;
use gpui_component::resizable::{h_resizable, resizable_panel};
use gpui_component::{button::Button, h_flex, ActiveTheme as _, IconName};

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::db::{Agent, SessionRecord, Team};

mod chat;
mod members;
mod teams;

pub struct TeamWorkspacePanel {
    focus_handle: FocusHandle,
    pub(crate) team_bus: Arc<crate::infrastructure::message_bus::routing::TeamBusRouter>,
    pub(crate) team_service: Arc<crate::application::services::team_service::TeamService>,
    pub(crate) teams: Vec<Team>,
    pub(crate) instances: Vec<crate::db::Instance>,
    pub(crate) selected_team_id: Option<String>,
    pub(crate) selected_instance_id: Option<String>,
    pub(crate) selected_session_id: Option<String>,
    pub(crate) sessions_for_instance: Vec<SessionRecord>,
    pub(crate) instance_active_session: HashMap<String, String>,
    pub(crate) agents: Vec<Agent>,
    pub(crate) workspace_path: Option<String>,
    pub(crate) cross_team_target_instance_id: Option<String>,
    pub(crate) cross_team_peer_instance_id: Option<String>,
    pub(crate) cross_team_cases: Vec<crate::core::models::CrossTeamCaseRecord>,
    pub(crate) selected_cross_team_case_id: Option<String>,
    pub(crate) show_history_sheet: bool,
    instances_expanded: bool,
    templates_expanded: bool,
    expanded_groups: HashSet<String>,
    chat_active_tab: usize,
    members_active_tab: usize,
    chat_histories: std::collections::HashMap<String, Vec<crate::providers::ChatMessage>>,
    chat_display_rows: std::collections::HashMap<String, Vec<ChatDisplayRow>>,
    expanded_messages: std::collections::HashSet<String>,
    expanded_threads: std::collections::HashSet<String>,
    chat_bus_epoch: Arc<AtomicUsize>,
    chat_input_state: Entity<gpui_component::input::InputState>,
    chat_list_state: ListState,
    pub(crate) attached_files: Vec<String>,
    pub(crate) is_workspace_dropdown_open: bool,
    pub(crate) recent_workspaces: Vec<String>,
    pub(crate) debate_mode: bool,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub(crate) office_webview: Option<Entity<gpui_component::webview::WebView>>,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub(crate) office_webview_error: Option<String>,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub(crate) office_webview_init_attempted: bool,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub(crate) office_webview_disabled: bool,
}

impl TeamWorkspacePanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _db = crate::AppState::global(cx).db.clone();
        let team_bus = crate::AppState::global(cx).team_bus.clone();
        let team_service = crate::AppState::global(cx).team_service.clone();

        let chat_input_state = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .multi_line(true)
                .auto_grow(1, 5)
                .placeholder("Describe new goal... (Enter to send, Shift+Enter for new line)")
        });

        let mut panel = Self {
            focus_handle: cx.focus_handle(),
            team_bus,
            team_service,
            teams: Vec::new(),
            instances: Vec::new(),
            selected_team_id: None,
            selected_instance_id: None,
            selected_session_id: None,
            sessions_for_instance: Vec::new(),
            instance_active_session: HashMap::new(),
            agents: Vec::new(),
            workspace_path: None,
            cross_team_target_instance_id: None,
            cross_team_peer_instance_id: None,
            cross_team_cases: Vec::new(),
            selected_cross_team_case_id: None,
            show_history_sheet: false,
            instances_expanded: true,
            templates_expanded: true,
            expanded_groups: HashSet::new(),
            chat_active_tab: 0,
            members_active_tab: 0,
            chat_histories: std::collections::HashMap::new(),
            chat_display_rows: std::collections::HashMap::new(),
            expanded_messages: std::collections::HashSet::new(),
            expanded_threads: std::collections::HashSet::new(),
            chat_bus_epoch: Arc::new(AtomicUsize::new(0)),
            chat_input_state,
            chat_list_state: ListState::new(0, ListAlignment::Bottom, px(200.)),
            attached_files: Vec::new(),
            is_workspace_dropdown_open: false,
            recent_workspaces: Vec::new(),
            debate_mode: false,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            office_webview: None,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            office_webview_error: None,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            office_webview_init_attempted: false,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            office_webview_disabled: std::env::var("AGENTFORGE_DISABLE_OFFICE_WEBVIEW")
                .ok()
                .map(|v| {
                    let v = v.to_ascii_lowercase();
                    v == "1" || v == "true" || v == "yes"
                })
                .unwrap_or(false),
        };

        panel.reload(cx);

        cx.subscribe_in(
            &panel.chat_input_state,
            window,
            |this, _, event: &gpui_component::input::InputEvent, window, cx| {
                if let gpui_component::input::InputEvent::PressEnter { secondary } = event {
                    if !secondary {
                        this.handle_send_chat(window, cx);
                    }
                }
            },
        )
        .detach();

        panel
    }

    fn parse_cross_team_payload(content: &str) -> Option<serde_json::Value> {
        if !content.starts_with("[CROSS_TEAM_HANDOFF]") {
            return None;
        }
        let payload_str = content.trim_start_matches("[CROSS_TEAM_HANDOFF]").trim();
        serde_json::from_str::<serde_json::Value>(payload_str).ok()
    }

    pub(crate) fn rebuild_chat_display(&mut self, session_id: &str) {
        let history = match self.chat_histories.get(session_id) {
            Some(h) => h,
            None => {
                self.chat_display_rows.remove(session_id);
                return;
            }
        };

        let mut first_index_for_correlation: HashMap<String, usize> = HashMap::new();
        let mut thread_indices: HashMap<String, Vec<usize>> = HashMap::new();
        let mut thread_meta: HashMap<String, (String, String, bool, bool, String)> = HashMap::new();

        for (ix, msg) in history.iter().enumerate() {
            if let Some(payload) = Self::parse_cross_team_payload(msg.content.as_ref()) {
                let correlation_id = payload
                    .get("correlation_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if correlation_id.is_empty() {
                    continue;
                }
                first_index_for_correlation
                    .entry(correlation_id.clone())
                    .or_insert(ix);
                thread_indices
                    .entry(correlation_id.clone())
                    .or_default()
                    .push(ix);
                let handoff_type = payload
                    .get("handoff_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("handoff")
                    .to_string();
                let from_team = payload
                    .get("from_team")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let briefing = payload
                    .get("briefing_package")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let preview = briefing
                    .lines()
                    .map(|l| l.trim())
                    .find(|l| !l.is_empty())
                    .unwrap_or("")
                    .to_string();
                let preview = {
                    let mut end = preview.len();
                    let mut chars = 0usize;
                    for (i, _) in preview.char_indices() {
                        if chars == 96 {
                            end = i;
                            break;
                        }
                        chars += 1;
                    }
                    if chars >= 96 && end < preview.len() {
                        format!("{}…", &preview[..end])
                    } else {
                        preview
                    }
                };
                let has_request = handoff_type == "review_request";
                let has_response = handoff_type == "review_response";
                thread_meta
                    .entry(correlation_id)
                    .and_modify(|e| {
                        e.2 = e.2 || has_request;
                        e.3 = e.3 || has_response;
                        if !from_team.is_empty() {
                            e.0 = from_team.clone();
                        }
                        if e.1.is_empty() && !preview.is_empty() {
                            e.1 = preview.clone();
                        }
                        e.4 = handoff_type.clone();
                    })
                    .or_insert((from_team, preview, has_request, has_response, handoff_type));
            }
        }

        let mut by_first_index: HashMap<usize, String> = HashMap::new();
        let mut skip_indices: HashSet<usize> = HashSet::new();
        for (cid, first_ix) in first_index_for_correlation.iter() {
            by_first_index.insert(*first_ix, cid.clone());
            if let Some(ixs) = thread_indices.get(cid) {
                for ix in ixs {
                    if ix != first_ix {
                        skip_indices.insert(*ix);
                    }
                }
            }
        }

        let mut rows = Vec::new();
        for ix in 0..history.len() {
            if skip_indices.contains(&ix) {
                continue;
            }
            if let Some(cid) = by_first_index.get(&ix) {
                let count = thread_indices.get(cid).map(|v| v.len()).unwrap_or(1);
                let (from_team, preview, has_request, has_response, handoff_type) =
                    thread_meta.get(cid).cloned().unwrap_or_else(|| {
                        (
                            "".to_string(),
                            "".to_string(),
                            false,
                            false,
                            "handoff".to_string(),
                        )
                    });
                rows.push(ChatDisplayRow::CrossTeamThreadHeader {
                    correlation_id: cid.clone(),
                    handoff_type,
                    from_team,
                    count,
                    preview,
                    has_request,
                    has_response,
                });
                if self.expanded_threads.contains(cid) {
                    if let Some(ixs) = thread_indices.get(cid) {
                        for ix in ixs {
                            rows.push(ChatDisplayRow::Message {
                                source_index: *ix,
                                msg: history[*ix].clone(),
                            });
                        }
                    }
                }
                continue;
            }
            rows.push(ChatDisplayRow::Message {
                source_index: ix,
                msg: history[ix].clone(),
            });
        }

        self.chat_display_rows.insert(session_id.to_string(), rows);
    }

    pub(crate) fn start_team_bus_subscription(
        &mut self,
        instance_id: String,
        cx: &mut Context<Self>,
    ) {
        let epoch = self.chat_bus_epoch.fetch_add(1, Ordering::SeqCst) + 1;
        let epoch_counter = self.chat_bus_epoch.clone();
        let team_bus = self.team_bus.clone();
        let view = cx.entity().clone();
        cx.spawn(async move |_, cx| {
            let mut rx = team_bus.subscribe_broadcast(&instance_id).await;
            loop {
                if epoch_counter.load(Ordering::SeqCst) != epoch {
                    break;
                }
                match rx.recv().await {
                    Ok(msg) => {
                        if msg.sender_member_id == "user" || msg.sender_member_id == "assistant" {
                            continue;
                        }
                        let _ = cx.update(|cx| {
                            view.update(cx, |this: &mut TeamWorkspacePanel, cx| {
                                let db = crate::AppState::global(cx).db.clone();
                                let agent_name = db
                                    .get_agent(&msg.sender_member_id)
                                    .ok()
                                    .flatten()
                                    .map(|a| a.name)
                                    .unwrap_or_else(|| msg.sender_member_id.clone());
                                let session_id = this
                                    .instance_active_session
                                    .get(&instance_id)
                                    .cloned()
                                    .or_else(|| this.selected_session_id.clone());
                                if let Some(session_id) = session_id {
                                    {
                                        let history = this
                                            .chat_histories
                                            .entry(session_id.clone())
                                            .or_default();
                                        history.push(crate::providers::ChatMessage {
                                            role: "assistant".into(),
                                            content: msg.content.into(),
                                            agent_name: Some(agent_name.into()),
                                        });
                                    }
                                    this.rebuild_chat_display(&session_id);
                                    if this.selected_session_id.as_deref()
                                        == Some(session_id.as_str())
                                    {
                                        let display_len = this
                                            .chat_display_rows
                                            .get(&session_id)
                                            .map(|v| v.len())
                                            .unwrap_or(0);
                                        this.chat_list_state = gpui::ListState::new(
                                            display_len,
                                            gpui::ListAlignment::Bottom,
                                            px(200.),
                                        );
                                    }
                                }
                                cx.notify();
                            });
                        });
                    }
                    Err(_) => break,
                }
            }
        })
        .detach();
    }

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let db = crate::AppState::global(cx).db.clone();
        if let Ok(teams) = self.team_service.list_teams() {
            self.teams = teams;
        }
        if let Ok(instances) = self.team_service.list_instances() {
            self.instances = instances;
        }
        if let Ok(agents) = db.list_agents() {
            self.agents = agents;
        }

        if let Some(instance_id) = &self.selected_instance_id {
            let mut sessions = self
                .team_service
                .list_sessions_for_instance(instance_id)
                .unwrap_or_default();

            if sessions.is_empty() {
                let agent_id = self
                    .team_service
                    .get_instance_agents(instance_id)
                    .ok()
                    .and_then(|ids| ids.first().cloned());
                if let Some(agent_id) = agent_id {
                    if let Ok(session_id) = self
                        .team_service
                        .create_session_for_instance(instance_id, &agent_id)
                    {
                        let _ = db.ensure_session(&session_id, &agent_id, Some(instance_id));
                        sessions = self
                            .team_service
                            .list_sessions_for_instance(instance_id)
                            .unwrap_or_default();
                    }
                }
            }

            self.sessions_for_instance = sessions.clone();

            let next_session_id = self
                .selected_session_id
                .clone()
                .filter(|sid| sessions.iter().any(|s| s.id == *sid))
                .or_else(|| sessions.first().map(|s| s.id.clone()));

            self.selected_session_id = next_session_id.clone();
            if let Some(session_id) = next_session_id {
                self.instance_active_session
                    .insert(instance_id.clone(), session_id.clone());
                let msgs = db.get_conversation_turns(&session_id).unwrap_or_default();
                self.chat_histories.insert(session_id.clone(), msgs);
                let history_len = self
                    .chat_histories
                    .get(&session_id)
                    .map(|h| h.len())
                    .unwrap_or(0);
                self.chat_list_state =
                    gpui::ListState::new(history_len, gpui::ListAlignment::Bottom, px(200.));
            }
        } else {
            self.sessions_for_instance = Vec::new();
            self.selected_session_id = None;
        }

        if let Some(instance_id) = &self.selected_instance_id {
            let key = format!("workspace_{}", instance_id);
            if let Ok(Some(path)) = db.get_setting(&key) {
                self.workspace_path = Some(path);
            } else {
                self.workspace_path = None;
            }

            let target_key = format!("cross_team_target_{}", instance_id);
            self.cross_team_target_instance_id = db
                .get_setting(&target_key)
                .ok()
                .flatten()
                .filter(|v| !v.trim().is_empty());
            let peer_key = format!("cross_team_peer_{}", instance_id);
            self.cross_team_peer_instance_id = db
                .get_setting(&peer_key)
                .ok()
                .flatten()
                .filter(|v| !v.trim().is_empty());

            self.cross_team_cases = db
                .list_cross_team_cases(instance_id, 50)
                .unwrap_or_default();
            if let Some(sel) = self.selected_cross_team_case_id.clone() {
                if !self
                    .cross_team_cases
                    .iter()
                    .any(|c| c.correlation_id == sel)
                {
                    self.selected_cross_team_case_id = None;
                }
            }
        } else {
            self.workspace_path = None;
            self.cross_team_target_instance_id = None;
            self.cross_team_peer_instance_id = None;
            self.cross_team_cases = Vec::new();
            self.selected_cross_team_case_id = None;
        }

        cx.notify();
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ChatDisplayRow {
    Message {
        source_index: usize,
        msg: crate::providers::ChatMessage,
    },
    CrossTeamThreadHeader {
        correlation_id: String,
        handoff_type: String,
        from_team: String,
        count: usize,
        preview: String,
        has_request: bool,
        has_response: bool,
    },
}
impl Render for TeamWorkspacePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let breadcrumb_title = if let Some(instance_id) = &self.selected_instance_id {
            let inst = self.instances.iter().find(|i| i.id == *instance_id);
            inst.map(|i| i.name.clone())
                .unwrap_or_else(|| instance_id[..std::cmp::min(6, instance_id.len())].to_string())
        } else {
            "Agent Workspaces".to_string()
        };

        let header = h_flex()
            .w_full()
            .h(px(40.))
            .px(px(16.))
            .items_center()
            .justify_between()
            .border_b(px(1.))
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                h_flex()
                    .gap(px(8.))
                    .child(div().text_color(theme.muted_foreground).child("Teams"))
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .child(IconName::ChevronRight),
                    )
                    .child(div().child(breadcrumb_title)),
            )
            .child(
                h_flex()
                    .gap(px(12.))
                    .child(Button::new("title-more").ghost().icon(IconName::Ellipsis)),
            );

        let active_team_id = self.selected_team_id.clone().or_else(|| {
            self.selected_instance_id.as_ref().and_then(|iid| {
                self.instances
                    .iter()
                    .find(|i| i.id == *iid)
                    .map(|i| i.team_id.clone())
            })
        });

        let resizer_id = if active_team_id.is_some() {
            "team-workspace-opened"
        } else {
            "team-workspace-closed"
        };

        let mut container = h_resizable(resizer_id).child(
            resizable_panel()
                .size(px(248.))
                .size_range(px(220.)..px(272.))
                .child(self.render_teams_column(cx)),
        );

        if let Some(_instance_id) = &self.selected_instance_id {
            container = container
                .child(
                    resizable_panel()
                        .size(px(280.))
                        .size_range(px(280.)..px(300.))
                        .child(self.render_members_column(cx)),
                )
                .child(resizable_panel().child(self.render_chat_column(window, cx)));
        } else if self.selected_team_id.is_some() {
            container = container.child(resizable_panel().child(self.render_template_view(cx)));
        } else {
            container = container.child(
                resizable_panel().child(
                    div()
                        .flex_1()
                        .flex()
                        .h_full()
                        .items_center()
                        .justify_center()
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            div()
                                .text_size(px(14.))
                                .child("Select a team to see members/tasks"),
                        ),
                ),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(theme.background)
            .child(header)
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h(px(0.))
                    .min_w(px(0.))
                    .overflow_hidden()
                    .child(container),
            )
    }
}
