use gpui::*;
use gpui_component::{ActiveTheme as _, 
    button::*,
    input::*,
    select::*,
    form::*,
    notification::NotificationType, WindowExt,
};
use std::sync::Arc;
use chrono::Utc;
use std::collections::HashSet;
use gpui_component::v_flex;
use crate::core::traits::database::DatabasePort;
use crate::db::{Team, Agent};

pub fn open_new_team_dialog<V: 'static>(
    db: Arc<dyn DatabasePort>,
    view: Entity<V>,
    window: &mut Window,
    cx: &mut Context<V>,
    on_success: impl Fn(&mut V, &mut Context<V>) + 'static + Clone,
) {
    let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Team name"));
    let description_input = cx.new(|cx| InputState::new(window, cx).placeholder("Description"));
    let objectives_input = cx.new(|cx| InputState::new(window, cx).placeholder("Objectives"));

    let agents = db.list_agents().unwrap_or_default();
    let state = cx.new(|_cx| ManageTeamState {
        team_id: uuid::Uuid::new_v4().to_string(),
        agents,
        selected_agents: HashSet::new(),
    });

    window.open_dialog(cx, move |dialog, _window, cx| {
        let view_save = view.clone();
        let db_save = db.clone();
        let on_success_save = on_success.clone();

        dialog
            .title("Create Team")
            .w(px(520.))
            .child(
                v_form()
                    .gap(px(12.))
                    .py(px(8.))
                    .child(
                        field()
                            .label("Name")
                            .required(true)
                            .child(Input::new(&name_input)),
                    )
                    .child(
                        field()
                            .label("Description")
                            .child(Input::new(&description_input)),
                    )
                    .child(
                        field()
                            .label("Objectives")
                            .child(Input::new(&objectives_input)),
                    ),
            )
            .child(
                v_flex()
                    .gap(px(12.))
                    .pt(px(8.))
                    .child(div().font_weight(FontWeight::BOLD).child("Agents"))
                    .child(
                        v_flex()
                            .gap(px(8.))
                            .max_h(px(280.))
                            .id("new-team-agent-scroll")
                            .overflow_y_scroll()
                            .children(state.read(cx).agents.iter().map(|agent| {
                                let agent_id = agent.id.clone();
                                let is_selected = state.read(cx).selected_agents.contains(&agent_id);
                                let state_clone = state.clone();
                                gpui_component::checkbox::Checkbox::new(SharedString::from(agent.id.clone()))
                                    .label(agent.name.clone())
                                    .checked(is_selected)
                                    .on_click(move |checked, _window, cx| {
                                        state_clone.update(cx, |s, cx| {
                                            if *checked {
                                                s.selected_agents.insert(agent_id.clone());
                                            } else {
                                                s.selected_agents.remove(&agent_id);
                                            }
                                            cx.notify();
                                        });
                                    })
                            })),
                    ),
            )
            .footer({
                let view_save = view_save.clone();
                let db_save = db_save.clone();
                let name_input = name_input.clone();
                let description_input = description_input.clone();
                let objectives_input = objectives_input.clone();
                let on_success_save = on_success_save.clone();
                let state_save = state.clone();

                move |_, _, _, _| {
                    let view_save2 = view_save.clone();
                    let db_save2 = db_save.clone();
                    let name_input2 = name_input.clone();
                    let description_input2 = description_input.clone();
                    let objectives_input2 = objectives_input.clone();
                    let on_success_save2 = on_success_save.clone();
                    let state_save2 = state_save.clone();

                    vec![
                        Button::new("cancel-team")
                            .label("Cancel")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            })
                            .into_any_element(),
                        Button::new("save-team")
                            .primary()
                            .label("Save Team")
                            .on_click({
                                let view_save3 = view_save2.clone();
                                let db_save3 = db_save2.clone();
                                let name_input3 = name_input2.clone();
                                let description_input3 = description_input2.clone();
                                let objectives_input3 = objectives_input2.clone();
                                let on_success_save3 = on_success_save2.clone();
                                let state_save3 = state_save2.clone();

                                move |_ev, window, cx| {
                                    let name = name_input3.read(cx).text().to_string();
                                    let name = name.trim().to_string();
                                    if name.is_empty() {
                                        window.push_notification(
                                            (NotificationType::Error, "Team name is required."),
                                            cx,
                                        );
                                        return;
                                    }
 
                                     if state_save3.read(cx).selected_agents.is_empty() {
                                         window.push_notification(
                                             (NotificationType::Error, "Please select at least one agent."),
                                             cx,
                                         );
                                         return;
                                     }

                                    let now = Utc::now().to_rfc3339();
                                    let description =
                                        description_input3.read(cx).text().to_string();
                                    let objectives =
                                        objectives_input3.read(cx).text().to_string();
                                    let team_id = uuid::Uuid::new_v4().to_string();
                                    let team = Team {
                                        id: team_id.clone(),
                                        name,
                                        description: {
                                            let t = description.trim().to_string();
                                            if t.is_empty() {
                                                None
                                            } else {
                                                Some(t)
                                            }
                                        },
                                        objectives: {
                                            let t = objectives.trim().to_string();
                                            if t.is_empty() {
                                                None
                                            } else {
                                                Some(t)
                                            }
                                        },
                                        created_at: now.clone(),
                                        updated_at: now,
                                    };

                                    if db_save3.insert_team(&team).is_ok() {
                                        let selected_agents: Vec<String> =
                                            state_save3.read(cx).selected_agents.iter().cloned().collect();
                                        for agent_id in selected_agents {
                                            let _ = db_save3.assign_agent_to_team(&team_id, &agent_id);
                                        }
                                        view_save3.update(cx, on_success_save3.clone());
                                        window.close_dialog(cx);
                                        window.push_notification(
                                            (
                                                NotificationType::Success,
                                                "Team created successfully.",
                                            ),
                                            cx,
                                        );
                                    } else {
                                        window.push_notification(
                                            (NotificationType::Error, "Failed to create team."),
                                            cx,
                                        );
                                    }
                                }
                            })
                            .into_any_element(),
                    ]
                }
            })
    });
}

