use crate::core::traits::database::DatabasePort;
use crate::infrastructure::database::sqlite_adapter::Database;
use gpui::{
    actions, div, px, size, Action, App, AppContext, Bounds, Context, Entity, Global,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, SharedString, Styled,
    Window, WindowBounds, WindowKind, WindowOptions,
};
use gpui_component::{
    dock::register_panel,
    dock::{DockArea, DockEvent, DockItem, PanelView},
    v_flex, Root,
};
use serde::Deserialize;
use std::sync::Arc;

pub mod application;
pub mod core;
pub mod infrastructure;
pub mod ui;

// Legacy module aliases for backward compatibility during migration
pub mod db {
    pub use crate::core::models::*;
    pub use crate::infrastructure::database::sqlite_adapter::Database;
}
pub use crate::application::agents;
pub use crate::application::cost_optimization as cost;
pub use crate::application::cross_team_router as cross_team;
pub use crate::application::doc_engine as docs;
pub use crate::application::iflow_engine as iflows;
pub use crate::application::knowledge;
pub use crate::application::marketplace_service as mcp_marketplace;
pub use crate::application::orchestration;
pub use crate::application::research;
pub use crate::application::session_manager as session;
pub use crate::application::skills;
pub use crate::application::tasks;
pub use crate::application::teams;
pub use crate::infrastructure::llm_providers as providers;
pub use crate::infrastructure::mcp;
pub use crate::infrastructure::message_bus as teambus;
pub use crate::infrastructure::monitoring;
pub use crate::infrastructure::performance;
pub use crate::infrastructure::security;
pub use crate::ui::shell::app_menus;

pub use crate::ui::shell::title_bar::AgentForgeTitleBar;

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
        ToggleMonitoring,
    ]
);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = agentforge, no_json)]
pub struct SelectLocale(pub SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = agentforge, no_json)]
pub struct SwitchTheme(pub SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = agentforge, no_json)]
pub struct SwitchThemeMode(pub gpui_component::ThemeMode);

// ── Global State ─────────────────────────────────────────

pub struct NotificationEntry {
    pub message: String,
}

pub struct AppState {
    pub active_panel: Entity<String>,
    pub notifications: Entity<Vec<NotificationEntry>>,
    pub db: Arc<dyn DatabasePort>,
    pub team_bus: std::sync::Arc<crate::infrastructure::message_bus::routing::TeamBusRouter>,
    pub tokio_runtime: std::sync::Arc<tokio::runtime::Runtime>,
    pub obsidian_watcher: std::sync::Arc<std::sync::Mutex<Option<notify::RecommendedWatcher>>>,
    pub mode_manager:
        std::sync::Arc<std::sync::Mutex<crate::application::orchestration::modes::ModeManager>>,
    pub chat_service: std::sync::Arc<crate::application::services::chat_service::ChatService>,
    pub team_service: std::sync::Arc<crate::application::services::team_service::TeamService>,
    pub knowledge_service:
        std::sync::Arc<crate::application::services::knowledge_service::KnowledgeService>,
}

impl Global for AppState {}

