use gpui::{App, Entity, Global};

use super::state::TextViewState;

#[derive(Default)]
pub(crate) struct GlobalState {
    pub(crate) text_view_state_stack: Vec<Entity<TextViewState>>,
}

impl Global for GlobalState {}

impl GlobalState {
    pub(crate) fn init(cx: &mut App) {
        cx.set_global(Self::default());
    }

    pub(crate) fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub(crate) fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub(crate) fn text_view_state(&self) -> Option<Entity<TextViewState>> {
        self.text_view_state_stack.last().cloned()
    }
}
