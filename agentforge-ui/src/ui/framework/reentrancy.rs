use std::sync::atomic::{AtomicBool, Ordering};

static OFFICE_WEBVIEW_INIT_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

pub fn set_office_webview_init_in_progress(value: bool) {
    OFFICE_WEBVIEW_INIT_IN_PROGRESS.store(value, Ordering::SeqCst);
}

pub fn office_webview_init_in_progress() -> bool {
    OFFICE_WEBVIEW_INIT_IN_PROGRESS.load(Ordering::SeqCst)
}

