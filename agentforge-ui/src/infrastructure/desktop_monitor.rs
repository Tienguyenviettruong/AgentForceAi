use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct DesktopWindowPreview {
    pub window_id: isize,
    pub title: String,
    pub application: String,
    pub minimized: bool,
    pub thumbnail_path: Option<PathBuf>,
}

#[derive(Clone)]
pub enum DesktopStreamEvent {
    Frame {
        image: std::sync::Arc<gpui::RenderImage>,
        width: u32,
        height: u32,
    },
    Closed,
    Error(String),
}

pub struct DesktopCaptureStream {
    latest: std::sync::Arc<std::sync::Mutex<Option<DesktopStreamEvent>>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl DesktopCaptureStream {
    pub fn take_latest(&self) -> Option<DesktopStreamEvent> {
        self.latest.lock().ok()?.take()
    }
}

impl Drop for DesktopCaptureStream {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::{DesktopCaptureStream, DesktopStreamEvent, DesktopWindowPreview};
    use image::RgbaImage;
    use std::ffi::OsStr;
    use std::mem::{size_of, zeroed};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
    use windows_capture::frame::Frame;
    use windows_capture::graphics_capture_api::InternalCaptureControl;
    use windows_capture::settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    };
    use windows_capture::window::Window;
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
        GetWindowDC, ReleaseDC, ScreenToClient, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DIB_RGB_COLORS, SRCCOPY,
    };
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsIconic, IsWindow, IsWindowVisible, PostMessageW, PW_RENDERFULLCONTENT, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MOUSEMOVE,
    };

    struct EnumContext {
        output: Vec<DesktopWindowPreview>,
        preview_dir: PathBuf,
        refresh_id: u128,
        current_pid: u32,
    }

    pub fn list_windows() -> Vec<DesktopWindowPreview> {
        let preview_dir = std::env::temp_dir().join("agentforge-remote-previews");
        let _ = std::fs::create_dir_all(&preview_dir);
        let refresh_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let mut context = EnumContext {
            output: Vec::new(),
            preview_dir,
            refresh_id,
            current_pid: std::process::id(),
        };
        unsafe {
            EnumWindows(
                Some(enum_window),
                &mut context as *mut EnumContext as LPARAM,
            );
        }
        context.output.sort_by(|left, right| {
            left.application
                .cmp(&right.application)
                .then(left.title.cmp(&right.title))
        });
        context.output
    }

    unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let context = &mut *(lparam as *mut EnumContext);
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let title_length = GetWindowTextLengthW(hwnd);
        if title_length <= 0 {
            return 1;
        }
        let mut title_buffer = vec![0u16; title_length as usize + 1];
        let copied = GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32);
        if copied <= 0 {
            return 1;
        }
        let title = String::from_utf16_lossy(&title_buffer[..copied as usize]);
        if title.trim().is_empty() {
            return 1;
        }

        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == context.current_pid {
            return 1;
        }
        let application = process_name(process_id).unwrap_or_else(|| "Application".to_string());
        let minimized = IsIconic(hwnd) != 0;
        let thumbnail_path = if minimized {
            None
        } else {
            let file_name = format!("{}-{}.png", hwnd as usize, context.refresh_id);
            let path = context.preview_dir.join(file_name);
            capture_window_image(hwnd, 480, 270)
                .and_then(|image| image.save(&path).ok())
                .map(|_| path)
        };
        context.output.push(DesktopWindowPreview {
            window_id: hwnd as isize,
            title,
            application,
            minimized,
            thumbnail_path,
        });
        1
    }

    unsafe fn process_name(process_id: u32) -> Option<String> {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if handle.is_null() {
            return None;
        }
        let mut buffer = vec![0u16; 1024];
        let mut length = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            buffer.as_mut_ptr(),
            &mut length,
        );
        CloseHandle(handle);
        if ok == 0 || length == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        Path::new(&path)
            .file_name()
            .and_then(OsStr::to_str)
            .map(ToOwned::to_owned)
    }

    pub fn capture_window_frame(window_id: isize) -> Option<Arc<gpui::RenderImage>> {
        let hwnd = window_id as HWND;
        unsafe {
            if IsWindow(hwnd) == 0 || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                return None;
            }
        }
        let mut image = unsafe { capture_window_image(hwnd, 1440, 900) }?;
        if !frame_has_visual_content(&image) {
            return None;
        }
        for pixel in image.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        Some(Arc::new(gpui::RenderImage::new(vec![image::Frame::new(
            image,
        )])))
    }

    struct LiveWindowCapture {
        latest: Arc<Mutex<Option<DesktopStreamEvent>>>,
        last_frame_at: Option<std::time::Instant>,
    }

    impl GraphicsCaptureApiHandler for LiveWindowCapture {
        type Flags = Arc<Mutex<Option<DesktopStreamEvent>>>;
        type Error = String;

        fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
            Ok(Self {
                latest: ctx.flags,
                last_frame_at: None,
            })
        }

        fn on_frame_arrived(
            &mut self,
            frame: &mut Frame,
            capture_control: InternalCaptureControl,
        ) -> Result<(), Self::Error> {
            let now = std::time::Instant::now();
            if self.last_frame_at.is_some_and(|last_frame_at| {
                now.duration_since(last_frame_at) < Duration::from_millis(100)
            }) {
                return Ok(());
            }
            self.last_frame_at = Some(now);

            let mut buffer = match frame.buffer() {
                Ok(buffer) => buffer,
                Err(error) => {
                    if let Ok(mut latest) = self.latest.lock() {
                        *latest = Some(DesktopStreamEvent::Error(format!(
                            "Unable to read the GPU frame: {error}"
                        )));
                    }
                    capture_control.stop();
                    return Ok(());
                }
            };
            let source_width = buffer.width();
            let source_height = buffer.height();
            let row_pitch = buffer.row_pitch() as usize;
            let (width, height) = fit_stream_frame(source_width, source_height, 1_600, 900);
            let raw = buffer.as_raw_buffer();
            let Some(pixels) =
                scale_rgba_bilinear(raw, row_pitch, source_width, source_height, width, height)
            else {
                if let Ok(mut latest) = self.latest.lock() {
                    *latest = Some(DesktopStreamEvent::Error(
                        "The captured frame buffer was incomplete.".to_string(),
                    ));
                }
                capture_control.stop();
                return Ok(());
            };
            let Some(image) = RgbaImage::from_raw(width, height, pixels) else {
                if let Ok(mut latest) = self.latest.lock() {
                    *latest = Some(DesktopStreamEvent::Error(
                        "The captured frame had an invalid pixel layout.".to_string(),
                    ));
                }
                capture_control.stop();
                return Ok(());
            };
            let render_image = Arc::new(gpui::RenderImage::new(vec![image::Frame::new(image)]));
            if let Ok(mut latest) = self.latest.lock() {
                *latest = Some(DesktopStreamEvent::Frame {
                    image: render_image,
                    width,
                    height,
                });
            }
            Ok(())
        }

        fn on_closed(&mut self) -> Result<(), Self::Error> {
            if let Ok(mut latest) = self.latest.lock() {
                *latest = Some(DesktopStreamEvent::Closed);
            }
            Ok(())
        }
    }

    fn fit_stream_frame(
        source_width: u32,
        source_height: u32,
        max_width: u32,
        max_height: u32,
    ) -> (u32, u32) {
        if source_width <= max_width && source_height <= max_height {
            return (source_width, source_height);
        }
        let width_scale = max_width as f64 / source_width as f64;
        let height_scale = max_height as f64 / source_height as f64;
        let scale = width_scale.min(height_scale);
        (
            (source_width as f64 * scale).round().max(1.0) as u32,
            (source_height as f64 * scale).round().max(1.0) as u32,
        )
    }

    fn scale_rgba_bilinear(
        source: &[u8],
        row_pitch: usize,
        source_width: u32,
        source_height: u32,
        target_width: u32,
        target_height: u32,
    ) -> Option<Vec<u8>> {
        if source_width == 0 || source_height == 0 || target_width == 0 || target_height == 0 {
            return None;
        }
        let required_source_len =
            (source_height as usize - 1) * row_pitch + source_width as usize * 4;
        if required_source_len > source.len() {
            return None;
        }

        let mut target = vec![0u8; target_width as usize * target_height as usize * 4];
        let x_scale = source_width as f32 / target_width as f32;
        let y_scale = source_height as f32 / target_height as f32;
        for target_y in 0..target_height {
            let source_y = ((target_y as f32 + 0.5) * y_scale - 0.5)
                .clamp(0.0, source_height.saturating_sub(1) as f32);
            let y0 = source_y.floor() as usize;
            let y1 = (y0 + 1).min(source_height as usize - 1);
            let y_weight = source_y - y0 as f32;

            for target_x in 0..target_width {
                let source_x = ((target_x as f32 + 0.5) * x_scale - 0.5)
                    .clamp(0.0, source_width.saturating_sub(1) as f32);
                let x0 = source_x.floor() as usize;
                let x1 = (x0 + 1).min(source_width as usize - 1);
                let x_weight = source_x - x0 as f32;
                let offsets = [
                    y0 * row_pitch + x0 * 4,
                    y0 * row_pitch + x1 * 4,
                    y1 * row_pitch + x0 * 4,
                    y1 * row_pitch + x1 * 4,
                ];
                let target_offset =
                    (target_y as usize * target_width as usize + target_x as usize) * 4;
                for channel in 0..4 {
                    let top = source[offsets[0] + channel] as f32 * (1.0 - x_weight)
                        + source[offsets[1] + channel] as f32 * x_weight;
                    let bottom = source[offsets[2] + channel] as f32 * (1.0 - x_weight)
                        + source[offsets[3] + channel] as f32 * x_weight;
                    target[target_offset + channel] =
                        (top * (1.0 - y_weight) + bottom * y_weight).round() as u8;
                }
            }
        }
        Some(target)
    }

    pub fn start_window_stream(window_id: isize) -> Result<DesktopCaptureStream, String> {
        let hwnd = window_id as HWND;
        unsafe {
            if IsWindow(hwnd) == 0 {
                return Err("This window no longer exists.".to_string());
            }
        }

        let latest = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let window = Window::from_raw_hwnd(hwnd.cast());
        let settings = Settings::new(
            window,
            CursorCaptureSettings::WithoutCursor,
            DrawBorderSettings::Default,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            ColorFormat::Rgba8,
            latest.clone(),
        );
        let control = LiveWindowCapture::start_free_threaded(settings)
            .map_err(|error| format!("Windows Graphics Capture could not start: {error}"))?;
        let watcher_stop = stop.clone();
        let watcher_latest = latest.clone();
        std::thread::spawn(move || {
            while !watcher_stop.load(Ordering::Acquire) && !control.is_finished() {
                std::thread::sleep(Duration::from_millis(20));
            }
            let stopped_by_viewer = watcher_stop.load(Ordering::Acquire);
            let result = if control.is_finished() {
                control.wait()
            } else {
                control.stop()
            };
            if !stopped_by_viewer {
                if let Err(error) = result {
                    if let Ok(mut latest) = watcher_latest.lock() {
                        *latest = Some(DesktopStreamEvent::Error(format!(
                            "The live capture session ended: {error}"
                        )));
                    }
                }
            }
        });

        Ok(DesktopCaptureStream { latest, stop })
    }

    pub fn send_window_click(
        window_id: isize,
        normalized_x: f32,
        normalized_y: f32,
    ) -> Result<(), String> {
        let hwnd = window_id as HWND;
        unsafe {
            if IsWindow(hwnd) == 0 {
                return Err("The controlled window no longer exists.".to_string());
            }
            let mut rect: RECT = zeroed();
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return Err("Unable to read the controlled window bounds.".to_string());
            }
            let width = (rect.right - rect.left).max(1);
            let height = (rect.bottom - rect.top).max(1);
            let mut point = POINT {
                x: rect.left + (normalized_x.clamp(0.0, 1.0) * width as f32) as i32,
                y: rect.top + (normalized_y.clamp(0.0, 1.0) * height as f32) as i32,
            };
            if ScreenToClient(hwnd, &mut point) == 0 {
                return Err("Unable to map the viewer position to the source window.".to_string());
            }
            let position = ((point.y as u32 & 0xffff) << 16 | (point.x as u32 & 0xffff)) as LPARAM;
            if PostMessageW(hwnd, WM_MOUSEMOVE, 0, position) == 0
                || PostMessageW(hwnd, WM_LBUTTONDOWN, 1, position) == 0
                || PostMessageW(hwnd, WM_LBUTTONUP, 0, position) == 0
            {
                return Err("Windows rejected the remote click.".to_string());
            }
        }
        Ok(())
    }

    fn frame_has_visual_content(image: &RgbaImage) -> bool {
        let sample_step = ((image.width() as usize * image.height() as usize) / 4096).max(1);
        let mut minimum = u8::MAX;
        let mut maximum = u8::MIN;
        for pixel in image.pixels().step_by(sample_step) {
            let luminance =
                ((pixel[0] as u16 * 54 + pixel[1] as u16 * 183 + pixel[2] as u16 * 19) >> 8) as u8;
            minimum = minimum.min(luminance);
            maximum = maximum.max(luminance);
        }
        maximum.saturating_sub(minimum) >= 8
    }

    unsafe fn capture_window_image(
        hwnd: HWND,
        max_width: u32,
        max_height: u32,
    ) -> Option<RgbaImage> {
        let mut rect: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return None;
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width < 80 || height < 60 || width > 8_192 || height > 8_192 {
            return None;
        }

        let source_dc = GetWindowDC(hwnd);
        if source_dc.is_null() {
            return None;
        }
        let memory_dc = CreateCompatibleDC(source_dc);
        let bitmap = CreateCompatibleBitmap(source_dc, width, height);
        if memory_dc.is_null() || bitmap.is_null() {
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            if !memory_dc.is_null() {
                DeleteDC(memory_dc);
            }
            ReleaseDC(hwnd, source_dc);
            return None;
        }
        let previous = SelectObject(memory_dc, bitmap);
        let mut copied = PrintWindow(hwnd, memory_dc, PW_RENDERFULLCONTENT);
        if copied == 0 {
            copied = BitBlt(memory_dc, 0, 0, width, height, source_dc, 0, 0, SRCCOPY);
        }

        let mut info: BITMAPINFO = zeroed();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..zeroed()
        };
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let rows = if copied != 0 {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height as u32,
                pixels.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };
        SelectObject(memory_dc, previous);
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
        ReleaseDC(hwnd, source_dc);
        if rows == 0 {
            return None;
        }
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        let Some(image) = RgbaImage::from_raw(width as u32, height as u32, pixels) else {
            return None;
        };
        Some(image::imageops::thumbnail(&image, max_width, max_height))
    }
}

#[cfg(target_os = "windows")]
pub fn list_desktop_windows() -> Vec<DesktopWindowPreview> {
    platform::list_windows()
}

#[cfg(target_os = "windows")]
pub fn capture_desktop_window(window_id: isize) -> Option<std::sync::Arc<gpui::RenderImage>> {
    platform::capture_window_frame(window_id)
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
pub fn capture_desktop_window(_window_id: isize) -> Option<std::sync::Arc<gpui::RenderImage>> {
    None
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
        if let Some(window) = windows
            .iter()
            .find(|window| !window.minimized && window.thumbnail_path.is_some())
        {
            if let Some(frame) = super::capture_desktop_window(window.window_id) {
                assert_eq!(frame.frame_count(), 1);
            }
        }
    }
}
