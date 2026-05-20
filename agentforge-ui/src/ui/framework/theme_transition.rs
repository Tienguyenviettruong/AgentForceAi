use gpui::App;
use gpui_component::{Theme, ThemeMode, ThemeRegistry};

pub struct ThemeTransitionManager;

impl ThemeTransitionManager {
    pub fn apply_theme(cx: &mut App, theme_name: &str) {
        if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(theme_name).cloned() {
            Theme::global_mut(cx).apply_config(&theme_config);
            cx.refresh_windows();
        }
    }

    pub fn toggle_mode(cx: &mut App) {
        let is_dark = cx.global::<Theme>().mode.is_dark();
        let new_mode = if is_dark {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        Theme::change(new_mode, None, cx);
        cx.refresh_windows();
    }
}
