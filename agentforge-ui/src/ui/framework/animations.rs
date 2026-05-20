use gpui::{div, App, IntoElement, ParentElement, RenderOnce, Window};

pub struct FadeIn {
    child: gpui::AnyElement,
}

impl FadeIn {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
        }
    }
}

impl RenderOnce for FadeIn {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().child(self.child)
    }
}

pub trait AnimationExt: IntoElement + Sized {
    fn with_fade_in(self) -> FadeIn {
        FadeIn::new(self)
    }
}

impl<T: IntoElement + Sized> AnimationExt for T {}
