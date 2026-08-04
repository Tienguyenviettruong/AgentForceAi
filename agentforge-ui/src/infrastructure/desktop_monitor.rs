use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct DesktopWindowPreview {
    pub window_id: isize,
    pub title: String,
    pub application: String,
    pub minimized: bool,
    pub thumbnail_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct DesktopFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub enum DesktopStreamEvent {
    Frame(DesktopFrame),
    Closed,
    Error(String),
}

pub struct DesktopCaptureStream {
    events: async_channel::Receiver<DesktopStreamEvent>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl DesktopCaptureStream {
    pub async fn recv(&self) -> Option<DesktopStreamEvent> {
        self.events.recv().await.ok()
    }
}

impl Drop for DesktopCaptureStream {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(target_os = "windows")]
#[path = "desktop_monitor/windows.rs"]
mod platform;

#[cfg(target_os = "windows")]
pub fn list_desktop_windows() -> Vec<DesktopWindowPreview> {
    platform::list_windows()
}

#[cfg(target_os = "windows")]
pub fn start_desktop_window_stream(window_id: isize) -> Result<DesktopCaptureStream, String> {
    platform::start_window_stream(window_id)
}

#[cfg(target_os = "windows")]
pub fn send_desktop_window_click(
    window_id: isize,
    normalized_x: f32,
    normalized_y: f32,
) -> Result<(), String> {
    platform::send_window_click(window_id, normalized_x, normalized_y)
}

#[cfg(not(target_os = "windows"))]
pub fn list_desktop_windows() -> Vec<DesktopWindowPreview> {
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
pub fn start_desktop_window_stream(_window_id: isize) -> Result<DesktopCaptureStream, String> {
    Err("Live desktop capture is currently supported on Windows only.".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn send_desktop_window_click(
    _window_id: isize,
    _normalized_x: f32,
    _normalized_y: f32,
) -> Result<(), String> {
    Err("Remote window control is currently supported on Windows only.".to_string())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    #[test]
    fn enumerated_windows_have_valid_preview_metadata() {
        let windows = super::list_desktop_windows();
        for window in &windows {
            assert!(!window.title.trim().is_empty());
            assert!(!window.application.trim().is_empty());
            if let Some(path) = &window.thumbnail_path {
                assert!(path.is_file());
            }
        }
    }
}
