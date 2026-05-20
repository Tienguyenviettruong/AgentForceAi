use crate::{AppState, NotificationEntry};
use gpui::App;

pub struct NotificationManager;

impl NotificationManager {
    pub fn notify(cx: &mut App, message: impl Into<String>) {
        let message = message.into();
        let notifications = AppState::global(cx).notifications.clone();
        notifications.update(cx, |n, cx| {
            n.push(NotificationEntry { message });
            cx.notify();
        });
    }
}
