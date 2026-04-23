# AgentForgeAI — Foundational UI Code Guidelines

**Version**: 1.0  
**Date**: 2026-04-08  
**Framework**: gpui-component v0.5.1 / gpui v0.2.2  
**Purpose**: Provide sample code patterns and implementation references for building the AgentForgeAI desktop application UI using the GPUI component library.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Project Structure](#2-project-structure)
3. [Application Bootstrap (`main.rs`)](#3-application-bootstrap-mainrs)
4. [Application Initialization (`lib.rs`)](#4-application-initialization-librs)
5. [Title Bar Implementation](#5-title-bar-implementation)
6. [App Menu Bar Implementation](#6-app-menu-bar-implementation)
7. [Window & Root Setup](#7-window--root-setup)
8. [Key Bindings & Actions](#8-key-bindings--actions)
9. [Theme Integration](#9-theme-integration)
10. [Dock Layout & Panels](#10-dock-layout--panels)
11. [Complete Integration Example](#11-complete-integration-example)
12. [Implementation References](#12-implementation-references)

---

## 1. Architecture Overview

AgentForgeAI uses a layered architecture for its UI:

```
┌─────────────────────────────────────────────────────┐
│                    TitleBar                          │
│  ┌──────────────┐                ┌────────────────┐  │
│  │  AppMenuBar   │                │  Controls      │  │
│  │  (File/Edit/  │                │  (Settings,    │  │
│  │   Window/Help)│                │   Notifications)│  │
│  └──────────────┘                └────────────────┘  │
├─────────────────────────────────────────────────────┤
│                   Root                               │
│  ┌──────────┐ ┌───────────────────┐ ┌────────────┐  │
│  │  Icon     │ │                   │ │ Inspector  │  │
│  │  Rail     │ │  Main Workspace   │ │  Panel     │  │
│  │  (Nav)    │ │  (Dock Layout)    │ │  (Detail)  │  │
│  │           │ │                   │ │            │  │
│  └──────────┘ └───────────────────┘ └────────────┘  │
├─────────────────────────────────────────────────────┤
│              Floating Input Bar                      │
└─────────────────────────────────────────────────────┘
```

**Key principles:**
- Every window wraps content in `Root::new(view, window, cx)` for proper event handling.
- `TitleBar` is the top-level chrome, containing `AppMenuBar` on the left and controls on the right.
- The main workspace uses GPUI's dock layout system for resizable panels.
- All components use the active theme via `cx.theme()` for consistent styling.

---

## 2. Project Structure

Adapt the upstream `crates/story` structure for AgentForgeAI:

```
agentforge-ui/
├── Cargo.toml
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Init, actions, window creation, global state
│   ├── app_menus.rs         # AppMenuBar definition (File/Edit/Window/Help)
│   ├── title_bar.rs         # Custom TitleBar with controls
│   ├── themes.rs            # Custom theme registration
│   └── panels/
│       ├── mod.rs
│       ├── dashboard.rs     # Main dashboard panel
│       ├── team_workspace.rs # Agent team panel
│       ├── iflow_builder.rs # Workflow designer panel
│       ├── session.rs       # Chat/session panel
│       ├── knowledge.rs     # Knowledge base panel
│       └── monitoring.rs    # Monitoring console panel
└── assets/
    └── icons/               # Custom icon assets
```

---

## 3. Application Bootstrap (`main.rs`)

The entry point initializes the application with assets and opens the main window.

> **Reference**: [crates/story/src/main.rs](https://github.com/longbridge/gpui-component/blob/main/crates/story/src/main.rs)

```rust
use agentforge_ui::{init, create_main_window};
use gpui_component_assets::Assets;

fn main() {
    // Create the application with icon/font assets
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        // Initialize all AgentForgeAI systems (themes, actions, panels, menus)
        init(cx);

        // Activate the application (bring window to front)
        cx.activate(true);

        // Open the main AgentForgeAI window
        create_main_window(
            "AgentForgeAI",
            |window, cx| agentforge_ui::MainWindow::new(window, cx),
            cx,
        );
    });
}
```

**Key points:**
- `with_assets(Assets)` loads the gpui-component icon and font bundles.
- `init(cx)` is the single initialization call that sets up everything.
- `create_main_window` is a helper that wraps window creation with proper options.
- The main view is wrapped in `Root::new()` internally by the window creation helper.

---

## 4. Application Initialization (`lib.rs`)

The `init` function sets up the component library, global state, themes, key bindings, and panel registrations.

> **Reference**: [crates/story/src/lib.rs](https://github.com/longbridge/gpui-component/blob/main/crates/story/src/lib.rs)

```rust
use gpui::{
    Action, App, KeyBinding, SharedString, Window, WindowBounds, WindowKind,
    WindowOptions, actions, px, size,
};
use gpui_component::{
    ActiveTheme, IconName, Root, TitleBar, WindowExt,
    dock::{register_panel},
    notification::Notification,
};
use serde::{Deserialize, Serialize};

mod app_menus;
mod title_bar;
mod themes;
mod panels;

pub use crate::title_bar::AgentForgeTitleBar;

// ── Actions ──────────────────────────────────────────────

actions!(
    agentforge,
    [
        About,
        Open,
        Quit,
        ToggleSearch,
        NewTeam,
        NewAgent,
        NewWorkflow,
        ToggleDashboard,
        ToggleMonitoring,
    ]
);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = agentforge, no_json)]
pub struct SelectLocale(SharedString);

// ── Global State ─────────────────────────────────────────

pub struct AppState {
    pub active_panel: Entity<String>,
    pub notifications: Entity<Vec<NotificationEntry>>,
}

impl AppState {
    pub fn init(cx: &mut App) {
        cx.set_global::<AppState>(Self {
            active_panel: cx.new(|_| String::from("dashboard")),
            notifications: cx.new(|_| Vec::new()),
        });
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

// ── Panel Registration ───────────────────────────────────

const PANEL_DASHBOARD: &str = "Dashboard";
const PANEL_TEAM_WORKSPACE: &str = "TeamWorkspace";
const PANEL_IFLOW_BUILDER: &str = "IFlowBuilder";
const PANEL_SESSION: &str = "Session";
const PANEL_KNOWLEDGE: &str = "Knowledge";
const PANEL_MONITORING: &str = "Monitoring";

// ── Init Function ────────────────────────────────────────

pub fn init(cx: &mut App) {
    // 1. Initialize gpui-component (THE required first call)
    gpui_component::init(cx);

    // 2. Initialize global state
    AppState::init(cx);

    // 3. Register custom themes
    themes::init(cx);

    // 4. Register dock panels
    register_panel(cx, PANEL_DASHBOARD, |_, _, info, window, cx| {
        Box::new(cx.new(|cx| panels::dashboard::DashboardPanel::new(window, cx)))
    });
    register_panel(cx, PANEL_TEAM_WORKSPACE, |_, _, info, window, cx| {
        Box::new(cx.new(|cx| panels::team_workspace::TeamWorkspacePanel::new(window, cx)))
    });
    register_panel(cx, PANEL_IFLOW_BUILDER, |_, _, info, window, cx| {
        Box::new(cx.new(|cx| panels::iflow_builder::IFlowBuilderPanel::new(window, cx)))
    });
    register_panel(cx, PANEL_SESSION, |_, _, info, window, cx| {
        Box::new(cx.new(|cx| panels::session::SessionPanel::new(window, cx)))
    });
    register_panel(cx, PANEL_KNOWLEDGE, |_, _, info, window, cx| {
        Box::new(cx.new(|cx| panels::knowledge::KnowledgePanel::new(window, cx)))
    });
    register_panel(cx, PANEL_MONITORING, |_, _, info, window, cx| {
        Box::new(cx.new(|cx| panels::monitoring::MonitoringPanel::new(window, cx)))
    });

    // 5. Register key bindings
    cx.bind_keys([
        KeyBinding::new("/", ToggleSearch, None),
        KeyBinding::new("cmd-n", NewTeam, None),
        KeyBinding::new("cmd-shift-n", NewAgent, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-q", Quit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("alt-f4", Quit, None),
    ]);

    // 6. Register global action handlers
    cx.on_action(|_: &Quit, cx: &mut App| {
        cx.quit();
    });

    cx.on_action(|_: &About, cx: &mut App| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |_, window, cx| {
                        window.defer(cx, |window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert.title("About AgentForgeAI")
                                    .description("AgentForgeAI\n\nMulti-AI Orchestration Platform\n\nVersion 0.1.0")
                            });
                        });
                    })
                    .unwrap();
            });
        }
    });

    // 7. Activate the application
    cx.activate(true);
}
```

**Key points:**
- `gpui_component::init(cx)` MUST be called before any component usage.
- `register_panel` connects panel name strings to factory closures that create panel views.
- `cx.bind_keys` registers keyboard shortcuts; use `cmd-` for macOS, `ctrl-`/`alt-` for others.
- `cx.on_action` registers global action handlers (like Quit, About).
- The `About` dialog pattern uses `window.open_alert_dialog` with markdown description support.

---

## 5. Title Bar Implementation

The title bar provides the application chrome with menu integration on the left and contextual controls on the right.

> **Reference**: [crates/story/src/title_bar.rs](https://github.com/longbridge/gpui-component/blob/main/crates/story/src/title_bar.rs)

```rust
use std::rc::Rc;
use gpui::{
    AnyElement, App, Context, Corner, Entity, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement as _, Render, SharedString,
    Subscription, Window, div,
};
use gpui_component::{
    ActiveTheme as _, IconName, TitleBar, WindowExt as _,
    badge::Badge,
    button::{Button, ButtonVariants as _},
    label::Label,
    menu::{AppMenuBar, DropdownMenu as _},
};
use crate::app_menus;

/// The main title bar for AgentForgeAI.
///
/// Layout:
///   LEFT:  AppMenuBar (File / Edit / Window / Help)
///   RIGHT: Mode indicator, Settings, Notifications bell
pub struct AgentForgeTitleBar {
    app_menu_bar: Entity<AppMenuBar>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    _subscriptions: Vec<Subscription>,
}

impl AgentForgeTitleBar {
    pub fn new(
        title: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let app_menu_bar = app_menus::init(title, cx);
        Self {
            app_menu_bar,
            child: Rc::new(|_, _| div().into_any_element()),
            _subscriptions: vec![],
        }
    }

    /// Allow injection of custom right-side controls.
    /// Usage: `.child(|window, cx| div().child("custom content"))`
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
        let notifications_count = 0; // Replace with actual notification count

        TitleBar::new()
            // ── Left side: Application menu bar ──
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(self.app_menu_bar.clone())
            )
            // ── Right side: Controls ──
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    // Prevent title bar drag when clicking controls
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    // Inject custom child controls
                    .child((self.child.clone())(window, cx))
                    // Operating mode indicator
                    .child(
                        Label::new("Supervision")
                            .secondary(true)
                            .text_sm(),
                    )
                    // Settings dropdown
                    .child(
                        Button::new("settings")
                            .icon(IconName::Settings2)
                            .small()
                            .ghost()
                            .dropdown_menu(move |this, _, cx| {
                                this.scrollable(true)
                                    .check_side(gpui_component::Side::Right)
                                    .max_h(px(480.))
                                    .label("Appearance")
                                    .menu_with_check("Light", false, Box::new(SwitchLight))
                                    .menu_with_check("Dark", true, Box::new(SwitchDark))
                                    .separator()
                                    .label("Font Size")
                                    .menu_with_check("Large", false, Box::new(FontSizeLarge))
                                    .menu_with_check("Medium", true, Box::new(FontSizeMedium))
                                    .menu_with_check("Small", false, Box::new(FontSizeSmall))
                            })
                            .anchor(Corner::TopRight),
                    )
                    // Notifications bell with badge
                    .child(
                        div().relative().child(
                            Badge::new()
                                .count(notifications_count)
                                .max(99)
                                .child(
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
```

**Key patterns:**
- `Rc<dyn Fn>` for the child closure enables dynamic right-side controls from the parent.
- `cx.stop_propagation()` on `on_mouse_down` prevents the OS from initiating a window drag when the user clicks a button.
- `Badge::new().count(n).max(99)` wraps a button to show a notification count.
- `DropdownMenu` on `Button` uses the same `MenuItem` types as `AppMenuBar` for consistency.
- `Corner::TopRight` anchors the dropdown to the top-right of the button.

---

## 6. App Menu Bar Implementation

The application menu bar provides native-style menus (File, Edit, Window, Help) with actions, submenus, separators, and checked items.

> **Reference**: [crates/story/src/app_menus.rs](https://github.com/longbridge/gpui-component/blob/main/crates/story/src/app_menus.rs)

```rust
use gpui::{App, Entity, Menu, MenuItem, SharedString};
use gpui_component::{
    ActiveTheme as _, GlobalState, Theme, ThemeMode, ThemeRegistry,
    menu::AppMenuBar,
};
use crate::{
    About, Open, Quit, ToggleSearch, SelectLocale,
    NewTeam, NewAgent, NewWorkflow,
};

/// Initialize the application menu bar with all menus.
///
/// Returns an `Entity<AppMenuBar>` that can be placed inside the TitleBar.
/// Also registers the menus with the OS (for native menu bar on macOS).
pub fn init(title: impl Into<SharedString>, cx: &mut App) -> Entity<AppMenuBar> {
    let app_menu_bar = AppMenuBar::new(cx);
    let title: SharedString = title.into();

    // Build and set the initial menus
    update_app_menu(title.clone(), app_menu_bar.clone(), cx);

    // Rebuild menus when locale changes (for i18n support)
    cx.on_action({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |s: &SelectLocale, cx: &mut App| {
            rust_i18n::set_locale(&s.0.as_str());
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    });

    // Rebuild menus when theme changes (to update checked states)
    cx.observe_global::<Theme>({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |cx| {
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    })
    .detach();

    app_menu_bar
}

/// Rebuild and set all application menus.
fn update_app_menu(
    title: impl Into<SharedString>,
    app_menu_bar: Entity<AppMenuBar>,
    cx: &mut App,
) {
    let title: SharedString = title.into();

    // Set OS-level native menus (e.g., macOS menu bar)
    cx.set_menus(build_menus(title.clone(), cx));

    // Set in-window menu bar via GlobalState
    let menus: Vec<_> = build_menus(title, cx)
        .into_iter()
        .map(|menu| menu.owned())
        .collect();
    GlobalState::global_mut(cx).set_app_menus(menus);

    // Reload the in-window AppMenuBar to pick up changes
    app_menu_bar.update(cx, |menu_bar, cx| {
        menu_bar.reload(cx);
    });
}

/// Build the complete menu structure.
fn build_menus(title: impl Into<SharedString>, cx: &App) -> Vec<Menu> {
    vec![
        // ── Application menu (named after the app) ──
        Menu {
            name: title.into(),
            items: vec![
                MenuItem::action("About AgentForgeAI", About),
                MenuItem::Separator,
                MenuItem::action("Open...", Open),
                MenuItem::Separator,
                // Appearance submenu with theme toggle
                MenuItem::Submenu(Menu {
                    name: "Appearance".into(),
                    items: vec![
                        MenuItem::action("Light", SwitchThemeMode(ThemeMode::Light))
                            .checked(!cx.theme().mode.is_dark()),
                        MenuItem::action("Dark", SwitchThemeMode(ThemeMode::Dark))
                            .checked(cx.theme().mode.is_dark()),
                    ],
                    disabled: false,
                }),
                // Theme submenu (dynamically populated from ThemeRegistry)
                theme_menu(cx),
                // Language submenu (for i18n)
                language_menu(cx),
                MenuItem::Separator,
                MenuItem::action("Quit AgentForgeAI", Quit),
            ],
            disabled: false,
        },
        // ── Edit menu ──
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", gpui_component::input::Undo),
                MenuItem::action("Redo", gpui_component::input::Redo),
                MenuItem::separator(),
                MenuItem::action("Cut", gpui_component::input::Cut),
                MenuItem::action("Copy", gpui_component::input::Copy),
                MenuItem::action("Paste", gpui_component::input::Paste),
                MenuItem::separator(),
                MenuItem::action("Delete", gpui_component::input::Delete),
                MenuItem::action(
                    "Delete Previous Word",
                    gpui_component::input::DeleteToPreviousWordStart,
                ),
                MenuItem::action(
                    "Delete Next Word",
                    gpui_component::input::DeleteToNextWordEnd,
                ),
                MenuItem::separator(),
                MenuItem::action("Find", gpui_component::input::Search),
                MenuItem::separator(),
                MenuItem::action("Select All", gpui_component::input::SelectAll),
            ],
            disabled: false,
        },
        // ── AgentForgeAI-specific menus ──
        Menu {
            name: "Agent".into(),
            items: vec![
                MenuItem::action("New Team...", NewTeam),
                MenuItem::action("New Agent...", NewAgent),
                MenuItem::Separator,
                MenuItem::action("New Workflow...", NewWorkflow),
            ],
            disabled: false,
        },
        // ── Window menu ──
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Toggle Search", ToggleSearch),
            ],
            disabled: false,
        },
        // ── Help menu ──
        Menu {
            name: "Help".into(),
            items: vec![
                MenuItem::action("Documentation", Open).disabled(true),
                MenuItem::separator(),
                MenuItem::action("Open Website", Open),
            ],
            disabled: false,
        },
    ]
}

/// Build the theme submenu from the ThemeRegistry.
fn theme_menu(cx: &App) -> MenuItem {
    let themes = ThemeRegistry::global(cx).sorted_themes();
    let current_name = cx.theme().theme_name();

    MenuItem::Submenu(Menu {
        name: "Theme".into(),
        items: themes
            .iter()
            .map(|theme| {
                let checked = current_name == &theme.name;
                MenuItem::action(
                    theme.name.clone(),
                    SwitchTheme(theme.name.clone()),
                )
                .checked(checked)
            })
            .collect(),
        disabled: false,
    })
}

/// Build the language submenu.
fn language_menu(_: &App) -> MenuItem {
    let locale = rust_i18n::locale().to_string();

    MenuItem::Submenu(Menu {
        name: "Language".into(),
        items: vec![
            MenuItem::action("English", SelectLocale("en".into()))
                .checked(locale == "en"),
            MenuItem::action("简体中文", SelectLocale("zh-CN".into()))
                .checked(locale == "zh-CN"),
        ],
        disabled: false,
    })
}
```

**Key patterns:**
- `cx.set_menus()` registers menus with the OS native menu bar (critical for macOS).
- `GlobalState::global_mut(cx).set_app_menus()` registers menus for the in-window `AppMenuBar`.
- `menu_bar.reload(cx)` must be called after updating menus to refresh the in-window display.
- `cx.observe_global::<Theme>()` reactively rebuilds menus when the theme changes (updates `.checked()` states).
- Built-in input actions (`Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `Delete`, `SelectAll`, `Search`) are provided by `gpui_component::input`.
- The "Agent" menu is AgentForgeAI-specific, adding `New Team`, `New Agent`, and `New Workflow` actions.
- `MenuItem::action(label, action).checked(bool)` shows a checkmark for toggle states.
- `MenuItem::Submenu(Menu { .. })` creates nested menus (Appearance, Theme, Language).

---

## 7. Window & Root Setup

The window creation helper configures window options, creates the Root wrapper, and sets the title.

> **Reference**: [crates/story/src/lib.rs](https://github.com/longbridge/gpui-component/blob/main/crates/story/src/lib.rs) — `create_new_window_with_size`

```rust
use gpui::{Bounds, Size, WindowBounds, WindowKind, WindowOptions, px, size};

/// Create the main AgentForgeAI window with proper configuration.
pub fn create_main_window<F, E>(
    title: &str,
    view_fn: F,
    cx: &mut App,
)
where
    E: Into<gpui::AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    let mut window_size = size(px(1600.0), px(1200.0));

    // Constrain to 85% of the primary display
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }

    let window_bounds = Bounds::centered(None, window_size, cx);
    let title = SharedString::from(title.to_string());

    cx.spawn(async move |cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            // Enable the custom TitleBar
            titlebar: Some(TitleBar::title_bar_options()),
            // Minimum window size
            window_min_size: Some(gpui::Size {
                width: px(480.),
                height: px(320.),
            }),
            kind: WindowKind::Normal,
            // Linux-specific: transparent background and client-side decorations
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        let window = cx
            .open_window(options, |window, cx| {
                // Create the main view
                let view = view_fn(window, cx);
                // Wrap in Root for proper event handling and focus management
                let root = cx.new(|cx| Root::new(view, window, cx));
                root
            })
            .expect("Failed to open window");

        // Activate and set title
        window.update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title(&title);
        })?;

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}
```

**Key points:**
- `TitleBar::title_bar_options()` configures the OS window to use the custom GPUI title bar instead of the native one.
- Window size is constrained to 85% of the display to ensure visibility.
- `Root::new(view, window, cx)` wraps the main view — this is required for proper event propagation and focus management.
- Linux requires `WindowBackgroundAppearance::Transparent` and `WindowDecorations::Client` for custom title bars.
- Minimum window size of 480x320 prevents the UI from becoming unusable.

---

## 8. Key Bindings & Actions

Define actions using the `actions!` macro and bind them to keyboard shortcuts.

```rust
use gpui::{actions, KeyBinding};

// Define actions in a namespace
actions!(
    agentforge,
    [
        About,           // Show about dialog
        Open,            // Open file/resource
        Quit,            // Quit application
        ToggleSearch,    // Toggle search bar
        NewTeam,         // Create new agent team
        NewAgent,        // Create new agent
        NewWorkflow,     // Create new iFlow workflow
    ]
);

// Custom actions with data
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = agentforge, no_json)]
pub struct SelectLocale(SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = agentforge, no_json)]
pub struct SwitchTheme(SharedString);

// Bind keys (call in init)
fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        // Global shortcuts
        KeyBinding::new("/", ToggleSearch, None),
        KeyBinding::new("cmd-shift-f", ToggleSearch, None),

        // File operations
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-o", Open, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-o", Open, None),

        // Agent operations
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-n", NewTeam, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-n", NewTeam, None),

        // Quit
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-q", Quit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("alt-f4", Quit, None),
    ]);
}
```

**Pattern**: Use `#[cfg(target_os = "macos")]` / `#[cfg(not(target_os = "macos"))]` to provide platform-appropriate key bindings. macOS uses `cmd-`, while Windows/Linux use `ctrl-` or `alt-`.

---

## 9. Theme Integration

Register custom themes and allow runtime switching.

```rust
use gpui_component::{Theme, ThemeMode, ThemeRegistry};

pub fn init(cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);

    // Register the default AgentForgeAI dark theme
    registry.register(Theme {
        name: "AgentForge Dark".into(),
        mode: ThemeMode::Dark,
        ..Default::default()
    });

    // Register a custom light theme
    registry.register(Theme {
        name: "AgentForge Light".into(),
        mode: ThemeMode::Light,
        ..Default::default()
    });

    // Set the default theme
    registry.set_theme("AgentForge Dark");
}
```

**Theme access in components:**
```rust
impl Render for MyComponent {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let bg = theme.background;
        let foreground = theme.foreground;
        let accent = theme.accent;

        div()
            .bg(bg)
            .text_color(foreground)
            .child("Themed content")
    }
}
```

---

## 10. Dock Layout & Panels

Use the dock system for resizable, panel-based layouts.

```rust
use gpui_component::dock::{Panel, PanelControl, PanelInfo, PanelState, TitleStyle};

/// Example: Dashboard panel implementing the Panel trait.
pub struct DashboardPanel {
    focus_handle: gpui::FocusHandle,
}

impl DashboardPanel {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Panel for DashboardPanel {
    fn panel_name(&self) -> &'static str {
        "Dashboard"
    }

    fn title_style(&self, _: &Window, cx: &App) -> Option<TitleStyle> {
        Some(TitleStyle {
            background: Some(cx.theme().title_bar_background),
            ..Default::default()
        })
    }
}

impl gpui::Focusable for DashboardPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DashboardPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child("Dashboard content here")
    }
}
```

---

## 11. Complete Integration Example

Putting it all together — the main window view that composes the title bar and dock layout:

```rust
use gpui::{Focusable, InteractiveElement, ParentElement, Render, Styled, Window, div};
use gpui_component::{
    ActiveTheme as _, Root, TitleBar, WindowExt as _,
    dock::{Dock, DockItem, DockPlacement, PanelState},
    h_flex, v_flex,
};

pub struct MainWindow {
    focus_handle: gpui::FocusHandle,
    title_bar: Entity<AgentForgeTitleBar>,
}

impl MainWindow {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        let title_bar = cx.new(|cx| {
            AgentForgeTitleBar::new("AgentForgeAI", window, cx)
                .child(|window, cx| {
                    // Additional right-side controls can be injected here
                    div().into_any_element()
                })
        });

        Self {
            focus_handle: cx.focus_handle(),
            title_bar,
        }
    }
}

impl Focusable for MainWindow {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(self.title_bar.clone())
            .child(
                div()
                    .flex_1()
                    .child("Main workspace content (dock layout goes here)")
            )
    }
}
```

---

## 12. Implementation References

All code patterns in this document are derived from and adapted to AgentForgeAI from the following upstream source files:

| Component | Source File | Description |
|-----------|-----------|-------------|
| **Application Entry** | [main.rs](https://github.com/longbridge/gpui-component/blob/main/crates/story/src/main.rs) | App bootstrap, asset loading, window creation |
| **Initialization & State** | [lib.rs](https://github.com/longbridge/gpui-component/blob/main/crates/story/src/lib.rs) | `init()`, global state, actions, key bindings, panel registration, window options |
| **Title Bar** | [title_bar.rs](https://github.com/longbridge/gpui-component/blob/main/crates/story/src/title_bar.rs) | `TitleBar` builder, `AppTitleBar` struct, badge/button controls, dropdown menus |
| **App Menu Bar** | [app_menus.rs](https://github.com/longbridge/gpui-component/blob/main/crates/story/src/app_menus.rs) | `AppMenuBar` init, `Menu`/`MenuItem` construction, theme/language submenus, reactive updates |

**Additional upstream references:**

- [gpui-component README](https://github.com/longbridge/gpui-component) — Overview, installation, component list
- [GPUI Component Docs](https://longbridge.github.io/gpui-component) — Official documentation site
- [gpui-component crate](https://crates.io/crates/gpui-component) — Published crate on crates.io
