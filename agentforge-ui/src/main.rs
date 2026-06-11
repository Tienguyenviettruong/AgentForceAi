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

fn run_app() {
    // Keep Windows composition behavior stable for GPUI rendering.
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("GPUI_DISABLE_DIRECT_COMPOSITION", "true");
    }

    let app = gpui::Application::new().with_assets(CombinedAssets);

    app.run(move |cx| {
        init(cx);

        let db = agentforge_ui::AppState::global(cx).db.clone();
        let role_manager = agentforge_ui::application::teams::role::RoleManager::new(db.clone());
        if let Err(e) = role_manager.load_roles() {
            eprintln!("Failed to load roles: {}", e);
        }

        cx.activate(true);

        create_main_window(
            "AgentForgeAI",
            |window, cx| cx.new(|cx| agentforge_ui::MainWindow::new(window, cx)),
            cx,
        );
    });
}

fn main() {
    // Spawn the entire GPUI app on a thread with a large stack (64 MB).
    //
    // In debug mode on Windows the default main-thread stack is ~1 MB.
    // GPUI's render pipeline + deeply nested render_chat_column / render_markdown
    // functions easily exceed that limit when a session with many messages is
    // selected, producing STATUS_STACK_BUFFER_OVERRUN (exit code 0xc000041d).
    //
    // 64 MB is well above the worst-case depth and has negligible memory cost
    // (virtual address space is committed lazily by the OS).
    let handle = std::thread::Builder::new()
        .name("agentforge-main".to_string())
        .stack_size(64 * 1024 * 1024) // 64 MB
        .spawn(run_app)
        .expect("Failed to spawn main app thread");

    handle.join().expect("Main app thread panicked");
}
