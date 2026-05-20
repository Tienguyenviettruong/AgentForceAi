use gpui::{App, Window};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use gpui_component::webview::WebView;

pub struct OfficeView {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub webview: Option<WebView>,
}

impl OfficeView {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        let mut webview = None;

        let builder = wry::WebViewBuilder::new();
        let html_content = include_str!("../../../assets/office/index.html");
        if let Ok(view) = builder.with_html(html_content).build_as_child(window) {
            webview = Some(WebView::new(view, window, cx));
        }

        Self { webview }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub fn new(_window: &mut Window, _cx: &mut App) -> Self {
        Self {}
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update_agents(&self, json_data: &str) {
        if let Some(webview) = &self.webview {
            let script = format!("window.updateAgents && window.updateAgents({});", json_data);
            let _ = webview.evaluate_script(&script);
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub fn update_agents(&self, _json_data: &str) {
        // No-op for Linux
    }
}
