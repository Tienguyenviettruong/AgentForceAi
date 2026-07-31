use gpui::{
    div, px, AppContext, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    form::{field, v_form},
    h_flex,
    input::{Input, InputState},
    notification::NotificationType,
    scroll::ScrollableElement,
    v_flex, ActiveTheme as _, Icon, IconName, Sizable, WindowExt,
};

use super::{
    model::{SoloConversation, SoloProject, SoloSection},
    SoloWorkspacePanel,
};

impl SoloWorkspacePanel {
    fn active_project_name(&self) -> &str {
        let Some(active_project_id) = self.active_project_id.as_deref() else {
            return "Conversations";
        };

        self.projects
            .iter()
            .find(|project| project.id == active_project_id)
            .map(|project| project.name.as_str())
            .unwrap_or("Project")
    }

    fn close_history_sidebar(&mut self, cx: &mut Context<Self>) {
        self.history_sidebar_open = false;
        cx.notify();
    }

    fn open_project_history(&mut self, project_id: String, cx: &mut Context<Self>) {
        self.upsert_active_conversation(cx);
        if self.active_project_id.as_deref() != Some(project_id.as_str()) {
            self.messages.clear();
            self.active_conversation_id = None;
            self.solo_attachments.clear();
            self.solo_expanded_messages.clear();
            self.reset_solo_chat_list_state();
        }
        self.solo_webview_open = false;
        self.solo_webview = None;
        self.solo_webview_generation = self.solo_webview_generation.wrapping_add(1);
        self.active_project_id = Some(project_id);
        self.active_section = SoloSection::Chat;
        self.history_sidebar_open = true;
        cx.notify();
    }

    fn open_standalone_history(&mut self, cx: &mut Context<Self>) {
        self.upsert_active_conversation(cx);
        if self.active_project_id.is_some() {
            self.messages.clear();
            self.active_conversation_id = None;
            self.solo_attachments.clear();
            self.solo_expanded_messages.clear();
            self.reset_solo_chat_list_state();
        }
        self.solo_webview_open = false;
        self.solo_webview = None;
        self.solo_webview_generation = self.solo_webview_generation.wrapping_add(1);
        self.active_project_id = None;
        self.active_section = SoloSection::Chat;
        self.history_sidebar_open = true;
        cx.notify();
    }

    fn open_section(&mut self, section: SoloSection, cx: &mut Context<Self>) {
        self.upsert_active_conversation(cx);
        self.active_section = section;
        if section != SoloSection::Chat {
            self.history_sidebar_open = false;
            self.solo_webview_open = false;
            self.solo_webview = None;
            self.solo_webview_generation = self.solo_webview_generation.wrapping_add(1);
        }
        if section == SoloSection::Remote {
            self.refresh_remote_windows(cx);
        } else {
            self.stop_remote_stream(cx);
        }
        cx.notify();
    }