impl AppState {
    pub fn init(cx: &mut App) {
        let active_panel = cx.new(|_| String::from("dashboard"));
        let notifications = cx.new(|_| Vec::new());
        let db: Arc<dyn DatabasePort> =
            Arc::new(Database::new().expect("Failed to initialize database"));
        let team_bus =
            std::sync::Arc::new(crate::infrastructure::message_bus::routing::TeamBusRouter::new());
        let tokio_runtime = std::sync::Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to init tokio runtime"),
        );
        let obsidian_watcher = std::sync::Arc::new(std::sync::Mutex::new(None));
        let mode_manager = std::sync::Arc::new(std::sync::Mutex::new(
            crate::application::orchestration::modes::ModeManager::new(
                crate::application::orchestration::modes::OperatingMode::HumanInteraction,
            ),
        ));
        let chat_service = std::sync::Arc::new(
            crate::application::services::chat_service::ChatService::new(
                db.clone(),
                team_bus.clone(),
            ),
        );
        let team_service = std::sync::Arc::new(
            crate::application::services::team_service::TeamService::new(db.clone()),
        );
        let knowledge_service = std::sync::Arc::new(
            crate::application::services::knowledge_service::KnowledgeService::new(db.clone()),
        );
        cx.set_global::<AppState>(Self {
            active_panel,
            notifications,
            db,
            team_bus,
            tokio_runtime,
            obsidian_watcher,
            mode_manager,
            chat_service,
            team_service,
            knowledge_service,
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

const PANEL_TEAM_WORKSPACE: &str = "TeamWorkspace";
const PANEL_IFLOW_BUILDER: &str = "IFlowBuilder";
const PANEL_SESSION: &str = "Session";
const PANEL_KNOWLEDGE: &str = "Knowledge";
const PANEL_MONITORING: &str = "Monitoring";
const PANEL_SETTINGS: &str = "Settings";
const PANEL_MCP_MARKETPLACE: &str = "McpMarketplace";
const PANEL_RESEARCH_NOTEBOOK: &str = "ResearchNotebook";

// ── Init Function ────────────────────────────────────────

pub fn init(cx: &mut App) {
    // 1. Initialize gpui-component (THE required first call)
    gpui_component::init(cx);

    // 2. Initialize global state
    AppState::init(cx);

    // 3. Initialize themes (watch ./themes dir for JSON files)
    crate::ui::framework::themes::init(cx);

    if let Err(e) = AppState::global(cx).db.seed_sdg_team() {
        eprintln!("Failed to seed SDG team: {}", e);
    }

    let vault_path = std::env::var("AGENTFORGE_OBSIDIAN_VAULT").ok().or_else(|| {
        AppState::global(cx)
            .db
            .get_setting("obsidian_vault_path")
            .ok()
            .flatten()
    });
    if let Some(vault_path) = vault_path {
        let state = AppState::global(cx);
        if state.obsidian_watcher.lock().unwrap().is_none() {
            if let Ok(watcher) =
                crate::infrastructure::fs::obsidian_adapter::ObsidianWatcher::start_sync(
                    state.db.clone(),
                    std::path::PathBuf::from(vault_path),
                    state.tokio_runtime.clone(),
                )
            {
                *state.obsidian_watcher.lock().unwrap() = Some(watcher);
            }
        }
    }

    {
        let state = AppState::global(cx);
        crate::application::iflow_engine::automation::IFlowAutomation::start(
            state.db.clone(),
            state.team_bus.clone(),
            state.tokio_runtime.clone(),
        );

        // Start Worker Manager
        let worker_manager = std::sync::Arc::new(
            crate::application::orchestration::worker::WorkerManager::new(state.db.clone(), state.team_bus.clone())
        );
        let wm_clone = worker_manager.clone();
        state.tokio_runtime.spawn(async move {
            // Check instances periodically and start workers
            loop {
                if let Ok(instances) = wm_clone.db.list_instances() {
                    for instance in instances {
                        wm_clone.start_workers_for_instance(&instance.id).await;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
        });
    }

    // 4. Register dock panels
    // register_panel(cx, PANEL_TEAM_WORKSPACE, |_, _, _, window, cx| {
    //     Box::new(cx.new(|cx| crate::ui::panels::team_workspace::TeamWorkspacePanel::new(window, cx)))
    // });
    register_panel(cx, PANEL_IFLOW_BUILDER, |_, _, _, window, cx| {
        Box::new(cx.new(|cx| crate::ui::panels::iflow_builder::IFlowBuilderPanel::new(window, cx)))
    });
    register_panel(cx, PANEL_SESSION, |_, _, _, window, cx| {
        Box::new(cx.new(|cx| crate::ui::panels::session::SessionPanel::new(window, cx)))
    });
    register_panel(cx, PANEL_KNOWLEDGE, |_, _, _, window, cx| {
        Box::new(cx.new(|cx| crate::ui::panels::knowledge::KnowledgePanel::new(window, cx)))
    });
    register_panel(cx, PANEL_MONITORING, |_, _, _, window, cx| {
        Box::new(cx.new(|cx| crate::ui::panels::monitoring::MonitoringPanel::new(window, cx)))
    });
    register_panel(cx, PANEL_SETTINGS, |_, _, _, window, cx| {
        Box::new(cx.new(|cx| crate::ui::panels::settings::SettingsPanel::new(window, cx)))
    });
    register_panel(cx, PANEL_MCP_MARKETPLACE, |_, _, _, window, cx| {
        Box::new(
            cx.new(|cx| crate::ui::panels::mcp_marketplace::McpMarketplacePanel::new(window, cx)),
        )
    });
    register_panel(cx, PANEL_RESEARCH_NOTEBOOK, |_, _, _, window, cx| {
        Box::new(
            cx.new(|cx| {
                crate::ui::panels::research_notebook::ResearchNotebookPanel::new(window, cx)
            }),
        )
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

    cx.on_action(|_: &About, _cx: &mut App| {
        println!("About AgentForgeAI\n\nMulti-AI Orchestration Platform\n\nVersion 0.1.0");
    });

    // 7. Register Theme action handlers
    cx.on_action(|switch: &SwitchTheme, cx: &mut App| {
        let name = switch.0.clone();
        if let Some(theme_config) = gpui_component::ThemeRegistry::global(cx)
            .themes()
            .get(&name)
            .cloned()
        {
            gpui_component::Theme::global_mut(cx).apply_config(&theme_config);
            
            // Save selected theme to DB
            if let Err(e) = AppState::global(cx).db.set_setting("theme", name.as_ref()) {
                eprintln!("Failed to save theme to DB: {}", e);
            }
        }
        cx.refresh_windows();
    });

    cx.on_action(|switch: &SwitchThemeMode, cx: &mut App| {
        gpui_component::Theme::change(switch.0, None, cx);
        
        // Also save the mode itself to ensure it's persisted correctly
        let mode_str = match switch.0 {
            gpui_component::ThemeMode::Light => "light",
            gpui_component::ThemeMode::Dark => "dark",
        };
        if let Err(e) = AppState::global(cx).db.set_setting("theme_mode", mode_str) {
            eprintln!("Failed to save theme mode string to DB: {}", e);
        }
        
        cx.refresh_windows();
    });
}

// ── Main Window Helper ───────────────────────────────────

pub fn create_main_window<F, E>(title: &str, view_fn: F, cx: &mut App)
where
    E: Into<gpui::AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    let mut window_size = size(px(1600.0), px(1200.0));

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
            titlebar: None,
            window_min_size: Some(gpui::Size {
                width: px(480.),
                height: px(320.),
            }),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        let window = cx
            .open_window(options, |window, cx| {
                let view = view_fn(window, cx);

                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");

        window.update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title(&title);
        })?;

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}

// ── MainWindow Component ─────────────────────────────────

pub struct MainWindow {
    title_bar: Entity<AgentForgeTitleBar>,
    activity_bar: Entity<crate::ui::shell::activity_bar::ActivityBar>,
    status_bar: Entity<crate::ui::shell::status_bar::StatusBar>,
    active_page: SharedString,
    dock_areas: std::collections::HashMap<SharedString, Entity<DockArea>>,
    team_workspace: Entity<crate::ui::panels::team_workspace::TeamWorkspacePanel>,
}

impl MainWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let team_workspace =
            cx.new(|cx| crate::ui::panels::team_workspace::TeamWorkspacePanel::new(window, cx));
        let mode_manager = AppState::global(cx).mode_manager.clone();

        let mode_manager_clone = mode_manager.clone();
        let title_bar = cx.new(|cx| {
            AgentForgeTitleBar::new("AgentForgeAI", mode_manager_clone, window, cx)
                .child(|_, _| div().into_any_element())
        });

        let activity_bar = cx.new(|cx| crate::ui::shell::activity_bar::ActivityBar::new(cx));
        let status_bar = cx.new(|cx| crate::ui::shell::status_bar::StatusBar::new(cx));

        let mut dock_areas = std::collections::HashMap::new();

        // Create a dock area for each main page.
        // We will initialize them lazily or upfront. Let's do it up front for simplicity.

        let pages = vec![
            "dashboard",
            "skills",
            "knowledge",
            "settings",
            "monitoring",
            "profile",
            "agents",
            "mcp_marketplace",
            "iflow_builder",
            "research_notebook",
            "orchestration",
        ];
        for page in pages {
            let dock_area = cx.new(|cx| DockArea::new(page, Some(1), window, cx));
            let page_str: SharedString = page.into();

            dock_area.update(cx, |dock, cx| {
                if let Some(state) = crate::ui::shell::dock_layout::load(&page_str) {
                    let _ = dock.load(state, window, cx);
                    return;
                }

                let weak_dock = cx.entity().downgrade();
                match page {
                    "dashboard" => {
                        let panel =
                            Arc::new(cx.new(|cx| {
                                crate::ui::panels::settings::SettingsPanel::new(window, cx)
                            }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "skills" => {
                        let panel =
                            Arc::new(cx.new(|cx| {
                                crate::ui::panels::session::SessionPanel::new(window, cx)
                            }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "knowledge" => {
                        let panel = Arc::new(cx.new(|cx| {
                            crate::ui::panels::knowledge::KnowledgePanel::new(window, cx)
                        }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "settings" => {
                        let panel =
                            Arc::new(cx.new(|cx| {
                                crate::ui::panels::settings::SettingsPanel::new(window, cx)
                            }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "agents" => {
                        let panel = Arc::new(
                            cx.new(|cx| crate::ui::panels::agents::AgentsPanel::new(window, cx)),
                        );
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "monitoring" => {
                        let panel = Arc::new(cx.new(|cx| {
                            crate::ui::panels::monitoring::MonitoringPanel::new(window, cx)
                        }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "profile" => {
                        // Use dashboard for now as profile panel is not implemented yet
                        let panel =
                            Arc::new(cx.new(|cx| {
                                crate::ui::panels::settings::SettingsPanel::new(window, cx)
                            }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "mcp_marketplace" => {
                        let panel = Arc::new(cx.new(|cx| {
                            crate::ui::panels::mcp_marketplace::McpMarketplacePanel::new(window, cx)
                        }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "iflow_builder" => {
                        let panel = Arc::new(cx.new(|cx| {
                            crate::ui::panels::iflow_builder::IFlowBuilderPanel::new(window, cx)
                        }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "research_notebook" => {
                        let panel = Arc::new(cx.new(|cx| {
                            crate::ui::panels::research_notebook::ResearchNotebookPanel::new(
                                window, cx,
                            )
                        }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    "orchestration" => {
                        let panel = Arc::new(cx.new(|cx| {
                            crate::ui::panels::orchestration::OrchestrationPanel::new(window, cx)
                        }));
                        dock.set_center(
                            DockItem::tabs(vec![panel], &weak_dock, window, cx),
                            window,
                            cx,
                        );
                    }
                    _ => {}
                }
            });

            let page_name = page_str.clone();
            cx.subscribe_in(
                &dock_area,
                window,
                move |_, dock_area, event: &DockEvent, _window, cx| {
                    if matches!(event, DockEvent::LayoutChanged) {
                        let state = dock_area.read(cx).dump(cx);
                        let _ = crate::ui::shell::dock_layout::save(&page_name, &state);
                    }
                },
            )
            .detach();

            dock_areas.insert(page_str, dock_area);
        }

        cx.subscribe_in(
            &activity_bar,
            window,
            |this,
             _activity_bar,
             event: &crate::ui::shell::activity_bar::ActivityBarEvent,
             window,
             cx| match event {
                crate::ui::shell::activity_bar::ActivityBarEvent::Selected(id) => {
                    this.switch_panel(id.clone(), window, cx);
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &status_bar,
            window,
            |this,
             _status_bar,
             event: &crate::ui::shell::status_bar::StatusBarEvent,
             window,
             cx| match event {
                crate::ui::shell::status_bar::StatusBarEvent::OpenMonitoring => {
                    this.switch_panel("monitoring".into(), window, cx);
                }
                crate::ui::shell::status_bar::StatusBarEvent::OpenAgents => {
                    this.switch_panel("agents".into(), window, cx);
                }
            },
        )
        .detach();

        Self {
            title_bar,
            activity_bar,
            status_bar,
            active_page: "dashboard".into(),
            dock_areas,
            team_workspace,
        }
    }

    fn switch_panel(&mut self, id: SharedString, _window: &mut Window, cx: &mut Context<Self>) {
        if self.dock_areas.contains_key(&id) || id == "teams" {
            self.active_page = id;
            cx.notify();
        }
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);
        let sheet_layer = Root::render_sheet_layer(window, cx);

        v_flex()
            .size_full()
            .on_action(cx.listener(|this, _: &NewTeam, window, cx| {
                crate::ui::components::dialogs::open_new_team_dialog(
                    crate::AppState::global(cx).db.clone(),
                    cx.entity().clone(),
                    window,
                    cx,
                    |_, _| {},
                );
                this.switch_panel("teams".into(), window, cx);
            }))
            .on_action(cx.listener(|this, _: &NewAgent, window, cx| {
                crate::ui::components::dialogs::open_new_agent_dialog(
                    crate::AppState::global(cx).db.clone(),
                    cx.entity().clone(),
                    window,
                    cx,
                    |_, _| {},
                );
                this.switch_panel("teams".into(), window, cx);
            }))
            .child(self.title_bar.clone())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_hidden()
                    .child(self.activity_bar.clone())
                    .child(
                        div()
                            .h_full()
                            .flex_1()
                            .min_h(px(0.))
                            .overflow_hidden()
                            .child(if self.active_page == "teams" {
                                self.team_workspace.clone().into_any_element()
                            } else if let Some(dock) = self.dock_areas.get(&self.active_page) {
                                div()
                                    .size_full()
                                    .relative()
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(-30.))
                                            .left(px(0.))
                                            .right(px(0.))
                                            .bottom(px(0.))
                                            .child(dock.clone())
                                    )
                                    .into_any_element()
                            } else {
                                div().into_any_element()
                            }),
                    ),
            )
            .child(self.status_bar.clone())
            .children(dialog_layer)
            .children(notification_layer)
            .children(sheet_layer)
    }
}
