use gpui::{
    div, px, Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    scroll::ScrollableElement,
    v_flex, ActiveTheme as _, Icon, IconName, Sizable,
};

use super::{
    model::{solo_projects, SoloConversation, SoloProject, SoloSection},
    SoloWorkspacePanel,
};

impl SoloWorkspacePanel {
    fn active_project_name(&self) -> &'static str {
        let Some(active_project_id) = self.active_project_id else {
            return "Conversations";
        };

        solo_projects()
            .iter()
            .find(|project| project.id == active_project_id)
            .map(|project| project.name)
            .unwrap_or("Project")
    }

    fn close_history_sidebar(&mut self, cx: &mut Context<Self>) {
        self.history_sidebar_open = false;
        cx.notify();
    }

    fn open_project_history(&mut self, project_id: &'static str, cx: &mut Context<Self>) {
        self.upsert_active_conversation();
        self.active_project_id = Some(project_id);
        self.active_section = SoloSection::Chat;
        self.history_sidebar_open = true;
        cx.notify();
    }

    fn open_standalone_history(&mut self, cx: &mut Context<Self>) {
        self.upsert_active_conversation();
        self.active_project_id = None;
        self.active_section = SoloSection::Chat;
        self.history_sidebar_open = true;
        cx.notify();
    }

    fn open_section(&mut self, section: SoloSection, cx: &mut Context<Self>) {
        self.upsert_active_conversation();
        self.active_section = section;
        if section != SoloSection::Chat {
            self.history_sidebar_open = false;
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
        let active =
            self.active_section == SoloSection::Chat && self.active_project_id == Some(project.id);
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
                this.open_project_history(project.id, cx);
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
        for project in solo_projects() {
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
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(Icon::new(IconName::Plus).size(px(13.))),
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
            .filter(|conversation| conversation.project_id == self.active_project_id)
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
                            .child(self.active_project_name()),
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
