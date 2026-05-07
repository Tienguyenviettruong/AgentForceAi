use crate::app_menus;
use gpui::{
    div, AnyElement, App, AppContext, Context, Entity, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, SharedString, Styled, Subscription, Window,
};
use gpui::{img, px, ObjectFit, StyledImage};

use crate::orchestration::modes::{ModeManager, OperatingMode};
use gpui_component::{
    badge::Badge,
    button::{Button, ButtonVariants},
    menu::AppMenuBar,
    select::{Select, SelectEvent, SelectState},
    IconName, IndexPath, Sizable, TitleBar,
};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct AgentForgeTitleBar {
    app_menu_bar: Entity<AppMenuBar>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    _subscriptions: Vec<Subscription>,
    mode_select_state: Entity<SelectState<Vec<SharedString>>>,
}

impl AgentForgeTitleBar {
    pub fn new(
        title: impl Into<SharedString>,
        mode_manager: Arc<Mutex<ModeManager>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title: SharedString = title.into();
        let app_menu_bar = app_menus::init(title.clone(), window, cx);

        let initial_mode_index = match mode_manager.lock().unwrap().current_mode() {
            OperatingMode::HumanInteraction => 0,
            OperatingMode::Supervision => 1,
            OperatingMode::Autonomous => 2,
        };

        let mode_select_state = cx.new(|cx| {
            SelectState::new(
                vec![
                    "Human Interaction".into(),
                    "Supervision".into(),
                    "Autonomous".into(),
                ],
                Some(IndexPath::new(initial_mode_index)),
                window,
                cx,
            )
        });

        let mode_manager_clone = mode_manager.clone();
        cx.subscribe(
            &mode_select_state,
            move |_this, _, event: &SelectEvent<Vec<SharedString>>, _cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let new_mode = match value.as_ref() {
                    "Human Interaction" => OperatingMode::HumanInteraction,
                    "Supervision" => OperatingMode::Supervision,
                    "Autonomous" => OperatingMode::Autonomous,
                    _ => return,
                };

                let mut manager = mode_manager_clone.lock().unwrap();
                if manager.can_transition(new_mode) {
                    let _ = manager.transition_to(new_mode, "User switched mode via title bar");
                }
            },
        )
        .detach();

        Self {
            app_menu_bar,
            child: Rc::new(|_, _| div().into_any_element()),
            _subscriptions: vec![],
            mode_select_state,
        }
    }

    pub fn child<F, E>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.child = Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }
}

impl Render for AgentForgeTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notifications_count = 0;
        TitleBar::new()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    // .pl(px(12.))
                    .child(
                        img("icons/logo.svg")
                            .size(px(32.))
                            .object_fit(ObjectFit::Contain),
                    )
                    .child(self.app_menu_bar.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child((self.child.clone())(window, cx))
                    .child(Select::new(&self.mode_select_state).small())
                    .child(
                        Button::new("settings")
                            .icon(IconName::Settings2)
                            .small()
                            .ghost(),
                    )
                    .child(
                        div().child(
                            Badge::new().count(notifications_count).max(99).child(
                                Button::new("bell")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Bell),
                            ),
                        ),
                    ),
            )
    }
}
