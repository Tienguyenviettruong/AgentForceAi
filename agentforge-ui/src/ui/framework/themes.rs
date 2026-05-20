use crate::AppState;
use gpui::App;
use gpui_component::{Theme, ThemeRegistry};

pub fn init(cx: &mut App) {
    // Watch the ./themes directory for JSON theme files.
    // ThemeRegistry will auto-load all *.json files in that dir at startup
    // and re-apply the active theme whenever files change.
    if let Err(err) =
        ThemeRegistry::watch_dir(std::path::PathBuf::from("./themes"), cx, move |cx| {
            restore_saved_theme(cx);
        })
    {
        eprintln!("Warning: Failed to watch themes directory: {}", err);
    }

    restore_saved_theme(cx);
}

fn restore_saved_theme(cx: &mut App) {
    let saved_theme = AppState::global(cx).db.get_setting("theme").ok().flatten();
    let target_theme = saved_theme.unwrap_or_else(|| "AgentForge Dark".to_string());

    // Theme::change applies the registry default for the mode, so run it before applying
    // the saved named theme. The named theme must be last to preserve its custom colors.
    if let Ok(Some(saved_mode)) = AppState::global(cx).db.get_setting("theme_mode") {
        let mode = if saved_mode == "light" {
            gpui_component::ThemeMode::Light
        } else {
            gpui_component::ThemeMode::Dark
        };
        Theme::change(mode, None, cx);
    }

    let ts: gpui::SharedString = target_theme.clone().into();
    if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&ts).cloned() {
        Theme::global_mut(cx).apply_config(&theme_config);

        let db = &AppState::global(cx).db;
        let _ = db.set_setting("theme", target_theme.as_str());
        let mode_str = if theme_config.mode.is_dark() {
            "dark"
        } else {
            "light"
        };
        let _ = db.set_setting("theme_mode", mode_str);
    }
}
