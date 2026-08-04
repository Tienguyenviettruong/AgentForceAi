use gpui::Context;

use super::SoloWorkspacePanel;

#[cfg(target_os = "windows")]
fn render_desktop_frame(
    frame: crate::infrastructure::desktop_monitor::DesktopFrame,
) -> Option<std::sync::Arc<gpui::RenderImage>> {
    let image = image::RgbaImage::from_raw(frame.width, frame.height, frame.pixels)?;
    Some(std::sync::Arc::new(gpui::RenderImage::new(vec![
        image::Frame::new(image),
    ])))
}

#[cfg(not(target_os = "windows"))]
fn render_desktop_frame(
    _frame: crate::infrastructure::desktop_monitor::DesktopFrame,
) -> Option<std::sync::Arc<gpui::RenderImage>> {
    None
}

#[derive(Default)]
pub(super) struct RemoteState {
    pub(super) windows: Vec<crate::infrastructure::desktop_monitor::DesktopWindowPreview>,
    pub(super) refreshing: bool,
    pub(super) last_refreshed: Option<String>,
    pub(super) selected_window: Option<isize>,
    pub(super) stream_frame: Option<std::sync::Arc<gpui::RenderImage>>,
    pub(super) stream_frame_size: Option<(u32, u32)>,
    pub(super) stream_error: Option<String>,
    pub(super) streaming: bool,
    pub(super) stream_generation: usize,
    pub(super) stream_cancel: Option<async_channel::Sender<()>>,
    pub(super) viewer_expanded: bool,
    pub(super) control_enabled: bool,
    pub(super) viewer_bounds: Option<gpui::Bounds<gpui::Pixels>>,
    pub(super) control_message: Option<String>,
}

impl SoloWorkspacePanel {
    pub(super) fn refresh_remote_windows(&mut self, cx: &mut Context<Self>) {
        if self.remote.refreshing {
            return;
        }
        self.remote.refreshing = true;
        cx.notify();
        let view = cx.entity().clone();
        cx.spawn(async move |_, cx| {
            let windows =
                smol::unblock(crate::infrastructure::desktop_monitor::list_desktop_windows).await;
            let _ = cx.update(|cx| {
                view.update(cx, |this, cx| {
                    this.remote.windows = windows;
                    this.remote.refreshing = false;
                    this.remote.last_refreshed =
                        Some(chrono::Local::now().format("%H:%M:%S").to_string());
                    cx.notify();
                });
            });
        })
        .detach();
    }

