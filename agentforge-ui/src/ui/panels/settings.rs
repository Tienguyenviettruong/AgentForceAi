use crate::ui::panels::custom_provider::CustomProviderSection;
use crate::AppState;
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, Render, SharedString, Window,
};
use gpui_component::{
    dock::{Panel, PanelEvent, TitleStyle},
    group_box::GroupBoxVariant,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings as GpuiSettings},
    theme::{ActiveTheme, ThemeMode},
    Sizable, Size,
};

pub struct SettingsPanel {
    focus_handle: gpui::FocusHandle,
    custom_provider: Entity<CustomProviderSection>,
    vault_path: SharedString,
}

impl SettingsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let custom_provider = cx.new(|cx| CustomProviderSection::new(window, cx));
        
        let db = AppState::global(cx).db.clone();
        let vault_path = db.get_setting("obsidian_vault_path")
            .unwrap_or_default()
            .unwrap_or_else(|| "~/Documents/Obsidian/AgentForge".to_string())
            .into();

        Self {
            focus_handle: cx.focus_handle(),
            custom_provider,
            vault_path,
        }
    }
}

impl Panel for SettingsPanel {
    fn panel_name(&self) -> &'static str {
        "Settings & Configuration"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for SettingsPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let custom_provider = self.custom_provider.clone();

