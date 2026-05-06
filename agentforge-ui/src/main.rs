use agentforge_ui::{create_main_window, init};
use gpui::{AppContext, AssetSource, SharedString};
use rust_embed::RustEmbed;
use std::borrow::Cow;

/// An asset source that loads customized local assets from the `./assets` folder.
#[derive(RustEmbed)]
#[folder = "./assets"]
pub struct LocalAssets;

pub struct CombinedAssets;

impl AssetSource for CombinedAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        // Try local assets first
        if let Some(f) = LocalAssets::get(path) {
            return Ok(Some(f.data));
        }

        // Fallback to gpui default assets
        gpui_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut combined_list: Vec<SharedString> = LocalAssets::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect();

        if let Ok(mut default_list) = gpui_component_assets::Assets.list(path) {
            combined_list.append(&mut default_list);
        }

        Ok(combined_list)
    }
}
fn main() {
    // Required this for Windows to render the WebView.
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("GPUI_DISABLE_DIRECT_COMPOSITION", "true");
    }

    // Create the application
    let app = gpui::Application::new().with_assets(CombinedAssets);

    app.run(move |cx| {
        // Initialize all AgentForgeAI systems (themes, actions, panels, menus)
        init(cx);
        
        // Initialize RoleManager and load roles
        let db = agentforge_ui::AppState::global(cx).db.clone();
        let role_manager = agentforge_ui::application::teams::role::RoleManager::new(db.clone());
        if let Err(e) = role_manager.load_roles() {
            eprintln!("Failed to load roles: {}", e);
        }

        // Activate the application (bring window to front)
        cx.activate(true);

        // Open the main AgentForgeAI window
        create_main_window(
            "AgentForgeAI",
            |window, cx| cx.new(|cx| agentforge_ui::MainWindow::new(window, cx)),
            cx,
        );
    });
}
