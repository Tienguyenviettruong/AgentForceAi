use gpui::{
    div, linear_color_stop, linear_gradient, px, Context, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::Input,
    scroll::ScrollableElement,
    v_flex, ActiveTheme as _, Icon, IconName,
};

use super::{
    model::{solo_templates, SoloMessage, SoloSection, SoloTemplate},
    SoloWorkspacePanel,
};

impl SoloWorkspacePanel {
    pub(super) fn submit_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = self.prompt_input.read(cx).text().to_string();
        let prompt = prompt.trim().to_string();
        if prompt.is_empty() {
            return;
        }

        self.messages.push(SoloMessage {
            role: "user",
            content: prompt.clone(),
        });
        self.messages.push(SoloMessage {
            role: "assistant",
            content: format!(
                "Captured as a personal task. The next version will route this through solo memory, background triggers, and guarded desktop actions.\n\n{}",
                prompt
            ),
        });

        self.prompt_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        self.upsert_active_conversation();
        cx.notify();
    }

    fn on_prompt_confirm(
        &mut self,
        _: &crate::ChatComposerConfirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.submit_prompt(window, cx);
    }

    fn prefill_prompt(
        &mut self,
        prompt: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.prompt_input.update(cx, |state, cx| {
            state.set_value(prompt, window, cx);
        });
        cx.notify();
    }

    pub(super) fn prefill_personal_prompt(
        &mut self,
        prompt: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_section = SoloSection::Chat;
        self.active_project_id = None;
        self.history_sidebar_open = true;
        self.prefill_prompt(prompt, window, cx);
    }

    fn conversation_title(messages: &[SoloMessage]) -> String {
        messages
            .iter()
            .find(|message| message.role == "user")
            .map(|message| {
                let mut title = message.content.trim().replace('\n', " ");
                if title.chars().count() > 42 {
                    title = title.chars().take(42).collect::<String>();
                    title.push_str("...");
                }
                title
            })
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| "Untitled chat".to_string())
    }

    pub(super) fn upsert_active_conversation(&mut self) {
        if self.messages.is_empty() {
            return;
        }

        let title = Self::conversation_title(&self.messages);
        if let Some(id) = self.active_conversation_id {
            if let Some(conversation) = self
                .conversations
                .iter_mut()
                .find(|conversation| conversation.id == id)
            {
                conversation.title = title;
                conversation.messages = self.messages.clone();
                conversation.project_id = self.active_project_id;
                return;
            }
        }

        let id = self.next_conversation_id;
        self.next_conversation_id += 1;
        self.active_conversation_id = Some(id);
        self.conversations.insert(
            0,
            super::model::SoloConversation {
                id,
                project_id: self.active_project_id,
                title,
                messages: self.messages.clone(),
            },
        );
    }

    pub(super) fn start_new_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.upsert_active_conversation();
        self.messages.clear();
        self.active_conversation_id = None;
        self.active_project_id = None;
        self.active_section = SoloSection::Chat;
        self.history_sidebar_open = true;
        self.prompt_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        cx.notify();
    }

    pub(super) fn load_conversation(
        &mut self,
        id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.upsert_active_conversation();
        if let Some(conversation) = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .cloned()
        {
            self.messages = conversation.messages;
            self.active_conversation_id = Some(id);
            self.active_project_id = conversation.project_id;
            self.active_section = SoloSection::Chat;
            self.history_sidebar_open = true;
            self.prompt_input.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
            cx.notify();
        }
    }

    fn template_card(&self, template: SoloTemplate, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let background = linear_gradient(
            135.,
            linear_color_stop(template.base.opacity(0.58), 0.),
            linear_color_stop(template.accent.opacity(0.34), 1.),
        );

        div()
            .id(gpui::ElementId::Name(
                format!("solo-template-card-{}", template.title).into(),
            ))
            .w(px(196.))
            .h(px(128.))
            .rounded_md()
            .border_1()
            .border_color(template.accent.opacity(0.34))
            .bg(background)
            .p(px(16.))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.prefill_prompt(template.prompt, window, cx);
            }))
            .child(
                v_flex()
                    .size_full()
                    .justify_between()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_start()
                            .child(
                                div()
                                    .w(px(36.))
                                    .h(px(36.))
                                    .rounded_md()
                                    .bg(theme.background.opacity(0.34))
                                    .border_1()
                                    .border_color(theme.foreground.opacity(0.08))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(theme.foreground)
                                    .child(Icon::new(template.icon).size(px(17.))),
                            )
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded_md()
                                    .bg(theme.background.opacity(0.26))
                                    .text_size(px(10.))
                                    .text_color(theme.foreground.opacity(0.78))
                                    .child("Solo"),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap(px(5.))
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.foreground)
                                    .child(template.title),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .line_height(gpui::relative(1.3))
                                    .text_color(theme.foreground.opacity(0.74))
                                    .child(template.detail),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_messages(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let mut messages = v_flex().gap(px(10.)).w_full();

        for message in &self.messages {
            let is_user = message.role == "user";
            messages = messages.child(
                v_flex()
                    .w_full()
                    .gap(px(4.))
                    .items_start()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(if is_user {
                                theme.primary
                            } else {
                                theme.muted_foreground
                            })
                            .child(if is_user { "You" } else { "Personal AI" }),
                    )
                    .child(
                        div()
                            .max_w(px(820.))
                            .rounded_md()
                            .border_1()
                            .border_color(if is_user {
                                theme.primary.opacity(0.24)
                            } else {
                                theme.border
                            })
                            .bg(if is_user {
                                theme.primary.opacity(0.08)
                            } else {
                                theme.secondary.opacity(0.30)
                            })
                            .p(px(11.))
                            .text_size(px(12.))
                            .line_height(gpui::relative(1.45))
                            .text_color(theme.foreground)
                            .child(message.content.clone()),
                    ),
            );
        }

        div().w_full().child(messages).into_any_element()
    }

    fn tool_chip(
        &self,
        label: &'static str,
        icon: IconName,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        h_flex()
            .h(px(32.))
            .px(px(10.))
            .gap(px(7.))
            .items_center()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary.opacity(0.54))
            .text_color(theme.foreground)
            .child(Icon::new(icon).size(px(14.)))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(label),
            )
            .into_any_element()
    }

    fn composer(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();

        v_flex()
            .w_full()
            .h(px(136.))
            .min_h(px(136.))
            .max_h(px(136.))
            .flex_shrink_0()
            .rounded(px(16.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .overflow_hidden()
            .key_context("SoloChatComposer")
            .on_action(cx.listener(Self::on_prompt_confirm))
            .child(
                div().w_full().h(px(92.)).p(px(16.)).child(
                    Input::new(&self.prompt_input)
                        .appearance(false)
                        .w_full()
                        .h_full(),
                ),
            )
            .child(
                h_flex()
                    .h(px(44.))
                    .w_full()
                    .px(px(14.))
                    .border_t_1()
                    .border_color(theme.border)
                    .justify_between()
                    .items_center()
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                div()
                                    .w(px(28.))
                                    .h(px(28.))
                                    .rounded_md()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(theme.muted_foreground)
                                    .child(Icon::new(IconName::Plus).size(px(16.))),
                            )
                            .child(self.tool_chip("Build", IconName::SquareTerminal, cx))
                            .child(self.tool_chip("Skills", IconName::Settings2, cx))
                            .child(self.tool_chip("Remote", IconName::Eye, cx)),
                    )
                    .child(
                        Button::new("solo-send")
                            .primary()
                            .label("Send")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.submit_prompt(window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn chat_content(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let has_messages = !self.messages.is_empty();
        let mut template_row = h_flex()
            .w_full()
            .max_w(px(880.))
            .gap(px(12.))
            .flex_wrap()
            .items_center()
            .justify_center();
        for template in solo_templates() {
            template_row = template_row.child(self.template_card(template, cx));
        }

        let header = v_flex()
            .w_full()
            .max_w(px(880.))
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(24.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme.foreground)
                    .child("Solo Mode"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(gpui::relative(1.35))
                    .text_align(gpui::TextAlign::Center)
                    .text_color(theme.muted_foreground)
                    .child("Private AI workspace for your own domains, projects, tasks, automations, memory, and guarded desktop control."),
            );

        if !has_messages {
            v_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .items_center()
                .justify_center()
                .px(px(28.))
                .pb(px(20.))
                .child(
                    v_flex()
                        .w_full()
                        .max_w(px(880.))
                        .items_center()
                        .gap(px(18.))
                        .child(header)
                        .child(template_row)
                        .child(self.composer(cx)),
                )
                .into_any_element()
        } else {
            v_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .items_center()
                .px(px(28.))
                .pt(px(18.))
                .pb(px(20.))
                .gap(px(12.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(880.))
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .pb(px(12.))
                        .child(self.render_messages(cx)),
                )
                .child(
                    div()
                        .w_full()
                        .max_w(px(880.))
                        .flex_shrink_0()
                        .child(self.composer(cx)),
                )
                .into_any_element()
        }
    }
}
