use gpui::{Action, App, KeyBinding};

pub struct ShortcutManager;

impl ShortcutManager {
    pub fn register<A: Action>(cx: &mut App, keystroke: &str, action: A) {
        cx.bind_keys([KeyBinding::new(keystroke, action, None)]);
    }

    pub fn register_global(_cx: &mut App) {
        // Register default global shortcuts
        // e.g., cx.bind_keys(...)
    }
}