    fn sidebar_item(
        &self,
        label: &'static str,
        icon: IconName,
        section: SoloSection,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let active = self.active_section == section;
        h_flex()
            .id(gpui::ElementId::Name(
                format!("solo-sidebar-{}", label.to_lowercase()).into(),
            ))
            .w_full()
            .h(px(34.))
            .px(px(10.))
            .gap(px(8.))
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_section(section, cx);
            }))
            .bg(if active {
                theme.primary.opacity(0.12)
            } else {
                theme.background.opacity(0.0)
            })
            .text_color(if active {
                theme.foreground
            } else {
                theme.muted_foreground
            })
            .child(
                div()
                    .text_color(if active {
                        theme.primary
                    } else {
                        theme.muted_foreground
                    })
                    .child(Icon::new(icon).size(px(14.))),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.))
                    .font_weight(if active {
                        gpui::FontWeight::SEMIBOLD
                    } else {
                        gpui::FontWeight::MEDIUM
                    })
                    .child(label),
            )
            .into_any_element()
    }

    fn project_row(&self, project: SoloProject, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let active = self.active_section == SoloSection::Chat
            && self.active_project_id.as_deref() == Some(project.id.as_str());
        let project_id = project.id.clone();
        h_flex()
            .id(gpui::ElementId::Name(
                format!("solo-project-{}", project.id).into(),
            ))
            .w_full()
            .h(px(36.))
            .rounded_md()
            .bg(if active {
                theme.primary.opacity(0.10)
            } else {
                theme.background.opacity(0.0)
            })
            .px(px(10.))
            .gap(px(8.))
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_project_history(project_id.clone(), cx);
            }))
            .child(
                div()
                    .text_color(if active {
                        theme.primary
                    } else {
                        theme.muted_foreground
                    })
                    .child(Icon::empty().path("icons/square.svg").size(px(13.))),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(if active {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    })
                    .child(project.name),
            )
            .into_any_element()
    }

    fn open_new_project_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Project name"));
        let view = cx.entity().clone();
        let db = crate::AppState::global(cx).db.clone();

        window.open_dialog(cx, move |dialog, _, _| {
            let name_input_footer = name_input.clone();
            let view_footer = view.clone();
            let db_footer = db.clone();
            dialog
                .title("Create Project")
                .w(px(440.0))
                .child(
                    v_form().py(px(8.0)).child(
                        field()
                            .label("Name")
                            .required(true)
                            .child(Input::new(&name_input).w_full()),
                    ),
                )
                .footer(move |_, _, _, _| {
                    let name_input = name_input_footer.clone();
                    let view = view_footer.clone();
                    let db = db_footer.clone();
                    vec![
                        Button::new("solo-project-cancel")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx))
                            .into_any_element(),
                        Button::new("solo-project-create")
                            .primary()
                            .label("Create Project")
                            .on_click(move |_, window, cx| {
                                let name = name_input.read(cx).text().to_string();
                                let name = name.trim().to_string();
                                if name.is_empty() {
                                    window.push_notification(
                                        (NotificationType::Error, "Project name is required."),
                                        cx,
                                    );
                                    return;
                                }
                                match db.create_solo_project(&name) {
                                    Ok(project) => {
                                        let project_id = project.id.clone();
                                        view.update(cx, |this, cx| {
                                            this.projects.push(SoloProject {
                                                id: project.id,
                                                name: project.name,
                                            });
                                            this.open_project_history(project_id, cx);
                                        });
                                        window.close_dialog(cx);
                                    }
                                    Err(error) => window.push_notification(
                                        (
                                            NotificationType::Error,
                                            gpui::SharedString::from(format!(
                                                "Could not create project: {error}"
                                            )),
                                        ),
                                        cx,
                                    ),
                                }
                            })
                            .into_any_element(),
                    ]
                })
        });
    }

    fn standalone_conversation_row(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let active = self.active_section == SoloSection::Chat && self.active_project_id.is_none();
        h_flex()
            .id("solo-standalone-conversations")
            .w_full()
            .h(px(36.))
            .rounded_md()
            .bg(if active {
                theme.primary.opacity(0.10)
            } else {
                theme.background.opacity(0.0)
            })
            .px(px(10.))
            .gap(px(8.))
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                this.open_standalone_history(cx);
            }))
            .child(
                div()
                    .text_color(if active {
                        theme.primary
                    } else {
                        theme.muted_foreground
                    })
                    .child(Icon::empty().path("icons/square.svg").size(px(13.))),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(if active {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    })
                    .child("Personal Chats"),
            )
            .into_any_element()
    }

    pub(super) fn primary_sidebar(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let mut project_list = v_flex().gap(px(4.));
        for project in self.projects.iter().cloned() {
            project_list = project_list.child(self.project_row(project, cx));
        }

        v_flex()
            .w(px(248.))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.secondary.opacity(0.38))
            .p(px(10.))
            .child(
                v_flex()
                    .gap(px(8.))
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(6.))
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.muted_foreground)
                            .child("PERSONAL"),
                    )
                    .child(
                        Button::new("solo-new-chat")
                            .primary()
                            .label("New Chat")
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.start_new_chat(window, cx);
                            })),
                    )
                    .child(self.sidebar_item("Memory", IconName::BookOpen, SoloSection::Memory, cx))
                    .child(self.sidebar_item(
                        "Automations",
                        IconName::Bell,
                        SoloSection::Automations,
                        cx,
                    ))
                    .child(self.sidebar_item(
                        "Triggers",
                        IconName::Settings2,
                        SoloSection::Triggers,
                        cx,
                    ))
                    .child(self.sidebar_item("Remote", IconName::Eye, SoloSection::Remote, cx)),
            )
            .child(
                v_flex()
                    .gap(px(8.))
                    .border_t_1()
                    .border_color(theme.border)
                    .pt(px(10.))
                    .child(
                        div()
                            .px(px(4.))
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.muted_foreground)
                            .child("CONVERSATIONS"),
                    )
                    .child(self.standalone_conversation_row(cx))
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .mt(px(6.))
                            .px(px(4.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.muted_foreground)
                                    .child("PROJECTS"),
                            )
                            .child(
                                Button::new("solo-new-project")
                                    .small()
                                    .compact()
                                    .ghost()
                                    .icon(IconName::Plus)
                                    .tooltip("Create project")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_new_project_dialog(window, cx);
                                    })),
                            ),
                    )
                    .child(project_list),
            )
            .into_any_element()
    }

    pub(super) fn history_sidebar(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let mut history = v_flex().gap(px(8.));
        let project_conversations: Vec<&SoloConversation> = self
            .conversations
            .iter()
            .filter(|conversation| {
                conversation.project_id.as_deref() == self.active_project_id.as_deref()
            })
            .collect();

        if project_conversations.is_empty() {
            let empty_history_text = if self.active_project_id.is_some() {
                "No chat history for this project yet."
            } else {
                "No standalone conversations yet."
            };

            history = history.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background.opacity(0.34))
                    .p(px(10.))
                    .text_size(px(11.))
                    .line_height(gpui::relative(1.35))
                    .text_color(theme.muted_foreground)
                    .child(empty_history_text),
            );
        } else {
            for conversation in project_conversations {
                let id = conversation.id;
                let is_active = self.active_conversation_id == Some(id);
                history = history.child(
                    div()
                        .id(gpui::ElementId::Name(format!("solo-history-{}", id).into()))
                        .rounded_md()
                        .border_1()
                        .border_color(if is_active {
                            theme.primary.opacity(0.28)
                        } else {
                            theme.border
                        })
                        .bg(if is_active {
                            theme.primary.opacity(0.10)
                        } else {
                            theme.background.opacity(0.32)
                        })
                        .p(px(10.))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.load_conversation(id, window, cx);
                        }))
                        .child(
                            v_flex()
                                .gap(px(3.))
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(12.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme.foreground)
                                        .child(conversation.title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(theme.muted_foreground)
                                        .child(format!("{} messages", conversation.messages.len())),
                                ),
                        ),
                );
            }
        }

        v_flex()
            .w(px(260.))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.background)
            .p(px(10.))
            .gap(px(10.))
            .child(
                h_flex()
                    .h(px(34.))
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.foreground)
                            .child(self.active_project_name().to_string()),
                    )
                    .child(
                        Button::new("solo-history-close")
                            .small()
                            .compact()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_history_sidebar(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(history),
            )
            .into_any_element()
    }
}