        GpuiSettings::new("agentforge-settings")
            .with_size(Size::Medium)
            .with_group_variant(GroupBoxVariant::Outline)
            .pages(vec![
                SettingPage::new("Provider Configuration")
                    .default_open(true)
                    .groups(vec![
                        SettingGroup::new()
                            .title("AI Providers")
                            .items(vec![
                                SettingItem::render(move |_options, _window, _cx| {
                                    custom_provider.clone().into_any_element()
                                })
                            ])
                    ]),

                SettingPage::new("Security Policy")
                    .groups(vec![
                        SettingGroup::new()
                            .title("Role-Based Access Control (RBAC)")
                            .items(vec![
                                SettingItem::new(
                                    "Require Approval for Destructive Actions",
                                    SettingField::switch(
                                        |_cx: &App| true,
                                        |_val: bool, _cx: &mut App| {},
                                    )
                                )
                                .description("When enabled, agents must request human approval before executing file deletions or database drops."),

                                SettingItem::new(
                                    "Data Masking",
                                    SettingField::switch(
                                        |_cx: &App| true,
                                        |_val: bool, _cx: &mut App| {},
                                    )
                                )
                                .description("Automatically mask PII and secrets in agent outputs and logs."),
                            ])
                    ]),

                SettingPage::new("API Key Management")
                    .groups(vec![
                        SettingGroup::new()
                            .title("Key Rotation & Tracking")
                            .items(vec![
                                SettingItem::new(
                                    "Rotation Schedule (Days)",
                                    SettingField::number_input(
                                        gpui_component::setting::NumberFieldOptions {
                                            min: 0.0,
                                            max: 365.0,
                                            ..Default::default()
                                        },
                                        |_cx: &App| 90.0,
                                        |_val: f64, _cx: &mut App| {},
                                    )
                                )
                                .description("Remind to rotate API keys after this many days. Set to 0 to disable."),

                                SettingItem::new(
                                    "Usage Tracking",
                                    SettingField::switch(
                                        |_cx: &App| true,
                                        |_val: bool, _cx: &mut App| {},
                                    )
                                )
                                .description("Monitor token usage per API key to detect anomalies."),
                            ])
                    ]),

                SettingPage::new("User Preferences")
                    .groups(vec![
                        SettingGroup::new()
                            .title("Appearance & UI")
                            .items(vec![
                                SettingItem::new(
                                    "Theme Mode",
                                    SettingField::dropdown(
                                        vec![
                                            ("system".into(), "System".into()),
                                            ("dark".into(), "Dark".into()),
                                            ("light".into(), "Light".into()),
                                        ],
                                        |cx: &App| {
                                            if cx.theme().mode.is_dark() {
                                                "dark".into()
                                            } else {
                                                "light".into()
                                            }
                                        },
                                        |val: SharedString, cx: &mut App| {
                                            let mode = match val.as_ref() {
                                                "dark" => ThemeMode::Dark,
                                                "light" => ThemeMode::Light,
                                                _ => ThemeMode::Light,
                                            };
                                            gpui_component::Theme::change(mode, None, cx);
                                            
                                            // Also save the mode itself to ensure it's persisted correctly
                                            let mode_str = match mode {
                                                ThemeMode::Light => "light",
                                                ThemeMode::Dark => "dark",
                                            };
                                            if let Err(e) = crate::AppState::global(cx).db.set_setting("theme_mode", mode_str) {
                                                eprintln!("Failed to save theme mode string to DB from settings: {}", e);
                                            }
                                            
                                            cx.refresh_windows();
                                        },
                                    )
                                ),
                                SettingItem::new(
                                    "Language",
                                    SettingField::dropdown(
                                        vec![
                                            ("en-US".into(), "English".into()),
                                            ("zh-CN".into(), "简体中文".into()),
                                        ],
                                        |_cx: &App| "en-US".into(),
                                        |_val: SharedString, _cx: &mut App| {},
                                    )
                                ),
                                SettingItem::new(
                                    "Notifications",
                                    SettingField::switch(
                                        |_cx: &App| true,
                                        |_val: bool, _cx: &mut App| {},
                                    )
                                )
                                .description("Enable desktop notifications for agent task completions and alerts."),
                            ])
                    ]),

                SettingPage::new("Token Budget")
                    .groups(vec![
                        SettingGroup::new()
                            .title("Resource Limits")
                            .items(vec![
                                SettingItem::new(
                                    "System Daily Budget ($)",
                                    SettingField::number_input(
                                        gpui_component::setting::NumberFieldOptions {
                                            min: 0.0,
                                            max: 10000.0,
                                            ..Default::default()
                                        },
                                        |_cx: &App| 50.0,
                                        |_val: f64, _cx: &mut App| {},
                                    )
                                ),
                                SettingItem::new(
                                    "Alert Threshold (%)",
                                    SettingField::number_input(
                                        gpui_component::setting::NumberFieldOptions {
                                            min: 50.0,
                                            max: 100.0,
                                            ..Default::default()
                                        },
                                        |_cx: &App| 80.0,
                                        |_val: f64, _cx: &mut App| {},
                                    )
                                )
                                .description("Send alert when daily token budget reaches this percentage."),
                            ])
                    ]),

                SettingPage::new("Data Retention")
                    .groups(vec![
                        SettingGroup::new()
                            .title("Storage Policies")
                            .items(vec![
                                SettingItem::new(
                                    "Conversation History (Days)",
                                    SettingField::number_input(
                                        gpui_component::setting::NumberFieldOptions {
                                            min: 1.0,
                                            max: 3650.0,
                                            ..Default::default()
                                        },
                                        |_cx: &App| 90.0,
                                        |_val: f64, _cx: &mut App| {},
                                    )
                                ),
                                SettingItem::new(
                                    "Audit Logs (Months)",
                                    SettingField::number_input(
                                        gpui_component::setting::NumberFieldOptions {
                                            min: 1.0,
                                            max: 120.0,
                                            ..Default::default()
                                        },
                                        |_cx: &App| 12.0,
                                        |_val: f64, _cx: &mut App| {},
                                    )
                                )
                                .description("Minimum retention period for immutable audit logs."),
                            ])
                    ]),

                SettingPage::new("Vault Configuration")
                    .groups(vec![
                        SettingGroup::new()
                            .title("Obsidian Integration")
                            .items(vec![
                                SettingItem::new(
                                    "Vault Path",
                                    SettingField::input(
                                        |cx: &App| {
                                            // Lấy giá trị hiện tại từ DB mỗi khi render
                                            let db = AppState::global(cx).db.clone();
                                            db.get_setting("obsidian_vault_path")
                                                .unwrap_or_default()
                                                .unwrap_or_else(|| "~/Documents/Obsidian/AgentForge".to_string())
                                                .into()
                                        },
                                        |val: SharedString, cx: &mut App| {
                                            let db = AppState::global(cx).db.clone();
                                            let _ = db.set_setting("obsidian_vault_path", val.as_ref());
                                        },
                                    )
                                )
                                .description("Absolute path to your local Obsidian vault directory."),

                                SettingItem::new(
                                    "Auto-Sync",
                                    SettingField::switch(
                                        |_cx: &App| true,
                                        |_val: bool, _cx: &mut App| {},
                                    )
                                )
                                .description("Automatically sync knowledge base entries with Obsidian vault."),

                                SettingItem::new(
                                    "Conflict Resolution",
                                    SettingField::dropdown(
                                        vec![
                                            ("agentforge".into(), "Prefer AgentForge".into()),
                                            ("obsidian".into(), "Prefer Obsidian".into()),
                                            ("ask".into(), "Ask Me".into()),
                                        ],
                                        |_cx: &App| "ask".into(),
                                        |_val: SharedString, _cx: &mut App| {},
                                    )
                                ),
                            ])
                    ]),
            ])
    }
}

impl gpui::EventEmitter<PanelEvent> for SettingsPanel {}