pub fn open_new_agent_dialog<V: 'static>(
    db: Arc<dyn DatabasePort>,
    view: Entity<V>,
    window: &mut Window,
    cx: &mut Context<V>,
    on_success: impl Fn(&mut V, &mut Context<V>) + 'static + Clone,
) {
    let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Agent name"));
    let system_prompt_input =
        cx.new(|cx| InputState::new(window, cx).placeholder("System prompt (optional)"));
    let role_input = cx.new(|cx| InputState::new(window, cx).placeholder("Role / Position"));
    let details_input = cx.new(|cx| InputState::new(window, cx).placeholder("Agent Details / Info"));

    let providers: Vec<SharedString> = db
        .list_providers()
        .unwrap_or_default()
        .into_iter()
        .map(|p| SharedString::from(format!("{} / {}", p.provider_name, p.model)))
        .collect();
    let provider_select = cx.new(|cx| SelectState::new(providers, None, window, cx));

    window.open_dialog(cx, move |dialog, _window, _cx| {
        let view_save = view.clone();
        let db_save = db.clone();
        let on_success_save = on_success.clone();

        let theme = _cx.theme().clone();
        dialog
            .title("Create Agent")
            .w(px(520.))
            .child(
                v_form()
                    .gap(px(12.))
                    .py(px(8.))
                    .child(
                        field()
                            .label("Name")
                            .required(true)
                            .child(Input::new(&name_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Tên định danh của Agent (VD: Backend Developer, Data Analyst).")),
                    )
                    .child(
                        field()
                            .label("Role / Position")
                            .child(Input::new(&role_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Vai trò chuyên môn trong dự án (VD: DEV, QA, PM). Dùng cho TeamBus Router.")),
                    )
                    .child(
                        field()
                            .label("Agent Details")
                            .child(Input::new(&details_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Thông tin bổ sung, kỹ năng hoặc giới hạn của Agent.")),
                    )
                    .child(field().label("Provider / Model").child(
                        Select::new(&provider_select).placeholder("Select Provider / Model"),
                    )
                    .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Mô hình LLM sẽ cung cấp năng lực cho Agent này (VD: Claude-3-Opus, GPT-4o).")))
                    .child(
                        field()
                            .label("System Prompt")
                            .child(Input::new(&system_prompt_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Chỉ dẫn hệ thống cốt lõi để định hình hành vi, phong cách trả lời và luồng suy nghĩ của Agent.")),
                    ),
            )
            .footer({
                let view_save = view_save.clone();
                let db_save = db_save.clone();
                let name_input = name_input.clone();
                let system_prompt_input = system_prompt_input.clone();
                let role_input = role_input.clone();
                let details_input = details_input.clone();
                let provider_select = provider_select.clone();
                let on_success_save = on_success_save.clone();

                move |_, _, _, _| {
                    let view_save2 = view_save.clone();
                    let db_save2 = db_save.clone();
                    let name_input2 = name_input.clone();
                    let system_prompt_input2 = system_prompt_input.clone();
                    let role_input2 = role_input.clone();
                    let details_input2 = details_input.clone();
                    let provider_select2 = provider_select.clone();
                    let on_success_save2 = on_success_save.clone();

                    vec![
                        Button::new("cancel-agent")
                            .label("Cancel")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            })
                            .into_any_element(),
                        Button::new("save-agent")
                            .primary()
                            .label("Save Agent")
                            .on_click({
                                let view_save3 = view_save2.clone();
                                let db_save3 = db_save2.clone();
                                let name_input3 = name_input2.clone();
                                let system_prompt_input3 = system_prompt_input2.clone();
                                let role_input3 = role_input2.clone();
                                let details_input3 = details_input2.clone();
                                let provider_select3 = provider_select2.clone();
                                let on_success_save3 = on_success_save2.clone();

                                move |_ev, window, cx| {
                                    let name = name_input3.read(cx).text().to_string();
                                    let name = name.trim().to_string();
                                    if name.is_empty() {
                                        window.push_notification(
                                            (
                                                NotificationType::Error,
                                                "Agent name is required.",
                                            ),
                                            cx,
                                        );
                                        return;
                                    }

                                    let provider = provider_select3
                                        .read(cx)
                                        .selected_value()
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| "unconfigured".to_string());

                                    let now = Utc::now().to_rfc3339();
                                    
                                    let role = role_input3.read(cx).text().to_string();
                                    let details = details_input3.read(cx).text().to_string();
                                    let config_json = serde_json::json!({
                                        "role": role,
                                        "details": details
                                    });
let system_prompt =
                                        system_prompt_input3.read(cx).text().to_string();
                                    let agent = Agent {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        name,
                                        provider,
                                        system_prompt: {
                                            let t = system_prompt.trim().to_string();
                                            if t.is_empty() {
                                                None
                                            } else {
                                                Some(t)
                                            }
                                        },
                                        config: Some(config_json.to_string()),
                                        status: "offline".to_string(),
                                        created_at: now.clone(),
                                        updated_at: now,
                                    };

                                    if db_save3.insert_agent(&agent).is_ok() {
                                        view_save3.update(cx, on_success_save3.clone());
                                        window.close_dialog(cx);
                                        window.push_notification(
                                            (
                                                NotificationType::Success,
                                                "Agent created successfully.",
                                            ),
                                            cx,
                                        );
                                    } else {
                                        window.push_notification(
                                            (
                                                NotificationType::Error,
                                                "Failed to create agent.",
                                            ),
                                            cx,
                                        );
                                    }
                                }
                            })
                            .into_any_element(),
                    ]
                }
            })
    });
}
pub fn open_new_instance_dialog<V: 'static>(
    db: Arc<dyn DatabasePort>,
    view: Entity<V>,
    window: &mut Window,
    cx: &mut Context<V>,
    on_success: impl Fn(&mut V, &mut Context<V>) + 'static + Clone,
) {
    let teams = db.list_teams().unwrap_or_default();
    let template_options: Vec<SharedString> = teams.iter().map(|t| t.name.clone().into()).collect();
    let template_select = cx.new(|cx| SelectState::new(template_options, None, window, cx));

    let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Instance name (e.g. Test Run)"));
    let config_input = cx.new(|cx| InputState::new(window, cx).placeholder("Configuration / Context"));

    window.open_dialog(cx, move |dialog, _window, _cx| {
        let view_save = view.clone();
        let db_save = db.clone();
        let on_success_save = on_success.clone();

        dialog
            .title("Create Instance")
            .w(px(520.))
            .child(
                v_form()
                    .gap(px(12.))
                    .py(px(8.))
                    .child(
                        field()
                            .label("Template (Team)")
                            .required(true)
                            .child(Select::new(&template_select).placeholder("Select Template")),
                    )
                    .child(
                        field()
                            .label("Instance Name")
                            .required(true)
                            .child(Input::new(&name_input)),
                    )
                    .child(
                        field()
                            .label("Configuration")
                            .child(Input::new(&config_input)),
                    ),
            )
            .footer({
                let view_save = view_save.clone();
                let db_save = db_save.clone();
                let name_input = name_input.clone();
                let config_input = config_input.clone();
                let template_select = template_select.clone();
                let on_success_save = on_success_save.clone();
                let teams_save = teams.clone();

                move |_, _, _, _| {
                    let view_save2 = view_save.clone();
                    let db_save2 = db_save.clone();
                    let name_input2 = name_input.clone();
                    let config_input2 = config_input.clone();
                    let template_select2 = template_select.clone();
                    let on_success_save2 = on_success_save.clone();
                    let teams_save2 = teams_save.clone();

                    vec![
                        Button::new("cancel-instance")
                            .label("Cancel")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            })
                            .into_any_element(),
                        Button::new("save-instance")
                            .primary()
                            .label("Create Instance")
                            .on_click({
                                let view_save3 = view_save2.clone();
                                let db_save3 = db_save2.clone();
                                let name_input3 = name_input2.clone();
                                let config_input3 = config_input2.clone();
                                let template_select3 = template_select2.clone();
                                let on_success_save3 = on_success_save2.clone();
                                let teams_save3 = teams_save2.clone();

                                move |_ev, window, cx| {
                                    let selected_template_name = template_select3.read(cx).selected_value();
                                    let team_id = selected_template_name.and_then(|name| {
                                        teams_save3.iter().find(|t| t.name == *name).map(|t| t.id.clone())
                                    });

                                    if team_id.is_none() {
                                        window.push_notification(
                                            (NotificationType::Error, "Please select a template."),
                                            cx,
                                        );
                                        return;
                                    }

                                    let name = name_input3.read(cx).text().to_string();
                                    let name = name.trim().to_string();
                                    if name.is_empty() {
                                        window.push_notification(
                                            (NotificationType::Error, "Instance name is required."),
                                            cx,
                                        );
                                        return;
                                    }

                                    let config = config_input3.read(cx).text().to_string();
                                    let config = config.trim().to_string();
                                    let config_opt = if config.is_empty() { None } else { Some(config.as_str()) };

                                    let instance_id = format!("{}-{}", name.replace(" ", "-").to_lowercase(), uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>());

                                    if db_save3.create_instance(&instance_id, &name, &team_id.unwrap(), config_opt, Some("running")).is_ok() {
                                        view_save3.update(cx, on_success_save3.clone());
                                        window.close_dialog(cx);
                                        window.push_notification(
                                            (NotificationType::Success, "Instance created successfully."),
                                            cx,
                                        );
                                    } else {
                                        window.push_notification(
                                            (NotificationType::Error, "Failed to create instance."),
                                            cx,
                                        );
                                    }
                                }
                            })
                            .into_any_element(),
                    ]
                }
            })
    });
}

pub struct ManageTeamState {
    pub team_id: String,
    pub agents: Vec<Agent>,
    pub selected_agents: HashSet<String>,
}

pub fn open_manage_team_dialog<V: 'static>(
    db: Arc<dyn DatabasePort>,
    view: Entity<V>,
    team_id: String,
    window: &mut Window,
    cx: &mut Context<V>,
    on_success: impl Fn(&mut V, &mut Context<V>) + 'static + Clone,
) {
    let agents = db.list_agents().unwrap_or_default();
    let current_team_agents = db.get_team_agents(&team_id).unwrap_or_default();
    let mut selected_agents = HashSet::new();
    for agent_id in current_team_agents {
        selected_agents.insert(agent_id);
    }

    let state = cx.new(|_cx| ManageTeamState {
        team_id: team_id.clone(),
        agents,
        selected_agents,
    });

    window.open_dialog(cx, move |dialog, _window, cx| {
        let view_save = view.clone();
        let db_save = db.clone();
        let on_success_save = on_success.clone();
        let state_save = state.clone();
        let team_id_save = team_id.clone();

        dialog
            .title("Manage Team Members")
            .w(px(400.))
            .child(
                v_flex()
                    .gap(px(12.))
                    .py(px(8.))
                    .max_h(px(400.))
                    .id("manage-team-scroll").overflow_y_scroll()
                    .children(state.read(cx).agents.iter().map(|agent| {
                        let agent_id = agent.id.clone();
                        let is_selected = state.read(cx).selected_agents.contains(&agent_id);
                        let state_clone = state.clone();
                        
                        // We use a checkbox for each agent
                        gpui_component::checkbox::Checkbox::new(SharedString::from(agent.id.clone()))
                            .label(agent.name.clone())
                            .checked(is_selected)
                            .on_click(move |checked, _window, cx| {
                                state_clone.update(cx, |s, cx| {
                                    if *checked {
                                        s.selected_agents.insert(agent_id.clone());
                                    } else {
                                        s.selected_agents.remove(&agent_id);
                                    }
                                    cx.notify();
                                });
                            })
                    }))
            )
            .footer({
                move |_, _, _, _| {
                    let view_save2 = view_save.clone();
                    let db_save2 = db_save.clone();
                    let on_success_save2 = on_success_save.clone();
                    let state_save2 = state_save.clone();
                    let team_id_save2 = team_id_save.clone();

                    vec![
                        Button::new("cancel-manage-team")
                            .label("Cancel")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            })
                            .into_any_element(),
                        Button::new("save-manage-team")
                            .primary()
                            .label("Save Members")
                            .on_click(move |_ev, window, cx| {
                                let state = state_save2.read(cx);
                                let current_team_agents = db_save2.get_team_agents(&team_id_save2).unwrap_or_default();
                                let current_set: HashSet<String> = current_team_agents.into_iter().collect();
                                
                                // Agents to add
                                for agent_id in state.selected_agents.difference(&current_set) {
                                    let _ = db_save2.assign_agent_to_team(&team_id_save2, agent_id);
                                }
                                
                                // Agents to remove
                                for agent_id in current_set.difference(&state.selected_agents) {
                                    let _ = db_save2.remove_agent_from_team(&team_id_save2, agent_id);
                                }
                                
                                view_save2.update(cx, on_success_save2.clone());
                                window.close_dialog(cx);
                                window.push_notification(
                                    (
                                        NotificationType::Success,
                                        "Team members updated successfully.",
                                    ),
                                    cx,
                                );
                            })
                            .into_any_element(),
                    ]
                }
            })
    });
}


