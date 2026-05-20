use crate::app_menus;
use gpui::prelude::FluentBuilder;
use gpui::{
    anchored, deferred, div, AnyElement, App, AppContext, ClickEvent, Context, DismissEvent,
    Entity, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
};
use gpui::{img, px, ObjectFit, StyledImage};

use crate::orchestration::modes::{ModeManager, OperatingMode};
use gpui_component::{
    badge::Badge,
    button::{Button, ButtonVariants},
    menu::{AppMenuBar, PopupMenu},
    select::{Select, SelectEvent, SelectState},
    ActiveTheme as _, IconName, IndexPath, Sizable, TitleBar,
};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct AgentForgeTitleBar {
    app_menu_bar: Entity<AppMenuBar>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    _subscriptions: Vec<Subscription>,
    mode_select_state: Entity<SelectState<Vec<SharedString>>>,
    logo_menu: Option<Entity<PopupMenu>>,
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
            logo_menu: None,
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

    fn build_logo_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let items = app_menus::build_app_menu_items(cx);
        let popup_menu = PopupMenu::build(window, cx, move |mut menu, window, cx| {
            if let Some(handle) = window.focused(cx) {
                menu = menu.action_context(handle);
            }

            for item in items {
                match item.owned() {
                    gpui::OwnedMenuItem::Action { name, action, .. } => {
                        menu = menu.menu(name, action);
                    }
                    gpui::OwnedMenuItem::Separator => {
                        menu = menu.separator();
                    }
                    gpui::OwnedMenuItem::Submenu(submenu) => {
                        menu = menu.submenu(submenu.name, window, cx, move |submenu_menu, _, _| {
                            submenu
                                .items
                                .clone()
                                .into_iter()
                                .fold(submenu_menu, |menu, item| match item {
                                    gpui::OwnedMenuItem::Action { name, action, .. } => {
                                        menu.menu(name, action)
                                    }
                                    gpui::OwnedMenuItem::Separator => menu.separator(),
                                    _ => menu,
                                })
                        });
                    }
                    gpui::OwnedMenuItem::SystemMenu(_) => {}
                }
            }

            menu
        });
        popup_menu.read(cx).focus_handle(cx).focus(window);
        self._subscriptions.push(cx.subscribe_in(
            &popup_menu,
            window,
            Self::handle_logo_menu_dismiss,
        ));
        self.logo_menu = Some(popup_menu.clone());
        popup_menu
    }

    fn handle_logo_menu_dismiss(
        &mut self,
        _: &Entity<PopupMenu>,
        _: &DismissEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.logo_menu.take();
        self._subscriptions.clear();
        cx.notify();
    }

    fn toggle_logo_menu(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.logo_menu.take().is_some() {
            self._subscriptions.clear();
        } else {
            self.build_logo_menu(window, cx);
        }
        cx.notify();
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
                    .child(
                        div()
                            .id("agentforge-logo-menu")
                            .relative()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(38.))
                            .h(px(28.))
                            .rounded(px(4.))
                            .hover(|style| style.bg(cx.theme().secondary))
                            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .on_click(cx.listener(Self::toggle_logo_menu))
                            .child(
                                img("icons/logo.svg")
                                    .size(px(24.))
                                    .object_fit(ObjectFit::Contain),
                            )
                            .when_some(self.logo_menu.clone(), |this, menu| {
                                this.child(deferred(
                                    anchored()
                                        .anchor(gpui::Corner::TopLeft)
                                        .snap_to_window_with_margin(px(8.))
                                        .child(div().size_full().occlude().top_1().child(menu)),
                                ))
                            }),
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
