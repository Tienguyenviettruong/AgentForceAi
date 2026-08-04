use super::{DesktopCaptureStream, DesktopFrame, DesktopStreamEvent, DesktopWindowPreview};
use image::{imageops::FilterType, ImageBuffer, Rgba, RgbaImage};
use std::ffi::OsStr;
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
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
    let ok =
        QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut length);
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

#[derive(Clone)]
struct StreamEvents {
    sender: async_channel::Sender<DesktopStreamEvent>,
    receiver: async_channel::Receiver<DesktopStreamEvent>,
}

impl StreamEvents {
    fn publish_latest(&self, event: DesktopStreamEvent) {
        while self.receiver.try_recv().is_ok() {}
        let _ = self.sender.try_send(event);
    }
}

struct LiveWindowCapture {
    events: StreamEvents,
    last_frame_at: Option<std::time::Instant>,
    staging: Vec<u8>,
}

impl GraphicsCaptureApiHandler for LiveWindowCapture {
    type Flags = StreamEvents;
    type Error = String;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            events: ctx.flags,
            last_frame_at: None,
            staging: Vec::new(),
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

        let buffer = match frame.buffer() {
            Ok(buffer) => buffer,
            Err(error) => {
                self.events
                    .publish_latest(DesktopStreamEvent::Error(format!(
                        "Unable to read the GPU frame: {error}"
                    )));
                capture_control.stop();
                return Ok(());
            }
        };
        let source_width = buffer.width();
        let source_height = buffer.height();
        let (width, height) = fit_stream_frame(source_width, source_height, 1_600, 900);
        let source_pixels = buffer.as_nopadding_buffer(&mut self.staging);
        let Some(source_image) =
            ImageBuffer::<Rgba<u8>, &[u8]>::from_raw(source_width, source_height, source_pixels)
        else {
            self.events.publish_latest(DesktopStreamEvent::Error(
                "The captured frame buffer was incomplete.".to_string(),
            ));
            capture_control.stop();
            return Ok(());
        };
        let pixels = if width == source_width && height == source_height {
            source_pixels.to_vec()
        } else {
            image::imageops::resize(&source_image, width, height, FilterType::Triangle).into_raw()
        };
        self.events
            .publish_latest(DesktopStreamEvent::Frame(DesktopFrame {
                pixels,
                width,
                height,
            }));
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        self.events.publish_latest(DesktopStreamEvent::Closed);
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

pub fn start_window_stream(window_id: isize) -> Result<DesktopCaptureStream, String> {
    let hwnd = window_id as HWND;
    unsafe {
        if IsWindow(hwnd) == 0 {
            return Err("This window no longer exists.".to_string());
        }
    }

    let (sender, receiver) = async_channel::bounded(1);
    let events = StreamEvents {
        sender,
        receiver: receiver.clone(),
    };
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
        events.clone(),
    );
    let control = LiveWindowCapture::start_free_threaded(settings)
        .map_err(|error| format!("Windows Graphics Capture could not start: {error}"))?;
    let watcher_stop = stop.clone();
    let watcher_events = events.clone();
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
                watcher_events.publish_latest(DesktopStreamEvent::Error(format!(
                    "The live capture session ended: {error}"
                )));
            }
        }
    });

    Ok(DesktopCaptureStream {
        events: receiver,
        stop,
    })
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

unsafe fn capture_window_image(hwnd: HWND, max_width: u32, max_height: u32) -> Option<RgbaImage> {
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
    let image = RgbaImage::from_raw(width as u32, height as u32, pixels)?;
    Some(image::imageops::thumbnail(&image, max_width, max_height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_frame_fit_preserves_aspect_ratio_and_limits() {
        assert_eq!(fit_stream_frame(1_920, 1_080, 1_600, 900), (1_600, 900));
        assert_eq!(fit_stream_frame(1_280, 720, 1_600, 900), (1_280, 720));
        assert_eq!(fit_stream_frame(2_560, 1_080, 1_600, 900), (1_600, 675));
    }

    #[test]
    fn stream_channel_keeps_only_the_latest_frame() {
        let (sender, receiver) = async_channel::bounded(1);
        let events = StreamEvents {
            sender,
            receiver: receiver.clone(),
        };
        events.publish_latest(DesktopStreamEvent::Frame(DesktopFrame {
            pixels: vec![1; 4],
            width: 1,
            height: 1,
        }));
        events.publish_latest(DesktopStreamEvent::Frame(DesktopFrame {
            pixels: vec![2; 4],
            width: 1,
            height: 1,
        }));

        let DesktopStreamEvent::Frame(frame) = receiver.try_recv().unwrap() else {
            panic!("expected a frame");
        };
        assert_eq!(frame.pixels, vec![2; 4]);
        assert!(receiver.try_recv().is_err());
    }
}