pub fn open_edit_agent_dialog<V: 'static>(
    db: Arc<dyn DatabasePort>,
    agent_to_edit: Agent,
    view: Entity<V>,
    window: &mut Window,
    cx: &mut Context<V>,
    on_success: impl Fn(&mut V, &mut Context<V>) + 'static + Clone,
) {
    let mut current_role = "".to_string();
    let mut current_details = "".to_string();
    if let Some(config_str) = &agent_to_edit.config {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(config_str) {
            if let Some(r) = val.get("role").and_then(|v| v.as_str()) {
                current_role = r.to_string();
            }
            if let Some(d) = val.get("details").and_then(|v| v.as_str()) {
                current_details = d.to_string();
            }
        }
    }

    let name_input = cx.new(|cx| {
        let mut state = InputState::new(window, cx).placeholder("Agent name");
        state.replace(agent_to_edit.name.clone(), window, cx);
        state
    });
    let system_prompt_input = cx.new(|cx| {
        let mut state = InputState::new(window, cx).placeholder("System prompt (optional)");
        if let Some(sp) = &agent_to_edit.system_prompt {
            state.replace(sp.clone(), window, cx);
        }
        state
    });
    let role_input = cx.new(|cx| {
        let mut state = InputState::new(window, cx).placeholder("Role / Position");
        state.replace(current_role, window, cx);
        state
    });
    let details_input = cx.new(|cx| {
        let mut state = InputState::new(window, cx).placeholder("Agent Details / Info");
        state.replace(current_details, window, cx);
        state
    });

    let providers: Vec<SharedString> = db
        .list_providers()
        .unwrap_or_default()
        .into_iter()
        .map(|p| SharedString::from(format!("{} / {}", p.provider_name, p.model)))
        .collect();
        
    let initial_provider_idx = providers.iter().position(|p| p.as_ref() == agent_to_edit.provider.as_str()).map(|i| gpui_component::IndexPath::new(i));
    let provider_select = cx.new(|cx| SelectState::new(providers, initial_provider_idx, window, cx));

    window.open_dialog(cx, move |dialog, _window, _cx| {
        let view_save = view.clone();
        let db_save = db.clone();
        let on_success_save = on_success.clone();
        let agent_to_edit_save = agent_to_edit.clone();

        let theme = _cx.theme().clone();
        dialog
            .title("Edit Agent")
            .w(px(520.))
            .child(
                v_form()
                    .gap(px(12.))
                    .py(px(8.))
                    .child(
                        field()
                            .label("Name")
                            .required(true)
                            .child(Input::new(&name_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Tên định danh của Agent (VD: Backend Developer, Data Analyst).")),
                    )
                    .child(
                        field()
                            .label("Role / Position")
                            .child(Input::new(&role_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Vai trò chuyên môn trong dự án (VD: DEV, QA, PM). Dùng cho TeamBus Router.")),
                    )
                    .child(
                        field()
                            .label("Agent Details")
                            .child(Input::new(&details_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Thông tin bổ sung, kỹ năng hoặc giới hạn của Agent.")),
                    )
                    .child(field().label("Provider / Model").child(
                        Select::new(&provider_select).placeholder("Select Provider / Model"),
                    )
                    .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Mô hình LLM sẽ cung cấp năng lực cho Agent này (VD: Claude-3-Opus, GPT-4o).")))
                    .child(
                        field()
                            .label("System Prompt")
                            .child(Input::new(&system_prompt_input))
                            .child(div().text_size(px(12.)).text_color(theme.muted_foreground).child("Chỉ dẫn hệ thống cốt lõi để định hình hành vi, phong cách trả lời và luồng suy nghĩ của Agent.")),
                    ),
            )
            .footer({
                let view_save = view_save.clone();
                let db_save = db_save.clone();
                let name_input = name_input.clone();
                let system_prompt_input = system_prompt_input.clone();
                let role_input = role_input.clone();
                let details_input = details_input.clone();
                let provider_select = provider_select.clone();
                let on_success_save = on_success_save.clone();
                let agent_to_edit_save = agent_to_edit_save.clone();

                move |_, _, _, _| {
                    let view_save2 = view_save.clone();
                    let db_save2 = db_save.clone();
                    let name_input2 = name_input.clone();
                    let system_prompt_input2 = system_prompt_input.clone();
                    let role_input2 = role_input.clone();
                    let details_input2 = details_input.clone();
                    let provider_select2 = provider_select.clone();
                    let on_success_save2 = on_success_save.clone();
                    let agent_to_edit_save2 = agent_to_edit_save.clone();

                    vec![
                        Button::new("cancel-edit-agent")
                            .label("Cancel")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            })
                            .into_any_element(),
                        Button::new("save-edit-agent")
                            .primary()
                            .label("Save Changes")
                            .on_click({
                                let view_save3 = view_save2.clone();
                                let db_save3 = db_save2.clone();
                                let name_input3 = name_input2.clone();
                                let system_prompt_input3 = system_prompt_input2.clone();
                                let role_input3 = role_input2.clone();
                                let details_input3 = details_input2.clone();
                                let provider_select3 = provider_select2.clone();
                                let on_success_save3 = on_success_save2.clone();
                                let agent_to_edit_save3 = agent_to_edit_save2.clone();

                                move |_ev, window, cx| {
                                    let name = name_input3.read(cx).text().to_string();
                                    let name = name.trim().to_string();
                                    if name.is_empty() {
                                        window.push_notification(
                                            (
                                                NotificationType::Error,
                                                "Agent name is required.",
                                            ),
                                            cx,
                                        );
                                        return;
                                    }

                                    let provider = provider_select3
                                        .read(cx)
                                        .selected_value()
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| agent_to_edit_save3.provider.clone());

                                    let role = role_input3.read(cx).text().to_string();
                                    let details = details_input3.read(cx).text().to_string();
                                    let config_json = serde_json::json!({
                                        "role": role,
                                        "details": details
                                    });

                                    let system_prompt = system_prompt_input3.read(cx).text().to_string();
                                    let now = Utc::now().to_rfc3339();
                                    let agent = Agent {
                                        id: agent_to_edit_save3.id.clone(),
                                        name,
                                        provider,
                                        system_prompt: {
                                            let t = system_prompt.trim().to_string();
                                            if t.is_empty() {
                                                None
                                            } else {
                                                Some(t)
                                            }
                                        },
                                        config: Some(config_json.to_string()),
                                        status: agent_to_edit_save3.status.clone(),
                                        created_at: agent_to_edit_save3.created_at.clone(),
                                        updated_at: now,
                                    };

                                    if db_save3.insert_agent(&agent).is_ok() {
                                        view_save3.update(cx, on_success_save3.clone());
                                        window.close_dialog(cx);
                                        window.push_notification(
                                            (
                                                NotificationType::Success,
                                                "Agent updated successfully.",
                                            ),
                                            cx,
                                        );
                                    } else {
                                        window.push_notification(
                                            (NotificationType::Error, "Failed to update agent."),
                                            cx,
                                        );
                                    }
                                }
                            })
                            .into_any_element(),
                    ]
                }
            })
    });
}
