use gpui::App;
use gpui_component::{Theme, ThemeRegistry};
use crate::AppState;

pub fn init(cx: &mut App) {
    // Watch the ./themes directory for JSON theme files.
    // ThemeRegistry will auto-load all *.json files in that dir at startup
    // and re-apply the active theme whenever files change.
    if let Err(err) =
        ThemeRegistry::watch_dir(std::path::PathBuf::from("./themes"), cx, move |cx| {
            // Re-apply current theme after reload
            let theme_name = Theme::global(cx).theme_name().clone();
            if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned()
            {
                Theme::global_mut(cx).apply_config(&theme_config);
            }
        })
    {
        eprintln!("Warning: Failed to watch themes directory: {}", err);
    }
    
    // Load theme from DB or default to Dark mode (AgentForge Dark / default dark theme)
    // The "agent" theme is our main dark theme according to themes/agent.json
    let mut target_theme = "agent".to_string();
    if let Ok(Some(saved_theme)) = AppState::global(cx).db.get_setting("theme") {
        target_theme = saved_theme;
    } else {
        // Save the default theme to DB if not present
        let _ = AppState::global(cx).db.set_setting("theme", "agent");
    }

    let ts: gpui::SharedString = target_theme.into();
    if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&ts).cloned() {
        Theme::global_mut(cx).apply_config(&theme_config);
    }
    
    // Also restore theme mode if saved
    if let Ok(Some(saved_mode)) = AppState::global(cx).db.get_setting("theme_mode") {
        let mode = if saved_mode == "light" {
            gpui_component::ThemeMode::Light
        } else {
            gpui_component::ThemeMode::Dark
        };
        Theme::change(mode, None, cx);
    }
}
