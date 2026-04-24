use gpui::App;
use gpui_component::{Theme, ThemeRegistry};

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
    
    // Set default theme to Dark mode (AgentForge Dark / default dark theme)
    // The "agent" theme is our main dark theme according to themes/agent.json
    if let Some(theme_config) = ThemeRegistry::global(cx).themes().get("agent").cloned() {
        Theme::global_mut(cx).apply_config(&theme_config);
    }
}
