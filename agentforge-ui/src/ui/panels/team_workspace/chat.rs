use crate::core::traits::database::DatabasePort;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::tab::{TabBar, Tab};
use gpui_component::select::{Select, SelectState};
use gpui_component::switch::Switch;
use gpui_component::WindowExt;
use gpui_component::IndexPath;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AppContext, Context, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Window
};
use gpui_component::{h_flex, ActiveTheme as _, Sizable as _, StyledExt as _, Icon, IconName};

use super::TeamWorkspacePanel;
use crate::ui::components::markdown::render_markdown_message;

impl TeamWorkspacePanel {
    pub(crate) fn render_chat_column(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let view = cx.entity().clone();

        let _active_team_id = self.selected_team_id.clone().or_else(|| {
            self.selected_instance_id.as_ref().and_then(|iid| {
                self.instances.iter().find(|i| i.id == *iid).map(|i| i.team_id.clone())
            })
        });

        let title = if let Some(instance_id) = &self.selected_instance_id {
            let inst = self.instances.iter().find(|i| i.id == *instance_id);
            inst.map(|i| i.name.clone()).unwrap_or_else(|| format!("Instance {}", &instance_id[..std::cmp::min(8, instance_id.len())]))
        } else if let Some(team_id) = &self.selected_team_id {
            self.teams.iter().find(|t| t.id == *team_id).map(|t| t.name.clone()).unwrap_or_else(|| "Team Chat".to_string())
        } else {
            "Team Chat".to_string()
        };

        let sessions_row = div().into_any_element();

        div()
            .h_full()
            .w_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child(
                // Tabs 
                div()
                    .w_full()
                    .border_b(px(1.))
                    .border_color(theme.border)
                    .bg(theme.background)
                    .child(
                        TabBar::new("chat_tabs")
                            .child(Tab::new().icon(IconName::SquareTerminal).label("Chat"))
                            .child(Tab::new().icon(IconName::ChartPie).label("Status Overview"))
                            .child(Tab::new().icon(IconName::Inbox).label("Output"))
                            .child(Tab::new().icon(IconName::Building2).label("Office"))
                            .selected_index(self.chat_active_tab)
                            .on_click({
                                let view = cx.entity().clone();
                                move |index, _window, cx| {
                                    view.update(cx, |this, cx| {
                                        this.chat_active_tab = *index;
                                        cx.notify();
                                    });
                                }
                            })
                    )
            )
            .child(
                // Top Header (Quality and Compliance Testing style)
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .border_b(px(1.))
                    .border_color(theme.border)
                    .bg(theme.background)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .p(px(16.))
                            .child(
                                h_flex()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .w(px(32.))
                                            .h(px(32.))
                                            .rounded_lg()
                                            .bg(gpui::red().opacity(0.2))
                                            .text_color(gpui::red())
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(IconName::User)
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(div().font_weight(gpui::FontWeight::BOLD).text_size(px(15.)).child(title))
                                            .child(
                                                h_flex()
                                                    .gap(px(6.))
                                                    .text_color(theme.muted_foreground)
                                                    .text_size(px(12.))
                                                    .child("Supervisor Chat")
                                                    .child(div().text_color(gpui::green()).ml(px(4.)).child("Active"))
                                            )
                                    )
                            )
                            .child(
                                h_flex()
                                    .gap(px(4.))
                                    .child(
                                        Button::new("cross-team-target-top")
                                            .ghost()
                                            .icon(IconName::Inbox)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let current = this.cross_team_target_instance_id.clone().unwrap_or_default();
                                                
                                                let mut options = Vec::new();
                                                let mut instance_ids = Vec::new();
                                                
                                                options.push(gpui::SharedString::from("None (Disable Cross-Team)"));
                                                instance_ids.push(String::new());
                                                
                                                for instance in &this.instances {
                                                    if Some(&instance.id) == this.selected_instance_id.as_ref() {
                                                        continue;
                                                    }
                                                    let team_name = this.teams.iter().find(|t| t.id == instance.team_id).map(|t| t.name.as_str()).unwrap_or("Unknown Team");
                                                    let label = format!("{} - {}", team_name, instance.id.split('-').next_back().unwrap_or(&instance.id));
                                                    options.push(gpui::SharedString::from(label));
                                                    instance_ids.push(instance.id.clone());
                                                }
                                                
                                                let selected_idx = if !current.is_empty() {
                                                    instance_ids.iter().position(|id| id == &current)
                                                } else {
                                                    Some(0)
                                                };
                                                
                                                let select_state = cx.new(|cx| {
                                                    SelectState::new(options.clone(), selected_idx.map(IndexPath::new), window, cx)
                                                });
                                                
                                                let select_state2 = select_state.clone();
                                                let instance_ids2 = instance_ids.clone();
                                                let options2 = options.clone();
                                                let view = cx.entity().clone();
                                                
                                                window.open_dialog(cx, move |dialog, _window, _cx| {
                                                    dialog
                                                        .title("Coordinate: Cross-Team Target")
                                                        .w(px(520.))
                                                        .child(
                                                            gpui_component::form::v_form()
                                                                .gap(px(12.))
                                                                .py(px(8.))
                                                                .child(
                                                                    gpui_component::form::field()
                                                                        .label("Select Target Instance")
                                                                        .child(Select::new(&select_state2).placeholder("Select instance...")),
                                                                ),
                                                        )
                                                        .footer({
                                                            let select_state3 = select_state2.clone();
                                                            let instance_ids3 = instance_ids2.clone();
                                                            let options3 = options2.clone();
                                                            let view = view.clone();
                                                            move |_, _, _, _| {
                                                                vec![
                                                                    gpui_component::button::Button::new("cancel-cross-team")
                                                                        .label("Cancel")
                                                                        .on_click(|_, window, cx| {
                                                                            window.close_dialog(cx);
                                                                        })
                                                                        .into_any_element(),
                                                                    gpui_component::button::Button::new("save-cross-team")
                                                                        .primary()
                                                                        .label("Save")
                                                                        .on_click({
                                                                            let select_state4 = select_state3.clone();
                                                                            let instance_ids4 = instance_ids3.clone();
                                                                            let options_clone = options3.clone();
                                                                            let view = view.clone();
                                                                            move |_, window, cx| {
                                                                                let selected_label = select_state4.read(cx).selected_value().map(|s| s.to_string());
                                                                                let mut value = String::new();
                                                                                if let Some(label) = selected_label {
                                                                                    if let Some(pos) = options_clone.iter().position(|o| o.as_ref() == label) {
                                                                                        if let Some(id) = instance_ids4.get(pos) {
                                                                                            value = id.clone();
                                                                                        }
                                                                                    }
                                                                                }
                                                                                view.update(cx, |this: &mut super::TeamWorkspacePanel, cx| {
                                                                                    this.cross_team_target_instance_id = if value.is_empty() { None } else { Some(value) };
                                                                                    cx.notify();
                                                                                });
                                                                                window.close_dialog(cx);
                                                                            }
                                                                        })
                                                                        .into_any_element(),
                                                                ]
                                                            }
                                                        })
                                                });
                                            }))
                                    )
                                    .child(
                                        Button::new("new-conversation-top")
                                            .ghost()
                                            .icon(IconName::Plus)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if let Some(instance_id) = &this.selected_instance_id {
                                                    let agent_id = crate::AppState::global(cx).db.get_instance_agents(instance_id)
                                                        .ok()
                                                        .and_then(|ids| ids.first().cloned());
                                                    if let Some(agent_id) = agent_id {
                                                        if crate::AppState::global(cx).db.create_session_for_instance(instance_id, &agent_id).is_ok() {
                                                            let sessions = crate::AppState::global(cx).db.list_sessions_for_instance(instance_id).unwrap_or_default();
                                                            this.sessions_for_instance = sessions.clone();
                                                            if let Some(s) = sessions.first() {
                                                                this.selected_session_id = Some(s.id.clone());
                                                                this.instance_active_session.insert(instance_id.clone(), s.id.clone());
                                                                let msgs = crate::AppState::global(cx).db.get_conversation_turns(&s.id).unwrap_or_default();
                                                                this.chat_histories.insert(s.id.clone(), msgs);
                                                                let history_len = this.chat_histories.get(&s.id).map(|h| h.len()).unwrap_or(0);
                                                                this.chat_list_state = gpui::ListState::new(history_len, gpui::ListAlignment::Bottom, px(200.));
                                                            }
                                                            cx.notify();
                                                        }
                                                    }
                                                }
                                            }))
                                    )
                                    .child(
                                        Button::new("toggle-history")
                                            .ghost()
                                            .icon(IconName::StarOff)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_history_sheet = !this.show_history_sheet;
                                                cx.notify();
                                            }))
                                    )
                                    .child(
                                        Switch::new("debate-mode")
                                            .checked(self.debate_mode)
                                            .label("Debate Mode")
                                            .tooltip("Enable agent debate before execution")
                                            .on_click({
                                                let view = view.clone();
                                                move |checked, _window, cx| {
                                                    let _ = view.update(cx, |this, cx| {
                                                        this.debate_mode = *checked;
                                                        cx.notify();
                                                    });
                                                }
                                            })
                                    )
                            )
                    )
                    .child(sessions_row)
            )
            .child(
                if self.chat_active_tab == 3 {
                    #[cfg(any(target_os = "windows", target_os = "macos"))]
                    {
                        // Native Embedded Office View via WebView (macOS / Windows)
                        if self.office_webview.is_none() {
                            let builder = wry::WebViewBuilder::new();
                            let html_content = include_str!("../../../../assets/office/index.html");
                            if let Ok(view) = builder.with_html(html_content).build_as_child(window) {
                                self.office_webview = Some(cx.new(|cx| gpui_component::webview::WebView::new(view, window, cx)));
                            }
                        }

                        if let Some(webview) = &self.office_webview {
                            // Update webview state
                            if let Some(instance_id) = &self.selected_instance_id {
                                if let Ok(agent_ids) = crate::AppState::global(cx).db.get_instance_agents(instance_id) {
                                    let mut active_agents = Vec::new();
                                    for agent in &self.agents {
                                        if agent_ids.contains(&agent.id) {
                                            active_agents.push(serde_json::json!({
                                                "id": agent.id,
                                                "name": agent.name,
                                                "provider": agent.provider,
                                                "status": agent.status,
                                                "message": None::<String>
                                            }));
                                        }
                                    }
                                    if let Ok(json) = serde_json::to_string(&active_agents) {
                                        let script = format!("window.updateAgents && window.updateAgents({});", json);
                                        let _ = webview.read(cx).evaluate_script(&script);
                                    }
                                }
                            }

                            div()
                                .flex_1()
                                .w_full()
                                .child(webview.clone())
                                .into_any_element()
                        } else {
                            div()
                                .flex_1()
                                .flex()
                                .justify_center()
                                .items_center()
                                .child("Failed to initialize WebView")
                                .into_any_element()
                        }
                    }

                    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                    {
                        // Fallback UI for Linux
                        let mut active_agents = Vec::new();
                        if let Some(instance_id) = &self.selected_instance_id {
                            if let Ok(agent_ids) = crate::AppState::global(cx).db.get_instance_agents(instance_id) {
                                for agent in &self.agents {
                                    if agent_ids.contains(&agent.id) {
                                        active_agents.push(agent.clone());
                                    }
                                }
                            }
                        }

                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .bg(theme.background)
                            .p_4()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .mb_4()
                                    .child(
                                        div()
                                            .text_size(px(18.))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.primary)
                                            .child("Embedded Virtual Office (Fallback UI)")
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.muted)
                                            .text_size(px(14.))
                                            .child(format!("{} Agents Active", active_agents.len()))
                                    )
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .w_full()
                                    .bg(theme.muted.opacity(0.1))
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(theme.border)
                                    .relative()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_wrap()
                                    .gap_6()
                                    .p_6()
                                    .justify_center()
                                    .items_center()
                                    .children(
                                        active_agents.clone().into_iter().enumerate().map(|(i, agent)| {
                                            let offset_y = if i % 2 == 0 { px(-20.) } else { px(20.) };
                                            div()
                                                .flex()
                                                .flex_col()
                                                .items_center()
                                                .mt(offset_y)
                                                .child(
                                                    div()
                                                        .w(px(64.))
                                                        .h(px(64.))
                                                        .rounded_full()
                                                        .bg(theme.primary.opacity(0.1))
                                                        .border_2()
                                                        .border_color(theme.primary)
                                                        .flex()
                                                        .justify_center()
                                                        .items_center()
                                                        .shadow_md()
                                                        .child(
                                                            gpui_component::Icon::new(gpui_component::IconName::User)
                                                                .size(px(32.))
                                                                .text_color(theme.primary)
                                                        )
                                                )
                                                .child(
                                                    div()
                                                        .mt_2()
                                                        .bg(theme.background)
                                                        .px_3()
                                                        .py_1()
                                                        .rounded_md()
                                                        .border_1()
                                                        .border_color(theme.border)
                                                        .text_size(px(12.))
                                                        .font_weight(gpui::FontWeight::MEDIUM)
                                                        .text_color(theme.foreground)
                                                        .child(agent.name.clone())
                                                )
                                                .child(
                                                    div()
                                                        .mt_1()
                                                        .text_size(px(10.))
                                                        .text_color(theme.muted)
                                                        .child(format!("Provider: {}", agent.provider))
                                                )
                                        })
                                    )
                                    .child(
                                        if active_agents.is_empty() {
                                            div()
                                                .absolute()
                                                .inset_0()
                                                .flex()
                                                .justify_center()
                                                .items_center()
                                                .text_color(theme.muted)
                                                .child("No agents currently in the office. Please select an instance or add agents.")
                                                .into_any_element()
                                        } else {
                                            div().into_any_element()
                                        }
                                    )
                            )
                            .into_any_element()
                    }
                } else {
                    // Chat View
                    let mut form_header = div()
                        .flex()
                        .gap_2()
                        .p_2()
                        .justify_between()
                        .items_center()
                        .child(
                            div().flex().gap_2().items_center().child(
                                Button::new("add-file")
                                    .icon(Icon::empty().path("icons/attachment.svg").size_4())
                                    .ghost()
                                    .mr_1()
                                    .on_click(cx.listener(|_this, _, _, cx| {
                                        let view = cx.entity().clone();
                                        cx.spawn(async move |_, cx| {
                                            if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                                                let path = file.path().to_string_lossy().to_string();
                                                let _ = cx.update(|cx| {
                                                    view.update(cx, |this: &mut super::TeamWorkspacePanel, cx| {
                                                        if !this.attached_files.contains(&path) {
                                                            this.attached_files.push(path);
                                                            cx.notify();
                                                        }
                                                    });
                                                });
                                            }
                                        }).detach();
                                    }))
                            )
                            
                        );

                    if !self.attached_files.is_empty() {
                        form_header = form_header.child(gpui_component::divider::Divider::vertical());
                        
                        let mut files_container = div().flex().gap_2().flex_wrap().w_full();
                        for (idx, path) in self.attached_files.iter().enumerate() {
                            let p = std::path::Path::new(path);
                            let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or(path).to_string();
                            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_uppercase();
                            let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                            let size_kb = size as f64 / 1024.0;
                            
                            let is_image = ["PNG", "JPG", "JPEG", "GIF", "WEBP"].contains(&ext.as_str());
                            
                            let icon_box = div()
                                .w(px(32.))
                                .h(px(32.))
                                .bg(theme.background)
                                .rounded_sm()
                                .overflow_hidden()
                                .flex()
                                .justify_center()
                                .items_center();

                            let icon_child = if is_image {
                                use gpui::StyledImage;
                                let path_buf = std::path::PathBuf::from(path);
                                icon_box.child(gpui::img(path_buf).w_full().h_full().object_fit(gpui::ObjectFit::Cover))
                            } else {
                                icon_box.child(Icon::new(IconName::File).size(px(14.)).text_color(theme.muted_foreground))
                            };

                            files_container = files_container.child(
                                div()
                                    .relative()
                                    .group(format!("file-upload-{}", idx))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .p_1()
                                    .pr_3() // Give some space on right
                                    .rounded_md()
                                    .bg(theme.secondary)
                                    .border_1()
                                    .border_color(theme.border)
                                    .child(icon_child)
                                    .child(
                                        div().flex_col()
                                            .child(div().text_xs().text_color(theme.foreground).child(file_name))
                                            .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{} • {:.1} KB", ext, size_kb)))
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(-6.))
                                            .right(px(-6.))
                                            .w(px(16.))
                                            .h(px(16.))
                                            .flex()
                                            .justify_center()
                                            .items_center()
                                            .bg(theme.border)
                                            .rounded_full()
                                            .cursor_pointer()
                                            .invisible()
                                            .group_hover(format!("file-upload-{}", idx), |s| s.visible().bg(gpui::rgba(0x000000aa)))
                                            .child(Icon::new(IconName::Close).size(px(10.)).text_color(theme.muted_foreground))
                                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                                this.attached_files.remove(idx);
                                                cx.notify();
                                            }))
                                    )
                            );
                        }
                        form_header = form_header.child(files_container);
                    }

                    let workspace_label = if let Some(path) = &self.workspace_path {
                        let p = std::path::Path::new(path);
                        p.file_name().and_then(|s| s.to_str()).unwrap_or(path).to_string()
                    } else {
                        "Select Folder".to_string()
                    };

                    let form_footer = div()
                        .flex()
                        .gap_2()
                        .p_2()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .relative()
                                        .child(
                                            div()
                                                .id("workspace-selector")
                                                .flex()
                                                .justify_start()
                                                .gap_2()
                                                .px_2()
                                                .py_1()
                                                .rounded_md()
                                                .bg(theme.secondary)
                                                .border_1()
                                                .border_color(theme.border)
                                                .items_center()
                                                .cursor_pointer()
                                                .hover(|s| s.bg(theme.border))
                                                .on_click(cx.listener(|this, _, _window, cx| {
                                                    this.is_workspace_dropdown_open = !this.is_workspace_dropdown_open;
                                                    if this.is_workspace_dropdown_open {
                                                        let db = crate::AppState::global(cx).db.clone();
                                                        if let Ok(recents) = db.get_recent_workspaces() {
                                                            this.recent_workspaces = recents;
                                                        }
                                                    }
                                                    cx.notify();
                                                }))
                                                .child(Icon::new(IconName::FolderOpen).size(px(14.)).text_color(theme.muted_foreground))
                                                .child(div().text_sm().child(workspace_label))
                                                .child(Icon::new(IconName::ChevronDown).size(px(12.)).text_color(theme.muted_foreground))
                                        )
                                        .child(
                                            if self.is_workspace_dropdown_open {
                                                // Create a list of dummy recent folders + Create Project button
                                                let mut recent_list = div().flex_col().gap_1();

                                                for r in &self.recent_workspaces {
                                                    let r_str = r.clone();
                                                    let display_name = std::path::Path::new(&r_str)
                                                        .file_name()
                                                        .and_then(|s| s.to_str())
                                                        .unwrap_or(&r_str)
                                                        .to_string();
                                                        
                                                    let is_selected = self.workspace_path.as_ref().is_some_and(|p| p == &r_str);

                                                    recent_list = recent_list.child(
                                                        div()
                                                            .id(gpui::ElementId::Name(format!("recent-{}", display_name).into()))
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .px_2()
                                                            .py_1()
                                                            .rounded_md()
                                                            .hover(|s| s.bg(theme.secondary))
                                                            .cursor_pointer()
                                                            .on_click({
                                                                let r_str_clone = r_str.clone();
                                                                cx.listener(move |this, _, _, cx| {
                                                                    if let Some(instance_id) = &this.selected_instance_id {
                                                                        let path = r_str_clone.clone();
                                                                        let key = format!("workspace_{}", instance_id);
                                                                        let _ = crate::AppState::global(cx).db.set_setting(&key, &path);
                                                                        this.workspace_path = Some(path);
                                                                        this.is_workspace_dropdown_open = false;
                                                                        cx.notify();
                                                                    }
                                                                })
                                                            })
                                                            .child(
                                                                div().flex().items_center().gap_2()
                                                                    .child(Icon::new(IconName::Folder).size(px(14.)).text_color(theme.muted_foreground))
                                                                    .child(div().text_sm().child(display_name))
                                                            )
                                                            .child(
                                                                if is_selected {
                                                                    Icon::new(IconName::Check).size(px(14.)).text_color(gpui::green())
                                                                } else {
                                                                    Icon::new(IconName::Check).size(px(14.)).text_color(gpui::transparent_black())
                                                                }
                                                            )
                                                    );
                                                }
                                                
                                                div()
                                                    .absolute()
                                                    .bottom(px(32.))
                                                    .left(px(0.))
                                                    .w(px(250.))
                                                    .bg(theme.background)
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .rounded_md()
                                                    .shadow_lg()
                                                    .p_2()
                                                    .flex_col()
                                                    .gap_2()
                                                    .child(div().text_sm().text_color(theme.muted_foreground).child("Recent"))
                                                    .child(recent_list)
                                                    .child(gpui_component::divider::Divider::horizontal().my_1())
                                                    .child(
                                                        div()
                                                            .id("create-project-btn")
                                                            .flex()
                                                            .items_center()
                                                            .gap_2()
                                                            .px_2()
                                                            .py_1()
                                                            .rounded_md()
                                                            .hover(|s| s.bg(theme.secondary))
                                                            .cursor_pointer()
                                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                                // Just open the folder picker for now
                                                                this.is_workspace_dropdown_open = false;
                                                                cx.notify();
                                                                
                                                                if let Some(instance_id) = &this.selected_instance_id {
                                                                    let db = crate::AppState::global(cx).db.clone();
                                                                    let instance_id_clone = instance_id.clone();
                                                                    let view = cx.entity().clone();
                                                                    let start_dir = this.workspace_path.clone();

                                                                    cx.spawn(async move |_, cx| {
                                                                        let mut dialog = rfd::AsyncFileDialog::new()
                                                                            .set_title("Select Workspace Folder");
                                                                        if let Some(ref dir) = start_dir {
                                                                            dialog = dialog.set_directory(dir);
                                                                        }
                                                                        if let Some(folder) = dialog.pick_folder().await {
                                                                            let path = folder.path().to_string_lossy().to_string();
                                                                            let key = format!("workspace_{}", instance_id_clone);
                                                                            let _ = db.set_setting(&key, &path);
                                                                            let _ = cx.update(|cx| {
                                                                                view.update(cx, |this: &mut super::TeamWorkspacePanel, cx| {
                                                                                    this.workspace_path = Some(path);
                                                                                    cx.notify();
                                                                                });
                                                                            });
                                                                        }
                                                                    }).detach();
                                                                }
                                                            }))
                                                            .child(Icon::new(IconName::Plus).size(px(14.)).text_color(theme.muted_foreground))
                                                            .child(div().text_sm().child("Create project"))
                                                    )
                                            } else {
                                                div()
                                            }
                                        )
                                )
                                .child(gpui_component::divider::Divider::vertical().h(px(16.)))
                                .child(
                                    div()
                                        .id("slash-cmd")
                                        .cursor_pointer()
                                        .child(div().text_sm().font_weight(gpui::FontWeight::BOLD).text_color(theme.muted_foreground).child("/"))
                                )
                                .child(
                                    div()
                                        .id("attach-btn")
                                        .cursor_pointer()
                                        .on_click(cx.listener(|_this, _, _window, cx| {
                                            let view = cx.entity().clone();
                                            cx.spawn(async move |_, cx| {
                                                if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                                                    let _ = cx.update(|cx| {
                                                        view.update(cx, |this: &mut super::TeamWorkspacePanel, cx| {
                                                            this.attached_files.push(file.path().to_string_lossy().to_string());
                                                            cx.notify();
                                                        });
                                                    });
                                                }
                                            }).detach();
                                        }))
                                        .child(Icon::empty().path("icons/attachment.svg").size_4().text_color(theme.muted_foreground))
                                )
                        )
                        .child(
                            Button::new("send-chat")
                                .rounded_full()
                                .bg(theme.accent)
                                .icon(IconName::ArrowUp)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.handle_send_chat(window, cx);
                                }))
                        );

                    let form = div()
                        .flex()
                        .flex_col()
                        .justify_between()
                        .rounded_2xl()
                        .border_1()
                        .border_color(theme.border.opacity(0.8))
                        .bg(theme.popover)
                        .h(px(220.))
                        .shadow_lg()
                        .w_full()
                        .child(
                            div().flex().flex_col().child(form_header).child(
                                gpui_component::input::Input::new(&self.chat_input_state)
                                    .appearance(false)
                            )
                        )
                        .child(form_footer);

                    h_flex()
                        .flex_1()
                        .size_full()
                        .child(
                            div()
                                .flex_1()
                                .v_flex()
                                .size_full()
                                .bg(theme.background)
                                .child(
                                    div()
                                        .p_2()
                                        .v_flex()
                                        .size_full()
                                        .child(
                                            div().p_2().pb(px(24.)).size_full().flex().child(
                                                gpui::list(
                                                    self.chat_list_state.clone(),
                                                    cx.processor(|this: &mut Self, ix, window, cx| this.render_entry(ix, window, cx)),
                                                )
                                                .size_full(),
                                            ),
                                        )
                                        .child(form)
                                )
                        )
                        .when(self.show_history_sheet, |d| {
                            let mut sessions_list = div().flex().flex_col().gap(px(4.));
                            for s in &self.sessions_for_instance {
                                let session_id = s.id.clone();
                                let is_selected = self.selected_session_id.as_deref() == Some(session_id.as_str());
                                let label = format!("S-{}", &session_id[..std::cmp::min(6, session_id.len())]);
                                sessions_list = sessions_list.child(
                                    gpui_component::button::Button::new(gpui::SharedString::from(format!("session-{}", session_id)))
                                        .small()
                                        .ghost()
                                        .when(is_selected, |b| b.primary())
                                        .label(label)
                                        .on_click(cx.listener({
                                            let session_id = session_id.clone();
                                            move |this, _, _, cx| {
                                                this.selected_session_id = Some(session_id.clone());
                                                if let Some(instance_id) = this.selected_instance_id.clone() {
                                                    this.instance_active_session
                                                        .insert(instance_id, session_id.clone());
                                                }
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
                                                this.chat_list_state = gpui::ListState::new(
                                                    history_len,
                                                    gpui::ListAlignment::Bottom,
                                                    px(200.),
                                                );
                                                cx.notify();
                                            }
                                        }))
                                );
                            }

                            d.child(
                                div()
                                    .w(px(250.))
                                    .h_full()
                                    .border_l(px(1.))
                                    .border_color(theme.border)
                                    .bg(theme.secondary.opacity(0.3))
                                    .p_2()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_size(px(14.))
                                            .mb(px(8.))
                                            .child("Session History")
                                    )
                                    .child(
                                        div()
                                            .id("session-history-scroll")
                                            .flex_1()
                                            .overflow_y_scroll()
                                            .child(sessions_list)
                                    )
                            )
                        })
                        .into_any_element()
                }
            )
    }

    pub(crate) fn handle_send_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let db = crate::AppState::global(cx).db.clone();
        let text = self.chat_input_state.read(cx).text().to_string();
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        
        let instance_id = if let Some(id) = &self.selected_instance_id {
            id.clone()
        } else {
            return; // Don't send if no instance is selected
        };

        let team_id = self.instances.iter().find(|i| i.id == *instance_id).map(|i| i.team_id.clone()).unwrap_or_default();
        let mode = crate::AppState::global(cx)
            .mode_manager
            .lock()
            .unwrap()
            .current_mode();

        let session_id = if let Some(session_id) = &self.selected_session_id {
            session_id.clone()
        } else {
            let agent_id = db
                .get_instance_agents(&instance_id)
                .ok()
                .and_then(|ids| ids.first().cloned());
            let Some(agent_id) = agent_id else { return };
            let Ok(session_id) = db.create_session_for_instance(&instance_id, &agent_id) else { return };
            self.selected_session_id = Some(session_id.clone());
            self.instance_active_session
                .insert(instance_id.clone(), session_id.clone());
            self.sessions_for_instance = db
                .list_sessions_for_instance(&instance_id)
                .unwrap_or_default();
            session_id
        };

        {
            let history = self.chat_histories.entry(session_id.clone()).or_default();
            history.push(crate::providers::ChatMessage { role: "user".into(), content: text.clone().into(), agent_name: None });
        }
        self.rebuild_chat_display(&session_id);
        let display_len = self
            .chat_display_rows
            .get(&session_id)
            .map(|v| v.len())
            .unwrap_or(0);
        self.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
        let history_snapshot = self.chat_histories.get(&session_id).cloned().unwrap_or_default();

        let user_msg = crate::teambus::routing::TeamMessage::new_broadcast(
            instance_id.clone(),
            "user".to_string(),
            text.clone(),
        );
        let _ = db.insert_team_message(&user_msg);
        let team_bus = self.team_bus.clone();
        let user_msg_clone = user_msg.clone();
        cx.spawn(async move |_, _| {
            let _ = team_bus.route_message(user_msg_clone).await;
        })
        .detach();

        if let Ok(agent_ids) = db.get_instance_agents(&instance_id) {
            if let Some(agent_id) = agent_ids.first() {
                let _ = db
                    .ensure_session(&session_id, agent_id, Some(&instance_id));
                let _ = db
                    .append_conversation_turn(&session_id, "user", &text, None);
                let _ = db.touch_session(&session_id);
            }
        }

        if let Some(target_instance_id) = self.cross_team_target_instance_id.clone() {
            if target_instance_id != instance_id {
                let forwarded = format!("[Cross-Team from {}]: {}", instance_id, text);
                let cross_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                    target_instance_id.clone(),
                    "cross-team".to_string(),
                    forwarded,
                );
                let _ = db.insert_team_message(&cross_msg);
                let team_bus = self.team_bus.clone();
                cx.spawn(async move |_, _| {
                    let _ = team_bus.route_message(cross_msg).await;
                })
                .detach();
            }
        }


        let is_run_command = text == "/run";
        let mode_clone = mode;
        let db_clone = db.clone();
        let _text_clone = text.clone();
        let team_bus_clone = self.team_bus.clone();
        let instance_id_clone = instance_id.clone();
        let team_id_clone = team_id.clone();
        let session_id_clone = session_id.clone();
        let _history_clone = history_snapshot.clone();
        let view = cx.entity().clone();
        let workspace_dir_clone = self.workspace_path.clone();

        
        if is_run_command {
            cx.spawn(async move |_, cx| {
                let _is_run_command = true;
                let _mode_clone = mode_clone;
                let _text_clone = "".to_string(); // Not used
                use crate::providers::BaseProviderAdapter;

                let all_agent_ids = db_clone.get_instance_agents(&instance_id_clone).unwrap_or_default();
                if all_agent_ids.is_empty() { return; }

                for agent_id in all_agent_ids {
                    let agent_id_clone = agent_id.clone();
                    let db_clone_agent = db_clone.clone();
                    let team_id_clone_agent = team_id_clone.clone();
                    let instance_id_clone_agent = instance_id_clone.clone();
                    let team_bus_clone_agent = team_bus_clone.clone();
                    let view_agent = view.clone();
                    let session_id_clone_agent = session_id_clone.clone();
                    let workspace_dir_agent = workspace_dir_clone.clone();

                    cx.spawn(async move |cx| {
                        loop {
                            let Ok(Some(agent)) = db_clone_agent.get_agent(&agent_id_clone) else { break; };
                            let Ok(Some(provider_config)) = db_clone_agent.get_provider_by_name(&agent.provider) else { break; };
                            
                            let tasks = db_clone_agent.list_tasks_for_instance(&instance_id_clone_agent).unwrap_or_default();
                            
                            // Find next pending task assigned to this agent where dependencies are met
                            let mut next_task = None;
                            for task in &tasks {
                                if task.status == "pending" && task.assignee_id.as_ref() == Some(&agent_id_clone) {
                                    if let Some(payload) = &task.payload {
                                        if let Ok(dag_task) = serde_json::from_str::<crate::orchestration::core::DagTask>(payload) {
                                            let mut all_deps_met = true;
                                            for dep_id in dag_task.dependencies {
                                                let full_dep_id = format!("{}:{}", instance_id_clone_agent, dep_id);
                                                if let Some(dt) = tasks.iter().find(|t| t.id == full_dep_id) {
                                                    if dt.status != "completed" {
                                                        all_deps_met = false;
                                                        break;
                                                    }
                                                }
                                            }
                                            if all_deps_met {
                                                next_task = Some(task.clone());
                                                break;
                            }
                                }
                                    }
                                }
                            }

                            let Some(task) = next_task else {
                                // If there are pending tasks but dependencies aren't met, wait and retry.
                                let has_pending = tasks.iter().any(|t| t.status == "pending" && t.assignee_id.as_ref() == Some(&agent_id_clone));
                                if has_pending {
                                    cx.background_executor().timer(std::time::Duration::from_secs(2)).await;
                                    continue;
                                } else {
                                    break;
                                }
                            };

                            // Claim task (sets to in_progress)
                            let claimed = db_clone_agent
                                .claim_task_for_instance(&task.id, &agent_id_clone, &instance_id_clone_agent)
                                .unwrap_or(false);
                            if !claimed {
                                continue;
                            }

                            let mut task_prompt = Vec::new();
                            let chat_service = crate::application::services::chat_service::ChatService::new(db_clone_agent.clone(), team_bus_clone_agent.clone());
                            if let Some(sys_prompt) = chat_service.build_dynamic_system_prompt(&team_id_clone_agent, &instance_id_clone_agent, &agent_id_clone) {
                                task_prompt.push(crate::core::models::ChatMessage { role: "system".into(), content: gpui::SharedString::from(sys_prompt), agent_name: None });
                            }
                            
                            // Instruct the LLM to output files if needed
                            let mut instructions = if let Some(ref ws) = workspace_dir_agent {
                                format!("Execute the following task. You are working in the directory: {}. If you generate or modify any files, use a markdown code block starting with ```file:<filepath> and ending with ```. Please output absolute file paths within this directory. For example:\n```file:{}/example.txt\nFile contents here\n```\nTask:\n", ws, ws)
                            } else {
                                "Execute the following task. If you generate or modify any files, use a markdown code block starting with ```file:<filepath> and ending with ```. For example:\n```file:/workspace/example.txt\nFile contents here\n```\nTask:\n".to_string()
                            };
                            
                            let task_text = task.payload.clone().unwrap_or_else(|| task.id.clone());
                            
                            // Perform RAG Vector Search
                            let embedding_provider = crate::providers::embeddings::EmbeddingProvider::new();
                            if let Ok(query_vec) = embedding_provider.get_embedding(&task_text).await {
                                if let Ok(similar) = db_clone_agent.search_similar_chunks(&query_vec, 3) {
                                    if !similar.is_empty() {
                                        instructions.push_str("\n\n[SYSTEM KNOWLEDGE RETRIEVAL]\nHere is context retrieved from the user's Obsidian Vault that might be relevant to your task:\n");
                                        for (title, chunk_content, sim) in similar {
                                            if sim > 0.6 { // Only include somewhat relevant chunks
                                                instructions.push_str(&format!("\n--- Document: {} (Similarity: {:.2}) ---\n{}\n", title, sim, chunk_content));
                                            }
                                        }
                                        instructions.push_str("\n[END KNOWLEDGE RETRIEVAL]\n\n");
                                    }
                                }
                            }
                            
                            task_prompt.push(crate::providers::ChatMessage { role: "user".into(), content: format!("{}{}", instructions, task_text).into(), agent_name: None });

                            let result = if provider_config.provider_name == "openrouter" {
                                let mut adapter = crate::providers::openrouter::OpenRouterAdapter::new();
                                if adapter.initialize(&provider_config).is_ok() {
                                    adapter.send_message(task_prompt).await
                                } else {
                                    Err(anyhow::anyhow!("Failed to init adapter"))
                                }
                            } else {
                                let mut adapter = crate::providers::claude::ClaudeAdapter::new();
                                if adapter.initialize(&provider_config).is_ok() {
                                    adapter.send_message(task_prompt).await
                                } else {
                                    Err(anyhow::anyhow!("Failed to init adapter"))
                                }
                            };

                            let (status_text, ok) = match result {
                                Ok(resp) => {
                                    let text = resp.content.to_string();
                                    // Insert token usage
                                    let _ = db_clone_agent.insert_token_usage(
                                        Some(&instance_id_clone_agent),
                                        &agent_id_clone,
                                        resp.token_usage.input_tokens,
                                        resp.token_usage.output_tokens,
                                        resp.token_usage.total_tokens
                                    );
                                    
                                    let chat_service = crate::application::services::chat_service::ChatService::new(db_clone_agent.clone(), team_bus_clone_agent.clone());
                                    let (files_written, _) = chat_service.parse_and_write_files(&text, workspace_dir_agent.as_ref());
                                    
                                    let mut final_text = format!("[Task Completed] {}:\n{}", task.id, text);
                                    if !files_written.is_empty() {
                                        final_text.push_str("\n\n**Files Generated/Modified:**\n");
                                        for f in files_written {
                                            // Strip absolute path
                                            let display_path = if let Some(ws) = workspace_dir_agent.as_ref() {
                                                f.replace(ws, "")
                                            } else {
                                                std::path::Path::new(&f).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or(f)
                                            };
                                            let display_path = display_path.trim_start_matches('/').trim_start_matches('\\');
                                            final_text.push_str(&format!("- `{}`\n", display_path));
                                        }
                                    }
                                    
                                    (final_text, true)
                                },
                                Err(e) => (format!("[Task Failed] {}:
{}", task.id, e), false),
                            };

                            let _ = if ok {
                                db_clone_agent.mark_task_completed(&task.id)
                            } else {
                                db_clone_agent.mark_task_failed(&task.id)
                            };

                            let agent_name_str = agent.name.clone();
                            let metadata = serde_json::json!({"agent_name": agent_name_str}).to_string();
                            
                            let mut msg = crate::teambus::routing::TeamMessage::new_broadcast(
                                instance_id_clone_agent.clone(),
                                "assistant".to_string(),
                                status_text.clone(),
                            );
                            msg.metadata = Some(metadata.clone());
                            let _ = db_clone_agent.insert_team_message(&msg);
                            let _ = team_bus_clone_agent.route_message(msg).await;
                            
                            // Save to database so it persists across reloads!
                            let _ = db_clone_agent.ensure_session(&session_id_clone_agent, &agent_id_clone, Some(&instance_id_clone_agent));
                            let _ = db_clone_agent.append_conversation_turn(
                                &session_id_clone_agent,
                                "assistant",
                                &status_text,
                                Some(&metadata),
                            );
                            let _ = db_clone_agent.touch_session(&session_id_clone_agent);

                            let _ = cx.update(|cx| view_agent.update(cx, |this: &mut Self, cx| {
                                let session_id = this
                                    .instance_active_session
                                    .get(&instance_id_clone_agent)
                                    .cloned()
                                    .or_else(|| this.selected_session_id.clone());
                                if let Some(session_id) = session_id {
                                    {
                                        let history = this.chat_histories.entry(session_id.clone()).or_default();
                                        history.push(crate::providers::ChatMessage {
                                            role: "assistant".into(),
                                            content: status_text.clone().into(),
                                            agent_name: Some(agent_name_str.into())
                                        });
                                    }
                                    this.rebuild_chat_display(&session_id);
                                    if this.selected_session_id.as_deref() == Some(session_id.as_str()) {
                                        let display_len = this
                                            .chat_display_rows
                                            .get(&session_id)
                                            .map(|v| v.len())
                                            .unwrap_or(0);
                                        this.chat_list_state = gpui::ListState::new(
                                            display_len,
                                            gpui::ListAlignment::Bottom,
                                            gpui::px(200.),
                                        );
                                    }
                                }
                                cx.notify();
                            }));
                        }
                    }).detach();
                }
            }).detach();
            self.chat_input_state.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
            cx.notify();
            return;
        }
        
        self.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
        self.chat_input_state.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        cx.notify();


        // Trigger AI response asynchronously
        let history_clone = history_snapshot.clone();
        let view = cx.entity().clone();
        let db = db.clone();
        let team_id_clone = team_id.clone();
        let query_text = text.clone();
        let instance_id_for_ai = instance_id.clone();
        let session_id_for_ai = session_id.clone();

        let debate_mode = self.debate_mode;
let db_clone = db.clone();
        let team_bus_for_ai = self.team_bus.clone();
        let team_bus_clone = self.team_bus.clone();
        cx.spawn(async move |_, cx| {
            use crate::providers::BaseProviderAdapter;

            let mut use_mock = false;

            if let Ok(agent_ids) = db.get_instance_agents(&instance_id_for_ai) {
                let agent_ids: Vec<String> = agent_ids;
                let mut current_history = history_clone.clone();

                let debate_steps: Vec<(String, &'static str)> = if debate_mode && agent_ids.len() > 1 {
                    let mut steps = vec![
                        (agent_ids[0].clone(), "PROPOSER"),
                        (agent_ids[1].clone(), "CRITIC"),
                        (agent_ids[0].clone(), "RESOLVER"),
                    ];
                    if agent_ids.len() > 2 {
                        steps.push((agent_ids[2].clone(), "JUDGE"));
                    }
                    steps
                } else {
                    agent_ids
                        .first()
                        .map(|id| vec![(id.clone(), "SOLO")])
                        .unwrap_or_default()
                };

                for (step_ix, (agent_id, debate_role)) in debate_steps.iter().enumerate() {
                    if !agent_id.is_empty() {
                        let agent_id = agent_id.clone();
                        if let Ok(Some(agent)) = db.get_agent(&agent_id) {
                            if let Ok(Some(provider_config)) = db.get_provider_by_name(&agent.provider) {
                                use_mock = false;
                            
                            let chat_service = crate::application::services::chat_service::ChatService::new(db.clone(), team_bus_for_ai.clone());
                            let mut full_history = current_history.clone();

                            if let Some(mut sys) = chat_service.build_dynamic_system_prompt(&team_id_clone, &instance_id_for_ai, &agent_id) {
                                if debate_mode && *debate_role != "SOLO" && step_ix > 0 {
                                    sys.push_str("\n\nDEBATE PROTOCOL\nYou are collaborating with other agents in the same team. You MUST read previous agent responses in the chat history.\n");
                                    match *debate_role {
                                        "PROPOSER" => {
                                            sys.push_str("ROLE: PROPOSER\nProvide an initial solution/plan. Be concrete and actionable.\n");
                                        }
                                        "CRITIC" => {
                                            sys.push_str("ROLE: CRITIC\nCritique the proposer response. Identify flaws, missing steps, security risks, and mismatches with requirements. Provide a numbered issue list.\n");
                                        }
                                        "RESOLVER" => {
                                            sys.push_str("ROLE: RESOLVER\nRevise the proposal to address ALL critique issues. Output the revised plan. If it is final and agreed, include [CONSENSUS_REACHED].\n");
                                        }
                                        "JUDGE" => {
                                            sys.push_str("ROLE: JUDGE\nDecide if the latest plan is ready to execute. If acceptable, include [CONSENSUS_REACHED]. If not, list blocking issues.\n");
                                        }
                                        _ => {}
                                    }
                                }
                                full_history.insert(0, crate::providers::ChatMessage { role: gpui::SharedString::from("system"), content: gpui::SharedString::from(sys), agent_name: None });
                            }

                            let mut round_result: Option<String> = None;
                            match provider_config.provider_name.as_str() {
                                "openrouter" => {
                                    let mut adapter = crate::providers::openrouter::OpenRouterAdapter::new();
                                    if adapter.initialize(&provider_config).is_ok() {
                                    let agent_name_str = agent.name.clone();
                                    let metadata = serde_json::json!({"agent_name": agent_name_str}).to_string();
                                    let mut office_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                                        instance_id_for_ai.clone(),
                                        "assistant".to_string(),
                                        format!("[{}]: ", agent.name),
                                    );
                                    office_msg.metadata = Some(metadata.clone());
                                    office_msg.delivery_status = "typing".to_string();
                                    let office_msg_id = office_msg.id.clone();
                                    let _ = db_clone.insert_team_message(&office_msg);

                                    // PUSH "AI thinking..." placeholder
                                    let mut msg_idx = 0;
                                    let _ = cx.update(|cx| {
                                        view.update(cx, |this: &mut Self, cx| {
                                            {
                                                let history = this.chat_histories.entry(session_id_for_ai.clone()).or_default();
                                                history.push(crate::providers::ChatMessage {
                                                    role: "assistant".into(),
                                                    content: "AI thinking...".into(),
                                                    agent_name: Some(agent.name.clone().into())
                                                });
                                                msg_idx = history.len() - 1;
                                            }
                                            this.rebuild_chat_display(&session_id_for_ai);
                                            let display_len = this
                                                .chat_display_rows
                                                .get(&session_id_for_ai)
                                                .map(|v| v.len())
                                                .unwrap_or(0);
                                            this.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
                                            cx.notify();
                                        });
                                    });

                                        
                                        let mcp_registry = std::sync::Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(db.clone()));
                                        let executor = crate::application::orchestration::executor::AgentExecutor::new(
                                            std::sync::Arc::new(adapter) as std::sync::Arc<dyn crate::providers::BaseProviderAdapter>,
                                            mcp_registry,
                                            db.clone(),
                                            team_bus_clone.clone(),
                                            instance_id.clone(),
                                            agent.id.clone()
                                        );
                                        
                                        match executor.execute_task(full_history).await {
                                            Ok(full_text) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &full_text);
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "delivered");
                                                
                                                let chat_service = crate::application::services::chat_service::ChatService::new(db_clone.clone(), team_bus_clone.clone());
                                                let (files_written, clean_text) = chat_service.parse_and_write_files(&full_text, None);
                                                if !files_written.is_empty() {
                                                    let _ = db_clone.update_team_message_content(&office_msg_id, &clean_text);
                                                }
                                                let chosen = if files_written.is_empty() { full_text } else { clean_text };
                                                round_result = Some(chosen);
                                                let _ = db_clone.ensure_session(&session_id_for_ai, &agent.id, Some(&instance_id_for_ai));
                                                let _ = db_clone.append_conversation_turn(&session_id_for_ai, "assistant", round_result.as_ref().unwrap(), Some(&metadata));
                                                let _ = db_clone.touch_session(&session_id_for_ai);
                                                
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                            Err(e) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &format!("Error: {}", e));
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "failed");
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                        }

                                    }
                                }
                                "claude" => {
                                    let mut adapter = crate::providers::claude::ClaudeAdapter::new();
                                    if adapter.initialize(&provider_config).is_ok() {
                                    let agent_name_str = agent.name.clone();
                                    let metadata = serde_json::json!({"agent_name": agent_name_str}).to_string();
                                    let mut office_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                                        instance_id_for_ai.clone(),
                                        "assistant".to_string(),
                                        format!("[{}]: ", agent.name),
                                    );
                                    office_msg.metadata = Some(metadata.clone());
                                    office_msg.delivery_status = "typing".to_string();
                                    let office_msg_id = office_msg.id.clone();
                                    let _ = db_clone.insert_team_message(&office_msg);

                                    // PUSH "AI thinking..." placeholder
                                    let mut msg_idx = 0;
                                    let _ = cx.update(|cx| {
                                        view.update(cx, |this: &mut Self, cx| {
                                            {
                                                let history = this.chat_histories.entry(session_id_for_ai.clone()).or_default();
                                                history.push(crate::providers::ChatMessage {
                                                    role: "assistant".into(),
                                                    content: "AI thinking...".into(),
                                                    agent_name: Some(agent.name.clone().into())
                                                });
                                                msg_idx = history.len() - 1;
                                            }
                                            this.rebuild_chat_display(&session_id_for_ai);
                                            let display_len = this
                                                .chat_display_rows
                                                .get(&session_id_for_ai)
                                                .map(|v| v.len())
                                                .unwrap_or(0);
                                            this.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
                                            cx.notify();
                                        });
                                    });

                                        
                                        let mcp_registry = std::sync::Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(db.clone()));
                                        let executor = crate::application::orchestration::executor::AgentExecutor::new(
                                            std::sync::Arc::new(adapter) as std::sync::Arc<dyn crate::providers::BaseProviderAdapter>,
                                            mcp_registry,
                                            db.clone(),
                                            team_bus_clone.clone(),
                                            instance_id.clone(),
                                            agent.id.clone()
                                        );
                                        
                                        match executor.execute_task(full_history).await {
                                            Ok(full_text) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &full_text);
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "delivered");
                                                
                                                let chat_service = crate::application::services::chat_service::ChatService::new(db_clone.clone(), team_bus_clone.clone());
                                                let (files_written, clean_text) = chat_service.parse_and_write_files(&full_text, None);
                                                if !files_written.is_empty() {
                                                    let _ = db_clone.update_team_message_content(&office_msg_id, &clean_text);
                                                }
                                                let chosen = if files_written.is_empty() { full_text } else { clean_text };
                                                round_result = Some(chosen);
                                                let _ = db_clone.ensure_session(&session_id_for_ai, &agent.id, Some(&instance_id_for_ai));
                                                let _ = db_clone.append_conversation_turn(&session_id_for_ai, "assistant", round_result.as_ref().unwrap(), Some(&metadata));
                                                let _ = db_clone.touch_session(&session_id_for_ai);
                                                
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                            Err(e) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &format!("Error: {}", e));
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "failed");
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                        }

                                    }
                                }
                                "gemini" => {
                                    let mut adapter = crate::providers::gemini::GeminiAdapter::new();
                                    if adapter.initialize(&provider_config).is_ok() {
                                        let agent_name_str = agent.name.clone();
                                        let metadata = serde_json::json!({"agent_name": agent_name_str}).to_string();
                                        let mut office_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                                            instance_id_for_ai.clone(),
                                            "assistant".to_string(),
                                            format!("[{}]: ", agent.name),
                                        );
                                        office_msg.metadata = Some(metadata.clone());
                                        office_msg.delivery_status = "typing".to_string();
                                        let office_msg_id = office_msg.id.clone();
                                        let _ = db_clone.insert_team_message(&office_msg);

                                        let mut msg_idx = 0;
                                        let _ = cx.update(|cx| {
                                            view.update(cx, |this: &mut Self, cx| {
                                                {
                                                    let history = this.chat_histories.entry(session_id_for_ai.clone()).or_default();
                                                    history.push(crate::providers::ChatMessage {
                                                        role: "assistant".into(),
                                                        content: "AI thinking...".into(),
                                                        agent_name: Some(agent.name.clone().into())
                                                    });
                                                    msg_idx = history.len() - 1;
                                                }
                                                this.rebuild_chat_display(&session_id_for_ai);
                                                let display_len = this
                                                    .chat_display_rows
                                                    .get(&session_id_for_ai)
                                                    .map(|v| v.len())
                                                    .unwrap_or(0);
                                                this.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
                                                cx.notify();
                                            });
                                        });

                                        
                                        let mcp_registry = std::sync::Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(db.clone()));
                                        let executor = crate::application::orchestration::executor::AgentExecutor::new(
                                            std::sync::Arc::new(adapter) as std::sync::Arc<dyn crate::providers::BaseProviderAdapter>,
                                            mcp_registry,
                                            db.clone(),
                                            team_bus_clone.clone(),
                                            instance_id.clone(),
                                            agent.id.clone()
                                        );
                                        
                                        match executor.execute_task(full_history).await {
                                            Ok(full_text) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &full_text);
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "delivered");
                                                
                                                let chat_service = crate::application::services::chat_service::ChatService::new(db_clone.clone(), team_bus_clone.clone());
                                                let (files_written, clean_text) = chat_service.parse_and_write_files(&full_text, None);
                                                if !files_written.is_empty() {
                                                    let _ = db_clone.update_team_message_content(&office_msg_id, &clean_text);
                                                }
                                                let chosen = if files_written.is_empty() { full_text } else { clean_text };
                                                round_result = Some(chosen);
                                                let _ = db_clone.ensure_session(&session_id_for_ai, &agent.id, Some(&instance_id_for_ai));
                                                let _ = db_clone.append_conversation_turn(&session_id_for_ai, "assistant", round_result.as_ref().unwrap(), Some(&metadata));
                                                let _ = db_clone.touch_session(&session_id_for_ai);
                                                
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                            Err(e) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &format!("Error: {}", e));
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "failed");
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                        }

                                    }
                                }
                                "codex" => {
                                    let mut adapter = crate::providers::codex::CodexAdapter::new();
                                    if adapter.initialize(&provider_config).is_ok() {
                                        let agent_name_str = agent.name.clone();
                                        let metadata = serde_json::json!({"agent_name": agent_name_str}).to_string();
                                        let mut office_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                                            instance_id_for_ai.clone(),
                                            "assistant".to_string(),
                                            format!("[{}]: ", agent.name),
                                        );
                                        office_msg.metadata = Some(metadata.clone());
                                        office_msg.delivery_status = "typing".to_string();
                                        let office_msg_id = office_msg.id.clone();
                                        let _ = db_clone.insert_team_message(&office_msg);

                                        let mut msg_idx = 0;
                                        let _ = cx.update(|cx| {
                                            view.update(cx, |this: &mut Self, cx| {
                                                {
                                                    let history = this.chat_histories.entry(session_id_for_ai.clone()).or_default();
                                                    history.push(crate::providers::ChatMessage {
                                                        role: "assistant".into(),
                                                        content: "AI thinking...".into(),
                                                        agent_name: Some(agent.name.clone().into())
                                                    });
                                                    msg_idx = history.len() - 1;
                                                }
                                                this.rebuild_chat_display(&session_id_for_ai);
                                                let display_len = this
                                                    .chat_display_rows
                                                    .get(&session_id_for_ai)
                                                    .map(|v| v.len())
                                                    .unwrap_or(0);
                                                this.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
                                                cx.notify();
                                            });
                                        });

                                        
                                        let mcp_registry = std::sync::Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(db.clone()));
                                        let executor = crate::application::orchestration::executor::AgentExecutor::new(
                                            std::sync::Arc::new(adapter) as std::sync::Arc<dyn crate::providers::BaseProviderAdapter>,
                                            mcp_registry,
                                            db.clone(),
                                            team_bus_clone.clone(),
                                            instance_id.clone(),
                                            agent.id.clone()
                                        );
                                        
                                        match executor.execute_task(full_history).await {
                                            Ok(full_text) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &full_text);
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "delivered");
                                                
                                                let chat_service = crate::application::services::chat_service::ChatService::new(db_clone.clone(), team_bus_clone.clone());
                                                let (files_written, clean_text) = chat_service.parse_and_write_files(&full_text, None);
                                                if !files_written.is_empty() {
                                                    let _ = db_clone.update_team_message_content(&office_msg_id, &clean_text);
                                                }
                                                let chosen = if files_written.is_empty() { full_text } else { clean_text };
                                                round_result = Some(chosen);
                                                let _ = db_clone.ensure_session(&session_id_for_ai, &agent.id, Some(&instance_id_for_ai));
                                                let _ = db_clone.append_conversation_turn(&session_id_for_ai, "assistant", round_result.as_ref().unwrap(), Some(&metadata));
                                                let _ = db_clone.touch_session(&session_id_for_ai);
                                                
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                            Err(e) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &format!("Error: {}", e));
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "failed");
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                        }

                                    }
                                }
                                "opencode" => {
                                    let mut adapter = crate::providers::opencode::OpenCodeAdapter::new();
                                    if adapter.initialize(&provider_config).is_ok() {
                                        let agent_name_str = agent.name.clone();
                                        let metadata = serde_json::json!({"agent_name": agent_name_str}).to_string();
                                        let mut office_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                                            instance_id_for_ai.clone(),
                                            "assistant".to_string(),
                                            format!("[{}]: ", agent.name),
                                        );
                                        office_msg.metadata = Some(metadata.clone());
                                        office_msg.delivery_status = "typing".to_string();
                                        let office_msg_id = office_msg.id.clone();
                                        let _ = db_clone.insert_team_message(&office_msg);

                                        let mut msg_idx = 0;
                                        let _ = cx.update(|cx| {
                                            view.update(cx, |this: &mut Self, cx| {
                                                {
                                                    let history = this.chat_histories.entry(session_id_for_ai.clone()).or_default();
                                                    history.push(crate::providers::ChatMessage {
                                                        role: "assistant".into(),
                                                        content: "AI thinking...".into(),
                                                        agent_name: Some(agent.name.clone().into())
                                                    });
                                                    msg_idx = history.len() - 1;
                                                }
                                                this.rebuild_chat_display(&session_id_for_ai);
                                                let display_len = this
                                                    .chat_display_rows
                                                    .get(&session_id_for_ai)
                                                    .map(|v| v.len())
                                                    .unwrap_or(0);
                                                this.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
                                                cx.notify();
                                            });
                                        });

                                        
                                        let mcp_registry = std::sync::Arc::new(crate::infrastructure::mcp::registry::McpToolRegistry::new(db.clone()));
                                        let executor = crate::application::orchestration::executor::AgentExecutor::new(
                                            std::sync::Arc::new(adapter) as std::sync::Arc<dyn crate::providers::BaseProviderAdapter>,
                                            mcp_registry,
                                            db.clone(),
                                            team_bus_clone.clone(),
                                            instance_id.clone(),
                                            agent.id.clone()
                                        );
                                        
                                        match executor.execute_task(full_history).await {
                                            Ok(full_text) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &full_text);
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "delivered");
                                                
                                                let chat_service = crate::application::services::chat_service::ChatService::new(db_clone.clone(), team_bus_clone.clone());
                                                let (files_written, clean_text) = chat_service.parse_and_write_files(&full_text, None);
                                                if !files_written.is_empty() {
                                                    let _ = db_clone.update_team_message_content(&office_msg_id, &clean_text);
                                                }
                                                let chosen = if files_written.is_empty() { full_text } else { clean_text };
                                                round_result = Some(chosen);
                                                let _ = db_clone.ensure_session(&session_id_for_ai, &agent.id, Some(&instance_id_for_ai));
                                                let _ = db_clone.append_conversation_turn(&session_id_for_ai, "assistant", round_result.as_ref().unwrap(), Some(&metadata));
                                                let _ = db_clone.touch_session(&session_id_for_ai);
                                                
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                            Err(e) => {
                                                let _ = db_clone.update_team_message_content(&office_msg_id, &format!("Error: {}", e));
                                                let _ = db_clone.update_team_message_delivery_status(&office_msg_id, "failed");
                                                let _ = cx.update(|cx| view.update(cx, |_, cx| cx.notify())).ok();
                                            }
                                        }

                                    }
                                }
                                _ => {}
                            }
                            
                            if let Some(text) = round_result {
                                current_history.push(crate::providers::ChatMessage {
                                    role: gpui::SharedString::from("assistant"),
                                    content: gpui::SharedString::from(text.clone()),
                                    agent_name: Some(gpui::SharedString::from(agent.name.clone())),
                                });
                                if debate_mode && text.contains("[CONSENSUS_REACHED]") {
                                    break;
                                }
                            }
                            }
                        }
                    }
                }
            }

            if use_mock {
                let _ = cx.update(|cx| {
                    view.update(cx, |this: &mut Self, cx| {
                        let error_text = "No valid provider found or configured for this team. Please check agent provider settings.";
                        {
                            let history = this
                                .chat_histories
                                .entry(session_id_for_ai.clone())
                                .or_default();
                            history.push(crate::providers::ChatMessage { role: "assistant".into(), content: error_text.into(), agent_name: None });
                        }
                        this.rebuild_chat_display(&session_id_for_ai);
                        let assistant_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                            instance_id_for_ai.clone(),
                            "assistant".to_string(),
                            error_text.to_string(),
                        );
                        let _ = crate::AppState::global(cx).db.insert_team_message(&assistant_msg);
                        let team_bus = this.team_bus.clone();
                        let assistant_msg_clone = assistant_msg.clone();
                        cx.spawn(async move |_, _| {
                            let _ = team_bus.route_message(assistant_msg_clone).await;
                        })
                        .detach();
                        let display_len = this
                            .chat_display_rows
                            .get(&session_id_for_ai)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        this.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
                        cx.notify();
                    });
                });
            }
        }).detach();
    }
    pub(crate) fn render_entry(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let session_id = if let Some(id) = &self.selected_session_id {
            id.clone()
        } else {
            return div().into_any_element();
        };

        let _history = self.chat_histories.entry(session_id.clone()).or_default();
        if !self.chat_display_rows.contains_key(&session_id) {
            self.rebuild_chat_display(&session_id);
        }
        let Some(rows) = self.chat_display_rows.get(&session_id) else {
            return div().into_any_element();
        };
        if rows.is_empty() || ix >= rows.len() {
            return div().into_any_element();
        }
        let row = rows[ix].clone();
        let theme = cx.theme().clone();

        let (source_index, msg) = match row {
            super::ChatDisplayRow::CrossTeamThreadHeader { correlation_id, handoff_type, from_team, count } => {
                let is_expanded = self.expanded_threads.contains(&correlation_id);
                let icon = if is_expanded { IconName::ChevronDown } else { IconName::ChevronRight };
                let session_id_clone = session_id.clone();
                let correlation_id_clone = correlation_id.clone();
                let from_team_short = if from_team.len() > 12 { format!("{}…", &from_team[..12]) } else { from_team };
                let correlation_short = if correlation_id.len() > 8 { &correlation_id[..8] } else { &correlation_id };
                return div()
                    .id(("cross-team-thread", ix))
                    .w_full()
                    .px(px(12.))
                    .py(px(8.))
                    .rounded_md()
                    .bg(theme.muted.opacity(0.08))
                    .border(px(1.))
                    .border_color(theme.border)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.expanded_threads.contains(&correlation_id_clone) {
                            this.expanded_threads.remove(&correlation_id_clone);
                        } else {
                            this.expanded_threads.insert(correlation_id_clone.clone());
                        }
                        this.rebuild_chat_display(&session_id_clone);
                        let display_len = this
                            .chat_display_rows
                            .get(&session_id_clone)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        this.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
                        cx.notify();
                    }))
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(Icon::new(icon).size(px(14.)).text_color(theme.muted_foreground))
                                    .child(div().font_weight(gpui::FontWeight::BOLD).text_size(px(13.)).child(format!("Cross-team {} ({})", handoff_type, count)))
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{} • {}", correlation_short, from_team_short))
                            )
                    )
                    .into_any_element();
            }
            super::ChatDisplayRow::Message { source_index, msg } => (source_index, msg),
        };

        let is_user = msg.role == "user";
        
        let msg_key = format!("{}_{}", session_id, source_index);
        let is_expanded = self.expanded_messages.contains(&msg_key);
        
        // Count lines for user message to see if we need collapse
        let lines: Vec<&str> = msg.content.lines().collect();
        let needs_collapse = is_user && lines.len() > 5;
        
        let content_to_render = if needs_collapse && !is_expanded {
            // Show only first line + indicator
            let first_line = lines.first().unwrap_or(&"");
            format!("{} ...", first_line)
        } else {
            msg.content.to_string()
        };

        let mut display_name = if let Some(name) = &msg.agent_name {
            name.to_string()
        } else {
            "Agent".to_string()
        };
        let mut display_content = content_to_render.clone();
        
        if !is_user {
            // Strip legacy prefixes like [Task Completed] {task_id}:

            if display_content.starts_with("[Task Completed]") {
                if let Some(idx) = display_content.find(":\n") {
                    display_content = display_content[idx + 2..].to_string();
                }
            }
            // Strip legacy interactive [AgentName]: 
            else if let Some(end_bracket) = display_content.find("]: ") {
                if display_content.starts_with('[') {
                    if msg.agent_name.is_none() {
                        display_name = display_content[1..end_bracket].to_string();
                    }
                    display_content = display_content[end_bracket + 3..].to_string();
                }
            }

            if display_content.starts_with("[CROSS_TEAM_HANDOFF]") {
                let payload_str = display_content
                    .trim_start_matches("[CROSS_TEAM_HANDOFF]")
                    .trim();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload_str) {
                    let handoff_type = v.get("handoff_type").and_then(|x| x.as_str()).unwrap_or("handoff");
                    let correlation_id = v.get("correlation_id").and_then(|x| x.as_str()).unwrap_or("");
                    let from_team = v.get("from_team").and_then(|x| x.as_str()).unwrap_or("");
                    let package = v.get("briefing_package").and_then(|x| x.as_str()).unwrap_or("");
                    display_name = format!("Cross-team {}", handoff_type);
                    display_content = format!(
                        "correlation_id: {}\nfrom_team: {}\n\n{}",
                        correlation_id, from_team, package
                    );
                }
            }
        }

        let agent_avatar = div()
            .w(px(28.))
            .h(px(28.))
            .rounded_full()
            .bg(gpui::blue().opacity(0.2))
            .text_color(gpui::blue())
            .flex()
            .items_center()
            .justify_center()
            .child(IconName::Bot);

        let text_element = Self::render_message_text(&display_content, &theme, window);

        let elem = if is_user {
            h_flex()
                .id(("msg-row", ix))
                .group("msg_row")
                .w_full()
                .justify_end()
                .items_start()
                .gap_2()
                .child(
                    h_flex()
                        .invisible()
                        .group_hover("msg_row", |s| s.visible())
                        .gap_1()
                        .child(
                            div()
                                .id(("delete-msg", ix))
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|s| s.text_color(gpui::red()))
                                .child(Icon::empty().path("icons/trash.svg").size_8())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(session_id) = this.selected_session_id.clone() {
                                        if let Some(history) = this.chat_histories.get_mut(&session_id) {
                                            if source_index < history.len() {
                                                history.remove(source_index);
                                                this.rebuild_chat_display(&session_id);
                                                let display_len = this
                                                    .chat_display_rows
                                                    .get(&session_id)
                                                    .map(|v| v.len())
                                                    .unwrap_or(0);
                                                this.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
                                                cx.notify();
                                            }
                                        }
                                    }
                                }))
                        )
                        // Clipboard
                        .child(
                            gpui_component::clipboard::Clipboard::new(("clipboard", ix))
                                .value(msg.content.clone().to_string())
                        )
                )
                .child(
                    div()
                    .max_w(px(640.0))
            // .ml_auto()
                    .p_2()
                    .bg(theme.list_even)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .flex()
                    .child(text_element)
                        .when(needs_collapse, |d: gpui::Div| {
                            d.child(
                                div()
                                    .id(("collapse-btn", ix))
                                    .cursor_pointer()
                                    .flex_none()
                                    .text_color(theme.muted_foreground)
                                    .child(if is_expanded { IconName::ChevronUp } else { IconName::ChevronDown })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.expanded_messages.contains(&msg_key) {
                                            this.expanded_messages.remove(&msg_key);
                                        } else {
                                            this.expanded_messages.insert(msg_key.clone());
                                        }
                                        cx.notify();
                                    }))
                            )
                        })
                ).into_any_element()
        } else {
            div()
                .w_full()
                .flex()
                .justify_start()
                .child(
                    h_flex()
                            .w_full()
                            .gap_2()
                            .items_start()
                            .child(agent_avatar)
                            .child(
                                div()
                                    .max_w(gpui::relative(0.85))
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .mb(px(2.))
                                            .child(display_name)
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .rounded_lg()
                                            .overflow_hidden()
                                            .child(text_element)
                                    )
                            )
                ).into_any_element()
        };

        div().p_2().child(elem).into_any_element()
    }

    fn render_message_text(content: &str, theme: &gpui_component::Theme, cx: &mut Window) -> impl IntoElement {
        render_markdown_message(content, theme, cx)
    }

}
