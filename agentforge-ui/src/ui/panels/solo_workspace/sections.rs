use gpui::{
    div, img, px, Context, Hsla, InteractiveElement, IntoElement, ObjectFit, ParentElement,
    StatefulInteractiveElement, Styled, StyledImage, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    scroll::ScrollableElement,
    v_flex, ActiveTheme as _, Disableable, Icon, IconName, Sizable,
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

    fn badge(&self, label: &'static str, color: Hsla, cx: &Context<Self>) -> gpui::AnyElement {
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

    fn section_page(
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

    pub(super) fn refresh_remote_windows(&mut self, cx: &mut Context<Self>) {
        if self.remote_refreshing {
            return;
        }
        self.remote_refreshing = true;
        cx.notify();
        let view = cx.entity().clone();
        cx.spawn(async move |_, cx| {
            let windows =
                smol::unblock(crate::infrastructure::desktop_monitor::list_desktop_windows).await;
            let _ = cx.update(|cx| {
                let _ = view.update(cx, |this, cx| {
                    this.remote_windows = windows;
                    this.remote_refreshing = false;
                    this.remote_last_refreshed =
                        Some(chrono::Local::now().format("%H:%M:%S").to_string());
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn start_remote_stream(&mut self, window_id: isize, cx: &mut Context<Self>) {
        if self.remote_selected_window == Some(window_id) && self.remote_streaming {
            return;
        }
        self.remote_selected_window = Some(window_id);
        self.remote_stream_frame = None;
        self.remote_stream_error = None;
        self.remote_streaming = true;
        self.remote_viewer_expanded = false;
        self.remote_stream_generation = self.remote_stream_generation.wrapping_add(1);
        let generation = self.remote_stream_generation;
        let view = cx.entity().clone();
        cx.notify();

        cx.spawn(async move |_, cx| {
            let session = smol::unblock(move || {
                crate::infrastructure::desktop_monitor::start_desktop_window_stream(window_id)
            })
            .await;
            let session = match session {
                Ok(session) => session,
                Err(error) => {
                    let _ = cx.update(|cx| {
                        let _ = view.update(cx, |this, cx| {
                            if this.remote_stream_generation == generation {
                                this.remote_streaming = false;
                                this.remote_stream_error = Some(error);
                                cx.notify();
                            }
                        });
                    });
                    return;
                }
            };

            loop {
                let event = session.take_latest();
                let keep_streaming = cx
                    .update(|cx| {
                        let active = {
                            let this = view.read(cx);
                            this.active_section == super::model::SoloSection::Remote
                                && this.remote_streaming
                                && this.remote_stream_generation == generation
                                && this.remote_selected_window == Some(window_id)
                        };
                        if active {
                            if let Some(event) = event {
                                let _ = view.update(cx, |this, cx| {
                                    match event {
                                        crate::infrastructure::desktop_monitor::DesktopStreamEvent::Frame(frame) => {
                                            this.remote_stream_frame = Some(frame);
                                            this.remote_stream_error = None;
                                        }
                                        crate::infrastructure::desktop_monitor::DesktopStreamEvent::Closed => {
                                            this.remote_streaming = false;
                                            this.remote_stream_error =
                                                Some("The source window was closed.".to_string());
                                        }
                                        crate::infrastructure::desktop_monitor::DesktopStreamEvent::Error(error) => {
                                            this.remote_streaming = false;
                                            this.remote_stream_error = Some(error);
                                        }
                                    }
                                    cx.notify();
                                });
                            }
                        }
                        active
                    })
                    .unwrap_or(false);
                if !keep_streaming {
                    break;
                }
                smol::Timer::after(std::time::Duration::from_millis(16)).await;
            }
        })
        .detach();
    }

    pub(super) fn stop_remote_stream(&mut self, cx: &mut Context<Self>) {
        self.remote_streaming = false;
        self.remote_stream_generation = self.remote_stream_generation.wrapping_add(1);
        self.remote_selected_window = None;
        self.remote_stream_frame = None;
        self.remote_stream_error = None;
        self.remote_viewer_expanded = false;
        cx.notify();
    }

    pub(super) fn remote_page(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let selected_window = self.remote_selected_window.and_then(|window_id| {
            self.remote_windows
                .iter()
                .find(|window| window.window_id == window_id)
        });
        let viewer = selected_window.map(|desktop_window| {
            let viewer_height = if self.remote_viewer_expanded {
                900.0
            } else {
                720.0
            };
            let frame = if let Some(frame) = self.remote_stream_frame.clone() {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(img(frame).h_full().object_fit(ObjectFit::Contain))
                    .into_any_element()
            } else {
                let (title, detail) = if let Some(error) = self.remote_stream_error.as_deref() {
                    ("Live stream stopped".to_string(), error.to_string())
                } else if desktop_window.minimized {
                    (
                        "Window is minimized".to_string(),
                        "Restore it on the desktop so Windows can deliver new GPU frames."
                            .to_string(),
                    )
                } else {
                    (
                        "Starting GPU live stream...".to_string(),
                        "Waiting for the first complete Windows Graphics Capture frame."
                            .to_string(),
                    )
                };
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .gap(px(10.0))
                    .text_color(theme.muted_foreground)
                    .child(
                        div()
                            .w(px(48.0))
                            .h(px(48.0))
                            .rounded_full()
                            .bg(theme.primary.opacity(0.12))
                            .text_color(theme.primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::Eye).size(px(22.0))),
                    )
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(title),
                    )
                    .child(
                        div()
                            .max_w(px(520.0))
                            .text_align(gpui::TextAlign::Center)
                            .text_size(px(11.0))
                            .line_height(gpui::relative(1.45))
                            .child(detail),
                    )
                    .into_any_element()
            };
            v_flex()
                .w_full()
                .overflow_hidden()
                .rounded_lg()
                .border_1()
                .border_color(theme.primary.opacity(0.42))
                .bg(theme.secondary.opacity(0.35))
                .child(
                    h_flex()
                        .w_full()
                        .h(px(42.0))
                        .flex_shrink_0()
                        .px(px(12.0))
                        .justify_between()
                        .items_center()
                        .bg(theme.background)
                        .child(
                            h_flex()
                                .min_w_0()
                                .gap(px(8.0))
                                .items_center()
                                .child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(gpui::green()))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(desktop_window.title.clone()),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .text_size(px(10.0))
                                        .text_color(theme.muted_foreground)
                                        .child("LIVE · WINDOWS GRAPHICS CAPTURE"),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap(px(4.0))
                                .child(
                                    Button::new("solo-remote-expand-viewer")
                                        .small()
                                        .compact()
                                        .ghost()
                                        .icon(if self.remote_viewer_expanded {
                                            IconName::Minimize
                                        } else {
                                            IconName::Maximize
                                        })
                                        .tooltip(if self.remote_viewer_expanded {
                                            "Reduce viewer"
                                        } else {
                                            "Enlarge viewer"
                                        })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.remote_viewer_expanded =
                                                !this.remote_viewer_expanded;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("solo-remote-close-viewer")
                                        .small()
                                        .compact()
                                        .ghost()
                                        .icon(IconName::Close)
                                        .tooltip("Close live viewer")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.stop_remote_stream(cx);
                                        })),
                                ),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(viewer_height))
                        .p(px(8.0))
                        .bg(theme.secondary.opacity(0.22))
                        .child(
                            div()
                                .size_full()
                                .overflow_hidden()
                                .rounded_md()
                                .bg(gpui::black())
                                .child(frame),
                        ),
                )
                .into_any_element()
        });

        let mut window_grid = h_flex().w_full().gap(px(8.0)).flex_wrap();
        if self.remote_windows.is_empty() {
            window_grid = window_grid.child(
                v_flex()
                    .w_full()
                    .items_center()
                    .gap(px(8.0))
                    .rounded_lg()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.secondary.opacity(0.2))
                    .p(px(20.0))
                    .text_color(theme.muted_foreground)
                    .child(Icon::new(IconName::Eye).size(px(24.0)))
                    .child(if self.remote_refreshing {
                        "Reading the current desktop..."
                    } else {
                        "No visible application windows were found."
                    }),
            );
        } else {
            for desktop_window in &self.remote_windows {
                let selected = self.remote_selected_window == Some(desktop_window.window_id);
                let window_id = desktop_window.window_id;
                let preview = if let Some(path) = desktop_window.thumbnail_path.clone() {
                    img(path)
                        .w_full()
                        .h(px(106.0))
                        .object_fit(ObjectFit::Cover)
                        .into_any_element()
                } else {
                    v_flex()
                        .w_full()
                        .h(px(106.0))
                        .items_center()
                        .justify_center()
                        .gap(px(6.0))
                        .bg(theme.secondary.opacity(0.45))
                        .text_color(theme.muted_foreground)
                        .child(Icon::new(IconName::Eye).size(px(22.0)))
                        .child(if desktop_window.minimized {
                            "Minimized"
                        } else {
                            "Preview unavailable"
                        })
                        .into_any_element()
                };
                window_grid = window_grid.child(
                    v_flex()
                        .id(gpui::ElementId::Name(
                            format!("remote-window-{}", desktop_window.window_id).into(),
                        ))
                        .w(px(220.0))
                        .overflow_hidden()
                        .rounded_lg()
                        .border_1()
                        .border_color(if selected {
                            theme.primary
                        } else {
                            theme.border
                        })
                        .bg(if selected {
                            theme.primary.opacity(0.08)
                        } else {
                            theme.background
                        })
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.start_remote_stream(window_id, cx);
                        }))
                        .child(preview)
                        .child(
                            v_flex()
                                .gap(px(3.0))
                                .p(px(8.0))
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(11.0))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme.foreground)
                                        .child(desktop_window.title.clone()),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(10.0))
                                        .text_color(theme.muted_foreground)
                                        .child(desktop_window.application.clone()),
                                ),
                        ),
                );
            }
        }

        let status = self
            .remote_last_refreshed
            .as_ref()
            .map(|time| format!("Scanned {time}"))
            .unwrap_or_else(|| "Not scanned yet".to_string());
        let body = v_flex()
            .gap(px(10.0))
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .border_1()
                    .border_color(theme.primary.opacity(0.25))
                    .bg(theme.primary.opacity(0.07))
                    .rounded_md()
                    .px(px(10.0))
                    .py(px(7.0))
                    .child(
                        h_flex()
                            .min_w_0()
                            .gap(px(8.0))
                            .items_center()
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_color(theme.primary)
                                    .child(Icon::new(IconName::Eye).size(px(14.0))),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(theme.muted_foreground)
                                    .child("Local monitor only · live frames stay on this computer · no clicks or typing"),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(10.0))
                            .text_color(theme.muted_foreground)
                            .child(status.clone()),
                    ),
            )
            .children(viewer)
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(format!("Windows ({})", self.remote_windows.len())),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.muted_foreground)
                                    .child("Select a window to open its live viewer"),
                            ),
                    )
                    .child(
                        Button::new("solo-remote-refresh")
                            .small()
                            .ghost()
                            .disabled(self.remote_refreshing)
                            .label(if self.remote_refreshing {
                                "Refreshing..."
                            } else {
                                "Rescan windows"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh_remote_windows(cx);
                            })),
                    ),
            )
            .child(window_grid)
            .child(
                h_flex()
                    .gap(px(8.0))
                    .text_size(px(10.0))
                    .text_color(theme.muted_foreground)
                    .child(self.badge("Observe: enabled", theme.primary, cx))
                    .child(self.badge("Control: ask first", theme.primary, cx))
                    .child(self.badge("Sensitive changes: guarded", theme.primary, cx)),
            );

        self.section_page(
            "Remote",
            "Monitor open desktop windows and enlarge one local live view.",
            IconName::Eye,
            body.into_any_element(),
            cx,
        )
    }
}
