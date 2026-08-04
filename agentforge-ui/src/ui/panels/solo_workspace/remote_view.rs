use gpui::{
    canvas, div, img, px, Context, InteractiveElement, IntoElement, MouseButton, ObjectFit,
    ParentElement, StatefulInteractiveElement, Styled, StyledImage,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, ActiveTheme as _, Disableable, Icon, IconName, Selectable, Sizable,
};

use super::SoloWorkspacePanel;

impl SoloWorkspacePanel {
    pub(super) fn remote_page(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let selected_window = self.remote.selected_window.and_then(|window_id| {
            self.remote
                .windows
                .iter()
                .find(|window| window.window_id == window_id)
        });
        let viewer = selected_window.map(|desktop_window| {
            let viewer_height = if self.remote.viewer_expanded {
                900.0
            } else {
                720.0
            };
            let view_for_bounds = cx.entity().clone();
            let frame = if let Some(frame) = self.remote.stream_frame.clone() {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(img(frame).h_full().object_fit(ObjectFit::Contain))
                    .into_any_element()
            } else {
                let (title, detail) = if let Some(error) = self.remote.stream_error.as_deref() {
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
                                    Button::new("solo-remote-control-mode")
                                        .small()
                                        .compact()
                                        .label(if self.remote.control_enabled {
                                            "Control on"
                                        } else {
                                            "Observe"
                                        })
                                        .selected(self.remote.control_enabled)
                                        .tooltip(if self.remote.control_enabled {
                                            "Clicks inside the viewer are forwarded to the source window"
                                        } else {
                                            "Enable guarded click control for this viewer"
                                        })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.remote.control_enabled =
                                                !this.remote.control_enabled;
                                            this.remote.control_message = Some(
                                                if this.remote.control_enabled {
                                                    "Control enabled for this session; click inside the live frame"
                                                } else {
                                                    "Viewer returned to observe-only mode"
                                                }
                                                .to_string(),
                                            );
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("solo-remote-expand-viewer")
                                        .small()
                                        .compact()
                                        .ghost()
                                        .icon(if self.remote.viewer_expanded {
                                            IconName::Minimize
                                        } else {
                                            IconName::Maximize
                                        })
                                        .tooltip(if self.remote.viewer_expanded {
                                            "Reduce viewer"
                                        } else {
                                            "Enlarge viewer"
                                        })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.remote.viewer_expanded =
                                                !this.remote.viewer_expanded;
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
                                .relative()
                                .size_full()
                                .overflow_hidden()
                                .rounded_md()
                                .bg(gpui::black())
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(
                                        |this, event: &gpui::MouseDownEvent, _, cx| {
                                            this.forward_remote_click(event.position, cx);
                                        },
                                    ),
                                )
                                .child(frame)
                                .child(
                                    canvas(
                                        move |bounds, _, cx| {
                                            view_for_bounds.update(cx, |this, _| {
                                                this.remote.viewer_bounds = Some(bounds);
                                            });
                                        },
                                        |_, _, _, _| {},
                                    )
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .size_full(),
                                ),
                        ),
                )
                .into_any_element()
        });

        let mut window_grid = h_flex().w_full().gap(px(8.0)).flex_wrap();
        if self.remote.windows.is_empty() {
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
                    .child(if self.remote.refreshing {
                        "Reading the current desktop..."
                    } else {
                        "No visible application windows were found."
                    }),
            );
        } else {
            for desktop_window in &self.remote.windows {
                let selected = self.remote.selected_window == Some(desktop_window.window_id);
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
            .remote
            .last_refreshed
            .as_ref()
            .map(|time| format!("Scanned {time}"))
            .unwrap_or_else(|| "Not scanned yet".to_string());
        let remote_mode_text = if self.remote.control_enabled {
            self.remote.control_message.clone().unwrap_or_else(|| {
                "Control enabled · clicks are forwarded locally · keyboard input is not enabled"
                    .to_string()
            })
        } else {
            "Observe only · cursor hidden · no clicks or typing are forwarded".to_string()
        };
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
                                    .child(remote_mode_text),
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
                                    .child(format!("Windows ({})", self.remote.windows.len())),
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
                            .disabled(self.remote.refreshing)
                            .label(if self.remote.refreshing {
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
