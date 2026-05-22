use crate::ui::panels::custom_provider::CustomProviderSection;
use crate::AppState;
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, Render, SharedString, Window,
};
use gpui_component::{
    dock::{Panel, PanelEvent, TitleStyle},
    group_box::GroupBoxVariant,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings as GpuiSettings},
    theme::ActiveTheme,
    Sizable, Size, ThemeRegistry,
};

fn setting_or_env(cx: &App, key: &str, env_key: &str, default: &str) -> SharedString {
    AppState::global(cx)
        .db
        .get_setting(key)
        .unwrap_or_default()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| std::env::var(env_key).ok().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| default.to_string())
        .into()
}

fn setting_only(cx: &App, key: &str) -> SharedString {
    AppState::global(cx)
        .db
        .get_setting(key)
        .unwrap_or_default()
        .unwrap_or_default()
        .into()
}

fn save_setting(cx: &mut App, key: &str, value: SharedString) {
    let _ = AppState::global(cx).db.set_setting(key, value.as_ref());
}

pub struct SettingsPanel {
    focus_handle: gpui::FocusHandle,
    custom_provider: Entity<CustomProviderSection>,
}

impl SettingsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let custom_provider = cx.new(|cx| CustomProviderSection::new(window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            custom_provider,
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

                SettingPage::new("Output Tools")
                    .groups(vec![
                        SettingGroup::new()
                            .title("External Output Services")
                            .items(vec![
                                SettingItem::new(
                                    "Image Service URL",
                                    SettingField::input(
                                        |cx: &App| {
                                            setting_or_env(
                                                cx,
                                                "output_image_endpoint",
                                                "AGENTFORGE_IMAGE_OUTPUT_URL",
                                                "https://api.openai.com/v1/images/generations",
                                            )
                                        },
                                        |val: SharedString, cx: &mut App| {
                                            save_setting(cx, "output_image_endpoint", val);
                                        },
                                    )
                                )
                                .description("OpenAI Images/DALL-E-compatible endpoint or a custom image service."),

                                SettingItem::new(
                                    "Image Model",
                                    SettingField::input(
                                        |cx: &App| {
                                            setting_or_env(
                                                cx,
                                                "output_image_model",
                                                "AGENTFORGE_IMAGE_OUTPUT_MODEL",
                                                "gpt-image-1",
                                            )
                                        },
                                        |val: SharedString, cx: &mut App| {
                                            save_setting(cx, "output_image_model", val);
                                        },
                                    )
                                )
                                .description("Default model sent by the generate_image tool."),

                                SettingItem::new(
                                    "Image API Key",
                                    SettingField::input(
                                        |cx: &App| setting_only(cx, "output_image_api_key"),
                                        |val: SharedString, cx: &mut App| {
                                            save_setting(cx, "output_image_api_key", val);
                                        },
                                    )
                                )
                                .description("Optional. If empty, generate_image also checks AGENTFORGE_IMAGE_OUTPUT_API_KEY and OPENAI_API_KEY."),

                                SettingItem::new(
                                    "PDF Service URL",
                                    SettingField::input(
                                        |cx: &App| {
                                            setting_or_env(
                                                cx,
                                                "output_pdf_endpoint",
                                                "AGENTFORGE_PDF_OUTPUT_URL",
                                                "",
                                            )
                                        },
                                        |val: SharedString, cx: &mut App| {
                                            save_setting(cx, "output_pdf_endpoint", val);
                                        },
                                    )
                                )
                                .description("External PDF renderer endpoint, for example a pdfkit service."),

                                SettingItem::new(
                                    "PDF API Key",
                                    SettingField::input(
                                        |cx: &App| setting_only(cx, "output_pdf_api_key"),
                                        |val: SharedString, cx: &mut App| {
                                            save_setting(cx, "output_pdf_api_key", val);
                                        },
                                    )
                                )
                                .description("Optional bearer token for the PDF service."),

                                SettingItem::new(
                                    "Video Service URL",
                                    SettingField::input(
                                        |cx: &App| {
                                            setting_or_env(
                                                cx,
                                                "output_video_endpoint",
                                                "AGENTFORGE_VIDEO_OUTPUT_URL",
                                                "",
                                            )
                                        },
                                        |val: SharedString, cx: &mut App| {
                                            save_setting(cx, "output_video_endpoint", val);
                                        },
                                    )
                                )
                                .description("External video renderer endpoint."),

                                SettingItem::new(
                                    "Video API Key",
                                    SettingField::input(
                                        |cx: &App| setting_only(cx, "output_video_api_key"),
                                        |val: SharedString, cx: &mut App| {
                                            save_setting(cx, "output_video_api_key", val);
                                        },
                                    )
                                )
                                .description("Optional bearer token for the video service."),

                                SettingItem::new(
                                    "Output Directory",
                                    SettingField::input(
                                        |cx: &App| {
                                            setting_or_env(
                                                cx,
                                                "output_tools_dir",
                                                "AGENTFORGE_OUTPUT_TOOLS_DIR",
                                                "outputs",
                                            )
                                        },
                                        |val: SharedString, cx: &mut App| {
                                            save_setting(cx, "output_tools_dir", val);
                                        },
                                    )
                                )
                                .description("Relative paths are stored under the selected workspace."),
                            ])
                    ]),

                SettingPage::new("User Preferences")
                    .groups(vec![
                        SettingGroup::new()
                            .title("Appearance & UI")
                            .items(vec![
                                SettingItem::new(
                                    "Theme",
                                    SettingField::dropdown(
                                        {
                                            let registry = ThemeRegistry::global(_cx);
                                            let mut theme_names: Vec<SharedString> =
                                                registry.themes().keys().cloned().collect();
                                            theme_names.sort();
                                            theme_names
                                                .into_iter()
                                                .map(|name| (name.clone(), name))
                                                .collect()
                                        },
                                        |cx: &App| {
                                            crate::AppState::global(cx)
                                                .db
                                                .get_setting("theme")
                                                .unwrap_or_default()
                                                .map(SharedString::from)
                                                .unwrap_or_else(|| cx.theme().theme_name().clone())
                                        },
                                        |val: SharedString, cx: &mut App| {
                                            if let Some(theme_config) =
                                                ThemeRegistry::global(cx).themes().get(&val).cloned()
                                            {
                                                gpui_component::Theme::global_mut(cx)
                                                    .apply_config(&theme_config);

                                                let db = &crate::AppState::global(cx).db;
                                                if let Err(e) = db.set_setting("theme", val.as_ref()) {
                                                    eprintln!("Failed to save theme from settings: {}", e);
                                                }

                                                let mode_str = if theme_config.mode.is_dark() {
                                                    "dark"
                                                } else {
                                                    "light"
                                                };
                                                if let Err(e) = db.set_setting("theme_mode", mode_str) {
                                                    eprintln!("Failed to save theme mode from settings: {}", e);
                                                }
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
