use gpui::{
    div, App, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString, Window,
};

pub trait AccessibleExt: IntoElement + Sized {
    fn aria_label(self, label: impl Into<String>) -> AccessibleElement {
        AccessibleElement::new(self).label(label)
    }
}

impl<T: IntoElement + Sized> AccessibleExt for T {}

pub struct AccessibleElement {
    child: gpui::AnyElement,
    label: Option<String>,
}

impl AccessibleElement {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
            label: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl RenderOnce for AccessibleElement {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        if let Some(label) = self.label {
            div()
                .id(SharedString::from(label))
                .child(self.child)
                .into_any_element()
        } else {
            div().child(self.child).into_any_element()
        }
    }
}
