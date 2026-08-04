use gpui::{div, px, Context, Hsla, IntoElement, ParentElement, Styled, Window};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    scroll::ScrollableElement,
    v_flex, ActiveTheme as _, Icon, IconName, Sizable,
};

use super::SoloWorkspacePanel;

impl SoloWorkspacePanel {
    fn prompt_button(
        &self,
        id: &'static str,
        label: &'static str,
        prompt: &'static str,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        Button::new(id)
            .small()
            .ghost()
            .label(label)
            .on_click(cx.listener(move |this, _, window: &mut Window, cx| {
                this.prefill_personal_prompt(prompt, window, cx);
            }))
            .into_any_element()
    }

    pub(super) fn badge(
        &self,
        label: &'static str,
        color: Hsla,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        div()
            .px(px(8.))
            .py(px(3.))
            .rounded_md()
            .bg(color.opacity(0.12))
            .border_1()
            .border_color(color.opacity(0.22))
            .text_size(px(10.))
            .text_color(if label == "Paused" {
                theme.muted_foreground
            } else {
                color
            })
            .child(label)
            .into_any_element()
    }

    fn summary_card(
        &self,
        title: &'static str,
        value: &'static str,
        detail: &'static str,
        icon: IconName,
        color: Hsla,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        div()
            .w(px(220.))
            .min_h(px(126.))
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary.opacity(0.24))
            .p(px(14.))
            .child(
                v_flex()
                    .size_full()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .w(px(34.))
                                    .h(px(34.))
                                    .rounded_md()
                                    .bg(color.opacity(0.12))
                                    .text_color(color)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(Icon::new(icon).size(px(16.))),
                            )
                            .child(
                                div()
                                    .text_size(px(24.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.foreground)
                                    .child(value),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.foreground)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .line_height(gpui::relative(1.35))
                                    .text_color(theme.muted_foreground)
                                    .child(detail),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn list_panel(
        &self,
        title: &'static str,
        subtitle: &'static str,
        rows: &[(&'static str, &'static str, IconName, &'static str)],
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let mut list = v_flex().gap(px(8.));
        for (row_title, row_detail, icon, state) in rows {
            list = list.child(
                h_flex()
                    .w_full()
                    .items_start()
                    .gap(px(10.))
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background.opacity(0.42))
                    .p(px(11.))
                    .child(
                        div()
                            .w(px(30.))
                            .h(px(30.))
                            .flex_shrink_0()
                            .rounded_md()
                            .bg(theme.primary.opacity(0.10))
                            .text_color(theme.primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(icon.clone()).size(px(15.))),
                    )
                    .child(
                        v_flex()
                            .min_w_0()
                            .flex_1()
                            .gap(px(4.))
                            .child(
                                h_flex()
                                    .justify_between()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(px(12.))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(theme.foreground)
                                            .child(*row_title),
                                    )
                                    .child(self.badge(*state, theme.primary, cx)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .line_height(gpui::relative(1.4))
                                    .text_color(theme.muted_foreground)
                                    .child(*row_detail),
                            ),
                    ),
            );
        }

        v_flex()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary.opacity(0.18))
            .p(px(14.))
            .gap(px(12.))
            .child(
                v_flex()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.foreground)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(subtitle),
                    ),
            )
            .child(list)
            .into_any_element()
    }

    pub(super) fn section_page(
        &self,
        title: &'static str,
        subtitle: &'static str,
        icon: IconName,
        body: gpui::AnyElement,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let is_remote = title == "Remote";
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scrollbar()
            .px(px(if is_remote { 20.0 } else { 32.0 }))
            .py(px(if is_remote { 18.0 } else { 28.0 }))
            .child(
                v_flex()
                    .w_full()
                    .max_w(px(if is_remote { 10_000.0 } else { 1020.0 }))
                    .gap(px(if is_remote { 12.0 } else { 18.0 }))
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                div()
                                    .w(px(42.))
                                    .h(px(42.))
                                    .rounded_md()
                                    .bg(theme.primary.opacity(0.10))
                                    .text_color(theme.primary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(Icon::new(icon).size(px(20.))),
                            )
                            .child(
                                v_flex()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .text_size(px(24.))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.foreground)
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(gpui::relative(1.4))
                                            .text_color(theme.muted_foreground)
                                            .child(subtitle),
                                    ),
                            ),
                    )
                    .child(body),
            )
            .into_any_element()
    }

    pub(super) fn memory_page(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let accent = Hsla::from(gpui::rgb(0x0ea5e9));
        let cards = h_flex()
            .w_full()
            .gap(px(12.))
            .flex_wrap()
            .child(self.summary_card(
                "Profile Memory",
                "18",
                "Stable preferences, writing style, and tool boundaries.",
                IconName::BookOpen,
                accent,
                cx,
            ))
            .child(self.summary_card(
                "Project Notes",
                "07",
                "Project-specific context separated from Team mode.",
                IconName::SquareTerminal,
                Hsla::from(gpui::rgb(0x10b981)),
                cx,
            ))
            .child(self.summary_card(
                "Review Queue",
                "03",
                "Candidate memories waiting for confirmation.",
                IconName::Search,
                Hsla::from(gpui::rgb(0xf59e0b)),
                cx,
            ));

        let body = v_flex()
            .gap(px(14.))
            .child(cards)
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(self.prompt_button(
                        "solo-memory-capture",
                        "Capture memory from chat",
                        "Review this conversation and suggest personal memories worth saving: ",
                        cx,
                    ))
                    .child(self.prompt_button(
                        "solo-memory-cleanup",
                        "Clean up memory",
                        "Audit my personal memory and propose what to merge, keep, or remove.",
                        cx,
                    )),
            )
            .child(self.list_panel(
                "Memory Inbox",
                "Candidates are private to Solo Mode until you approve them.",
                &[
                    (
                        "Coding preferences",
                        "Prefer concise plans, scoped edits, and verification commands before handoff.",
                        IconName::SquareTerminal,
                        "Ready",
                    ),
                    (
                        "Research vocabulary",
                        "Keep domain notes reusable across standalone chats and personal projects.",
                        IconName::Search,
                        "Draft",
                    ),
                    (
                        "Remote permissions",
                        "Ask before desktop actions that click, type, install, or move files.",
                        IconName::Eye,
                        "Guarded",
                    ),
                ],
                cx,
            ))
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.primary.opacity(0.06))
                    .p(px(14.))
                    .text_size(px(12.))
                    .line_height(gpui::relative(1.45))
                    .text_color(theme.foreground)
                    .child("Memory is intentionally separate from Team Workspace. Project memories can be attached later without leaking into shared team context."),
            );

        self.section_page(
            "Personal Memory",
            "Private context, preferences, and reusable knowledge for Solo Mode.",
            IconName::BookOpen,
            body.into_any_element(),
            cx,
        )
    }

    pub(super) fn automations_page(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let cards = h_flex()
            .w_full()
            .gap(px(12.))
            .flex_wrap()
            .child(self.summary_card(
                "Active",
                "03",
                "Running quietly in the personal workspace.",
                IconName::Bell,
                Hsla::from(gpui::rgb(0x8b5cf6)),
                cx,
            ))
            .child(self.summary_card(
                "Paused",
                "01",
                "Disabled until permissions are reviewed.",
                IconName::Settings2,
                Hsla::from(gpui::rgb(0x64748b)),
                cx,
            ))
            .child(self.summary_card(
                "Needs You",
                "02",
                "Sensitive actions waiting for approval.",
                IconName::Eye,
                Hsla::from(gpui::rgb(0xf97316)),
                cx,
            ));

        let body = v_flex()
            .gap(px(14.))
            .child(cards)
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(self.prompt_button(
                        "solo-automation-build",
                        "Design automation",
                        "Design a personal automation for this recurring workflow: ",
                        cx,
                    ))
                    .child(self.prompt_button(
                        "solo-automation-audit",
                        "Audit automations",
                        "Review my personal automations and identify risky or duplicated routines.",
                        cx,
                    )),
            )
            .child(self.list_panel(
                "Automation Runs",
                "Personal routines stay outside the Team task runner.",
                &[
                    (
                        "Daily research digest",
                        "Collect updates, summarize changes, and prepare a morning brief.",
                        IconName::Search,
                        "Active",
                    ),
                    (
                        "Repo health check",
                        "Run selected checks and summarize failures before work starts.",
                        IconName::SquareTerminal,
                        "Active",
                    ),
                    (
                        "Follow-up reminders",
                        "Surface personal promises and unresolved chat requests.",
                        IconName::Bell,
                        "Paused",
                    ),
                ],
                cx,
            ));

        self.section_page(
            "Automations",
            "Personal recurring work, local routines, and approval-aware background runs.",
            IconName::Bell,
            body.into_any_element(),
            cx,
        )
    }

    pub(super) fn triggers_page(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let cards = h_flex()
            .w_full()
            .gap(px(12.))
            .flex_wrap()
            .child(self.summary_card(
                "Schedules",
                "04",
                "Time-based prompts and reminders.",
                IconName::Bell,
                Hsla::from(gpui::rgb(0x06b6d4)),
                cx,
            ))
            .child(self.summary_card(
                "File Signals",
                "02",
                "Watch local project changes before acting.",
                IconName::BookOpen,
                Hsla::from(gpui::rgb(0x22c55e)),
                cx,
            ))
            .child(self.summary_card(
                "Manual Intents",
                "05",
                "Reusable prompt starts from the composer.",
                IconName::Settings2,
                Hsla::from(gpui::rgb(0xa855f7)),
                cx,
            ));

        let body = v_flex()
            .gap(px(14.))
            .child(cards)
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(self.prompt_button(
                        "solo-trigger-create",
                        "Create trigger",
                        "Create a personal trigger with clear condition, action, permission gate, and rollback: ",
                        cx,
                    ))
                    .child(self.prompt_button(
                        "solo-trigger-map",
                        "Map triggers",
                        "Map useful triggers for my personal workspace and group them by risk.",
                        cx,
                    )),
            )
            .child(self.list_panel(
                "Trigger Rules",
                "Rules can start suggestions, not uncontrolled desktop actions.",
                &[
                    (
                        "When a project changes",
                        "Summarize modified files and ask before running verification.",
                        IconName::BookOpen,
                        "Guarded",
                    ),
                    (
                        "When a deadline is near",
                        "Prepare a progress brief and suggest the next action.",
                        IconName::Bell,
                        "Active",
                    ),
                    (
                        "When I type a remote task",
                        "Require confirmation before keyboard or mouse control.",
                        IconName::Eye,
                        "Guarded",
                    ),
                ],
                cx,
            ));

        self.section_page(
            "Triggers",
            "Personal event rules that wake up automations without mixing into Team mode.",
            IconName::Settings2,
            body.into_any_element(),
            cx,
        )
    }
}
