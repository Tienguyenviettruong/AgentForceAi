use crate::{
    About, NewAgent, NewTeam, NewWorkflow, Open, Quit, SwitchTheme, SwitchThemeMode, ToggleSearch,
};
use gpui::{App, Entity, Menu, MenuItem, SharedString, Window};
use gpui_component::menu::AppMenuBar;
use gpui_component::{ThemeMode, ThemeRegistry};

pub fn init(
    title: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<AppMenuBar> {
    let title: SharedString = title.into();

    cx.set_menus(build_menus(title.clone(), cx));

    AppMenuBar::new(window, cx)
}

pub fn build_theme_menu_items(cx: &App) -> Vec<MenuItem> {
    // Collect available themes from the registry
    let registry = ThemeRegistry::global(cx);
    let mut theme_items: Vec<MenuItem> = Vec::new();

    // Add "Light Mode" / "Dark Mode" toggles
    theme_items.push(MenuItem::action(
        "Light Mode",
        SwitchThemeMode(ThemeMode::Light),
    ));
    theme_items.push(MenuItem::action(
        "Dark Mode",
        SwitchThemeMode(ThemeMode::Dark),
    ));
    theme_items.push(MenuItem::separator());

    // Add individual theme entries from registry
    let mut theme_names: Vec<SharedString> = registry.themes().keys().cloned().collect();
    theme_names.sort();

    for name in theme_names {
        let action_name = name.clone();
        theme_items.push(MenuItem::action(name.to_string(), SwitchTheme(action_name)));
    }

    theme_items
}

pub fn build_app_menu_items(cx: &App) -> Vec<MenuItem> {
    vec![
        MenuItem::action("About AgentForgeAI", About),
        MenuItem::Separator,
        MenuItem::action("Open...", Open),
        MenuItem::Separator,
        MenuItem::action("Quit AgentForgeAI", Quit),
        MenuItem::Submenu(Menu {
            name: "Themes".into(),
            items: build_theme_menu_items(cx),
        }),
    ]
}

fn build_menus(_title: impl Into<SharedString>, _cx: &App) -> Vec<Menu> {
    vec![
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
        },
        Menu {
            name: "Agent".into(),
            items: vec![
                MenuItem::action("New Team...", NewTeam),
                MenuItem::action("New Agent...", NewAgent),
                MenuItem::Separator,
                MenuItem::action("New Workflow...", NewWorkflow),
            ],
        },
        // Menu {
        //     name: "Themes".into(),
        //     items: theme_items,
        // },
        Menu {
            name: "Window".into(),
            items: vec![MenuItem::action("Toggle Search", ToggleSearch)],
        },
        Menu {
            name: "Help".into(),
            items: vec![
                MenuItem::action("Documentation", Open),
                MenuItem::separator(),
                MenuItem::action("Open Website", Open),
            ],
        },
    ]
}