    pub(super) fn start_remote_stream(&mut self, window_id: isize, cx: &mut Context<Self>) {
        if self.remote.selected_window == Some(window_id) && self.remote.streaming {
            return;
        }
        if let Some(cancel) = self.remote.stream_cancel.take() {
            let _ = cancel.try_send(());
        }
        self.remote.selected_window = Some(window_id);
        self.remote.stream_frame = None;
        self.remote.stream_frame_size = None;
        self.remote.stream_error = None;
        self.remote.control_enabled = false;
        self.remote.viewer_bounds = None;
        self.remote.control_message = None;
        self.remote.streaming = true;
        self.remote.viewer_expanded = false;
        self.remote.stream_generation = self.remote.stream_generation.wrapping_add(1);
        let generation = self.remote.stream_generation;
        let (cancel_sender, cancel_receiver) = async_channel::bounded(1);
        self.remote.stream_cancel = Some(cancel_sender);
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
                        view.update(cx, |this, cx| {
                            if this.remote.stream_generation == generation {
                                this.remote.streaming = false;
                                this.remote.stream_error = Some(error);
                                this.remote.stream_cancel = None;
                                cx.notify();
                            }
                        });
                    });
                    return;
                }
            };

            loop {
                let event = match futures::future::select(
                    Box::pin(cancel_receiver.recv()),
                    Box::pin(session.recv()),
                )
                .await
                {
                    futures::future::Either::Left(_) => break,
                    futures::future::Either::Right((Some(event), _)) => event,
                    futures::future::Either::Right((None, _)) => break,
                };
                let keep_streaming = cx
                    .update(|cx| {
                        let active = {
                            let this = view.read(cx);
                            this.active_section == super::model::SoloSection::Remote
                                && this.remote.streaming
                                && this.remote.stream_generation == generation
                                && this.remote.selected_window == Some(window_id)
                        };
                        if active {
                            view.update(cx, |this, cx| {
                                match event {
                                    crate::infrastructure::desktop_monitor::DesktopStreamEvent::Frame(frame) => {
                                        let size = (frame.width, frame.height);
                                        if let Some(image) = render_desktop_frame(frame) {
                                            this.remote.stream_frame = Some(image);
                                            this.remote.stream_frame_size = Some(size);
                                            this.remote.stream_error = None;
                                        }
                                    }
                                    crate::infrastructure::desktop_monitor::DesktopStreamEvent::Closed => {
                                        this.remote.streaming = false;
                                        this.remote.stream_error =
                                            Some("The source window was closed.".to_string());
                                    }
                                    crate::infrastructure::desktop_monitor::DesktopStreamEvent::Error(error) => {
                                        this.remote.streaming = false;
                                        this.remote.stream_error = Some(error);
                                    }
                                }
                                cx.notify();
                            });
                        }
                        active
                    })
                    .unwrap_or(false);
                if !keep_streaming {
                    break;
                }
            }
            let _ = cx.update(|cx| {
                view.update(cx, |this, _| {
                    if this.remote.stream_generation == generation {
                        this.remote.stream_cancel = None;
                    }
                });
            });
        })
        .detach();
    }

    pub(super) fn stop_remote_stream(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.remote.stream_cancel.take() {
            let _ = cancel.try_send(());
        }
        self.remote.streaming = false;
        self.remote.stream_generation = self.remote.stream_generation.wrapping_add(1);
        self.remote.selected_window = None;
        self.remote.stream_frame = None;
        self.remote.stream_frame_size = None;
        self.remote.stream_error = None;
        self.remote.viewer_expanded = false;
        self.remote.control_enabled = false;
        self.remote.viewer_bounds = None;
        self.remote.control_message = None;
        cx.notify();
    }

    pub(super) fn forward_remote_click(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        if !self.remote.control_enabled {
            return;
        }
        let (Some(window_id), Some(bounds), Some((frame_width, frame_height))) = (
            self.remote.selected_window,
            self.remote.viewer_bounds,
            self.remote.stream_frame_size,
        ) else {
            return;
        };
        let bounds_width: f32 = bounds.size.width.into();
        let bounds_height: f32 = bounds.size.height.into();
        let bounds_x: f32 = bounds.origin.x.into();
        let bounds_y: f32 = bounds.origin.y.into();
        let scale = (bounds_width / frame_width as f32).min(bounds_height / frame_height as f32);
        let display_width = frame_width as f32 * scale;
        let display_height = frame_height as f32 * scale;
        let display_x = bounds_x + (bounds_width - display_width) * 0.5;
        let display_y = bounds_y + (bounds_height - display_height) * 0.5;
        let mouse_x: f32 = position.x.into();
        let mouse_y: f32 = position.y.into();
        if mouse_x < display_x
            || mouse_x > display_x + display_width
            || mouse_y < display_y
            || mouse_y > display_y + display_height
        {
            return;
        }
        let normalized_x = (mouse_x - display_x) / display_width;
        let normalized_y = (mouse_y - display_y) / display_height;
        self.remote.control_message =
            match crate::infrastructure::desktop_monitor::send_desktop_window_click(
                window_id,
                normalized_x,
                normalized_y,
            ) {
                Ok(()) => Some("Click sent to the source window".to_string()),
                Err(error) => Some(error),
            };
        cx.notify();
    }
}
