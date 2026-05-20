use gpui::{div, px, App, IntoElement, ParentElement, RenderOnce, Window};

pub enum Breakpoint {
    Mobile,
    Tablet,
    Desktop,
}

impl Breakpoint {
    pub fn current(window: &Window) -> Self {
        let width = window.window_bounds().get_bounds().size.width;
        if width < px(768.0) {
            Self::Mobile
        } else if width < px(1024.0) {
            Self::Tablet
        } else {
            Self::Desktop
        }
    }
}

pub trait ResponsiveExt: IntoElement + Sized {
    fn hide_on_mobile(self, window: &Window) -> ResponsiveElement {
        ResponsiveElement::new(self).hide_on(Breakpoint::Mobile, window)
    }
}

impl<T: IntoElement + Sized> ResponsiveExt for T {}

pub struct ResponsiveElement {
    child: gpui::AnyElement,
    hidden: bool,
}

impl ResponsiveElement {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
            hidden: false,
        }
    }

    pub fn hide_on(mut self, bp: Breakpoint, window: &Window) -> Self {
        let current = Breakpoint::current(window);
        match (bp, current) {
            (Breakpoint::Mobile, Breakpoint::Mobile) => self.hidden = true,
            (Breakpoint::Tablet, Breakpoint::Tablet) => self.hidden = true,
            (Breakpoint::Desktop, Breakpoint::Desktop) => self.hidden = true,
            _ => {}
        }
        self
    }
}

impl RenderOnce for ResponsiveElement {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        if self.hidden {
            div()
        } else {
            div().child(self.child)
        }
    }
}
