use crate::db::{Provider, ProviderTemplate};
use crate::infrastructure::security::keychain::is_credential_reference;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AppContext, Context, Entity, FontWeight, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    form::{field, v_form},
    h_flex,
    input::{Input, InputState},
    notification::NotificationType,
    select::{Select, SelectEvent, SelectState},
    theme::ActiveTheme,
    v_flex, Sizable, WindowExt,
};

fn sanitize_base_url(s: &str) -> String {
    s.trim().trim_matches('`').trim().to_string()
}

fn sanitize_model_id(s: &str) -> String {
    s.trim().trim_matches('`').trim().to_string()
}

fn sanitize_display_name(s: &str) -> String {
    s.trim()
        .trim_matches('`')
        .trim()
        .replace(" / ", " - ")
        .to_string()
}

fn unique_provider_name(desired: &str, existing: &[Provider]) -> String {
    let desired = desired.trim();
    if !existing.iter().any(|p| p.provider_name == desired) {
        return desired.to_string();
    }
    for ix in 2..1000 {
        let candidate = format!("{} {}", desired, ix);
        if !existing.iter().any(|p| p.provider_name == candidate) {
            return candidate;
        }
    }
    format!("{} {}", desired, uuid::Uuid::new_v4())
}

fn provider_protocol(p: &Provider, templates: &[ProviderTemplate]) -> String {
    templates
        .iter()
        .find(|t| t.label == p.provider_name)
        .or_else(|| templates.iter().find(|t| t.adapter == p.adapter_type))
        .map(|t| t.protocol.clone())
        .unwrap_or_else(|| p.adapter_type.clone())
}

fn endpoint_label(command: Option<&str>) -> String {
    let Some(endpoint) = command.map(str::trim).filter(|value| !value.is_empty()) else {
        return "Default endpoint".to_string();
    };
    let without_scheme = endpoint
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(endpoint);
    without_scheme
        .split('/')
        .next()
        .filter(|host| !host.trim().is_empty())
        .unwrap_or(endpoint)
        .to_string()
}

fn credential_label(api_key_ref: Option<&str>) -> String {
    match api_key_ref.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) if value.starts_with("secret://") => "Secret stored".to_string(),
        Some(value) if value.starts_with("env:") => {
            format!("Env: {}", value.trim_start_matches("env:"))
        }
        Some(_) => "Credential ref".to_string(),
        None => "No key".to_string(),
    }
}

fn secret_account_from_ref(api_key_ref: Option<&str>) -> Option<String> {
    api_key_ref
        .map(str::trim)
        .and_then(|value| value.strip_prefix("secret://"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn provider_status(p: &Provider) -> (&'static str, gpui::Hsla) {
    let status = p.status.trim().to_lowercase();
    if !matches!(
        status.as_str(),
        "available" | "online" | "active" | "healthy" | "ready"
    ) {
        return ("Offline", gpui::red());
    }
    if p.adapter_type == "CustomAdapter"
        && p.command.as_deref().unwrap_or_default().trim().is_empty()
    {
        return ("Needs endpoint", gpui::yellow());
    }
    if p.api_key_ref
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return ("Needs key", gpui::yellow());
    }
    ("Ready", gpui::green())
}

// ── Component ─────────────────────────────────────────────────────────────

pub struct CustomProviderSection {
    provider_select: Entity<SelectState<Vec<SharedString>>>,
    model_select: Entity<SelectState<Vec<SharedString>>>,
    provider_alias_input: Entity<InputState>,
    model_override_input: Entity<InputState>,
    api_key_input: Entity<InputState>,
    base_url_input: Entity<InputState>,

    custom_providers: Vec<Provider>,
    provider_templates: Vec<ProviderTemplate>,
}

impl CustomProviderSection {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        let custom_providers = db.list_providers().unwrap_or_default();
        let provider_templates = db.list_provider_templates().unwrap_or_default();

        let providers: Vec<SharedString> = provider_templates
            .iter()
            .map(|t| SharedString::from(t.label.clone()))
            .collect();

        let provider_select = cx.new(|cx| SelectState::new(providers, None, window, cx));
        let model_select =
            cx.new(|cx| SelectState::new(Vec::<SharedString>::new(), None, window, cx));
        let provider_alias_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Display name (optional)"));
        let model_override_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Actual model or deployment id"));
        let api_key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("secret://account or env:VARIABLE (optional)")
                .masked(true)
        });
        let base_url_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Base URL (auto-filled per provider)")
        });

        let section = Self {
            provider_select: provider_select.clone(),
            model_select: model_select.clone(),
            provider_alias_input,
            model_override_input,
            api_key_input,
            base_url_input: base_url_input.clone(),
            custom_providers,
            provider_templates: provider_templates.clone(),
        };

        // When provider changes → update model list + auto-fill base URL
        cx.subscribe_in(
            &provider_select,
            window,
            move |this: &mut Self, _state, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                if let SelectEvent::Confirm(Some(val)) = event {
                    let label = val.as_str();
                    if let Some(template) = provider_templates.iter().find(|t| t.label == label) {
                        let models: Vec<SharedString> = template
                            .models
                            .iter()
                            .map(|m: &String| SharedString::from(m.clone()))
                            .collect();
                        let url = sanitize_base_url(&template.default_base_url);

                        this.model_select.update(cx, |state, cx| {
                            state.set_items(models, window, cx);
                            state.set_selected_index(None, window, cx);
                        });
                        this.base_url_input.update(cx, |state, cx| {
                            state.set_value(url, window, cx);
                        });
                        this.model_override_input.update(cx, |state, cx| {
                            state.set_value("", window, cx);
                        });
                    }
                }
            },
        )
        .detach();

        section
    }

    fn save_provider(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let provider_name = self.provider_select.read(cx).selected_value();
        let selected_model = self.model_select.read(cx).selected_value();
        let provider_alias =
            sanitize_display_name(&self.provider_alias_input.read(cx).text().to_string());
        let model_override =
            sanitize_model_id(&self.model_override_input.read(cx).text().to_string());
        let api_key = self.api_key_input.read(cx).text().to_string();
        let base_url = sanitize_base_url(&self.base_url_input.read(cx).text().to_string());

        if provider_name.is_none() {
            window.push_notification((NotificationType::Error, "Please select a provider."), cx);
            return;
        }
        let api_key_trim = api_key.trim();
        let api_key_ref = if api_key_trim.is_empty() {
            None
        } else if is_credential_reference(api_key_trim) {
            Some(api_key_trim.to_string())
        } else {
            let secret_account = format!("custom-provider-{}", uuid::Uuid::new_v4());
            let save_result = smol::block_on(async {
                let keychain = crate::infrastructure::security::keychain::Keychain::new().await?;
                keychain
                    .set_secret(
                        crate::infrastructure::security::keychain::SECURE_SECRET_SERVICE,
                        &secret_account,
                        api_key_trim,
                    )
                    .await?;
                Ok::<(), anyhow::Error>(())
            });

            match save_result {
                Ok(()) => Some(format!("secret://{}", secret_account)),
                Err(error) => {
                    window.push_notification(
                        (
                            NotificationType::Error,
                            gpui::SharedString::from(format!(
                                "Failed to store raw credential in OS keychain: {}",
                                error
                            )),
                        ),
                        cx,
                    );
                    return;
                }
            }
        };

        let template_name = provider_name.unwrap().to_string();
        let adapter = self
            .provider_templates
            .iter()
            .find(|t| t.label == template_name)
            .map(|t| t.adapter.clone())
            .unwrap_or_else(|| "CustomAdapter".to_string());
        let p_name = if provider_alias.is_empty() {
            unique_provider_name(&template_name, &self.custom_providers)
        } else {
            unique_provider_name(&provider_alias, &self.custom_providers)
        };
        let selected_model = selected_model
            .map(|m| sanitize_model_id(&m.to_string()))
            .unwrap_or_default();
        let m_name = if model_override.is_empty() {
            selected_model
        } else {
            model_override
        };

        if m_name.is_empty() || (adapter == "CustomAdapter" && m_name == "custom-model") {
            window.push_notification(
                (
                    NotificationType::Error,
                    "Enter the real custom provider model or deployment id.",
                ),
                cx,
            );
            return;
        }

        let command = if base_url.is_empty() {
            None
        } else {
            Some(base_url)
        };

        let provider = Provider {
            id: uuid::Uuid::new_v4().to_string(),
            provider_name: p_name,
            model: m_name,
            adapter_type: adapter,
            command,
            api_key_ref,
            status: "available".to_string(),
            // capabilities: None = text-only by default (safe for local/unknown models)
            capabilities: None,
        };

        let db = crate::AppState::global(cx).db.clone();
        if let Ok(_) = db.insert_provider(&provider) {
            self.custom_providers = db.list_providers().unwrap_or_default();

            // Reset form fields
            self.provider_select
                .update(cx, |s, cx| s.set_selected_index(None, window, cx));
            self.model_select
                .update(cx, |s, cx| s.set_selected_index(None, window, cx));
            self.provider_alias_input =
                cx.new(|cx| InputState::new(window, cx).placeholder("Display name (optional)"));
            self.model_override_input = cx
                .new(|cx| InputState::new(window, cx).placeholder("Actual model or deployment id"));
            self.api_key_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("secret://account or env:VARIABLE (optional)")
                    .masked(true)
            });
            self.base_url_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Base URL (auto-filled per provider)")
            });

            window.close_dialog(cx);
            window.push_notification(
                (NotificationType::Success, "Provider saved successfully!"),
                cx,
            );
            cx.notify();
        } else {
            window.push_notification((NotificationType::Error, "Failed to save provider."), cx);
        }
    }

    fn build_api_key_ref(
        &self,
        api_key: &str,
        existing_ref: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let api_key_trim = api_key.trim();
        if api_key_trim.is_empty() {
            window.push_notification(
                (
                    NotificationType::Error,
                    "Enter a new API key, secret:// reference, or env:VARIABLE.",
                ),
                cx,
            );
            return None;
        }
        if is_credential_reference(api_key_trim) {
            return Some(api_key_trim.to_string());
        }

        let secret_account = secret_account_from_ref(existing_ref)
            .unwrap_or_else(|| format!("custom-provider-{}", uuid::Uuid::new_v4()));
        let save_result = smol::block_on(async {
            let keychain = crate::infrastructure::security::keychain::Keychain::new().await?;
            keychain
                .set_secret(
                    crate::infrastructure::security::keychain::SECURE_SECRET_SERVICE,
                    &secret_account,
                    api_key_trim,
                )
                .await?;
            Ok::<(), anyhow::Error>(())
        });

        match save_result {
            Ok(()) => Some(format!("secret://{}", secret_account)),
            Err(error) => {
                window.push_notification(
                    (
                        NotificationType::Error,
                        gpui::SharedString::from(format!(
                            "Failed to store credential in OS keychain: {}",
                            error
                        )),
                    ),
                    cx,
                );
                None
            }
        }
    }

    fn update_provider_key(
        &mut self,
        provider_id: String,
        key_input: Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(existing_provider) = self
            .custom_providers
            .iter()
            .find(|provider| provider.id == provider_id)
            .cloned()
        else {
            window.push_notification((NotificationType::Error, "Provider was not found."), cx);
            return;
        };

        let api_key = key_input.read(cx).text().to_string();
        let Some(api_key_ref) = self.build_api_key_ref(
            &api_key,
            existing_provider.api_key_ref.as_deref(),
            window,
            cx,
        ) else {
            return;
        };

        let mut updated_provider = existing_provider;
        updated_provider.api_key_ref = Some(api_key_ref);
        updated_provider.status = "available".to_string();

        let db = crate::AppState::global(cx).db.clone();
        match db.insert_provider(&updated_provider) {
            Ok(()) => {
                self.custom_providers = db.list_providers().unwrap_or_default();
                window.close_dialog(cx);
                window.push_notification(
                    (NotificationType::Success, "Provider API key updated."),
                    cx,
                );
                cx.notify();
            }
            Err(error) => {
                window.push_notification(
                    (
                        NotificationType::Error,
                        gpui::SharedString::from(format!(
                            "Failed to update provider key: {}",
                            error
                        )),
                    ),
                    cx,
                );
            }
        }
    }
}

impl Render for CustomProviderSection {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let view = cx.entity().clone();
        let mut container = v_flex().gap_4().w_full();

        // ── Provider Table ────────────────────────────────────────────────
        if !self.custom_providers.is_empty() {
            let mut provider_list = v_flex()
                .w_full()
                .border_1()
                .border_color(theme.border.opacity(0.55))
                .rounded_md()
                .overflow_hidden()
                .bg(theme.background);

            for (ix, p) in self.custom_providers.iter().enumerate() {
                let protocol = provider_protocol(p, &self.provider_templates);
                let endpoint = endpoint_label(p.command.as_deref());
                let credential = credential_label(p.api_key_ref.as_deref());
                let (status_label, status_color) = provider_status(p);
                let provider_for_key = p.clone();
                let view_for_key = view.clone();
                let dialog_muted_foreground = theme.muted_foreground;
                provider_list = provider_list.child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap(px(12.))
                        .p(px(10.))
                        .when(ix > 0, |row| {
                            row.border_t_1().border_color(theme.border.opacity(0.35))
                        })
                        .child(
                            div()
                                .w(px(230.))
                                .min_w(px(0.))
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(p.provider_name.clone()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(protocol),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .child(div().child(p.model.clone()))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(endpoint),
                                ),
                        )
                        .child(
                            div()
                                .w(px(150.))
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(credential),
                        )
                        .child(
                            div().w(px(120.)).child(
                                div()
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded_full()
                                    .bg(status_color.opacity(0.12))
                                    .text_color(status_color)
                                    .text_xs()
                                    .child(status_label),
                            ),
                        )
                        .child(
                            div().w(px(110.)).child(
                                Button::new(("update-key", ix))
                                    .small()
                                    .label("Update Key")
                                    .on_click(move |_ev, window, cx| {
                                        let provider = provider_for_key.clone();
                                        let view_save = view_for_key.clone();
                                        let key_input = cx.new(|cx| {
                                            InputState::new(window, cx)
                                                .placeholder("Paste new key, secret://..., or env:...")
                                                .masked(true)
                                        });
                                        if let Some(env_ref) = provider
                                            .api_key_ref
                                            .as_deref()
                                            .filter(|value| value.trim().starts_with("env:"))
                                        {
                                            key_input.update(cx, |state, cx| {
                                                state.set_value(env_ref.to_string(), window, cx);
                                            });
                                        }
                                        let key_input_for_save = key_input.clone();
                                        window.open_dialog(cx, move |dialog, _window, _cx| {
                                            let provider_id = provider.id.clone();
                                            let provider_name = provider.provider_name.clone();
                                            let provider_model = provider.model.clone();
                                            let current_credential =
                                                credential_label(provider.api_key_ref.as_deref());
                                            let view_save2 = view_save.clone();
                                            let key_input_for_footer = key_input_for_save.clone();

                                            dialog
                                                .title("Update Provider Key")
                                                .w(px(520.))
                                                .child(
                                                    v_flex()
                                                        .gap(px(12.))
                                                        .py(px(8.))
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .text_color(
                                                                    dialog_muted_foreground,
                                                                )
                                                                .child(format!(
                                                                    "{} / {}",
                                                                    provider_name, provider_model
                                                                )),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(
                                                                    dialog_muted_foreground,
                                                                )
                                                                .child(format!(
                                                                    "Current credential: {}",
                                                                    current_credential
                                                                )),
                                                        )
                                                        .child(
                                                            field()
                                                                .label("New API Key / Reference")
                                                                .required(true)
                                                                .child(
                                                                    Input::new(
                                                                        &key_input_for_save,
                                                                    )
                                                                    .mask_toggle(),
                                                                ),
                                                        ),
                                                )
                                                .footer(move |_, _, _, _| {
                                                    let provider_id = provider_id.clone();
                                                    let view_save3 = view_save2.clone();
                                                    let key_input_for_save =
                                                        key_input_for_footer.clone();
                                                    vec![
                                                        Button::new("cancel-update-provider-key")
                                                            .label("Cancel")
                                                            .on_click(|_, window, cx| {
                                                                window.close_dialog(cx);
                                                            })
                                                            .into_any_element(),
                                                        Button::new("save-provider-key")
                                                            .primary()
                                                            .label("Save Key")
                                                            .on_click(move |_ev, window, cx| {
                                                                view_save3.update(
                                                                    cx,
                                                                    |this: &mut CustomProviderSection, cx| {
                                                                        this.update_provider_key(
                                                                            provider_id.clone(),
                                                                            key_input_for_save.clone(),
                                                                            window,
                                                                            cx,
                                                                        )
                                                                    },
                                                                );
                                                            })
                                                            .into_any_element(),
                                                    ]
                                                })
                                        });
                                    }),
                            ),
                        ),
                );
            }
            container = container.child(provider_list);
        } else {
            container =
                container.child(div().text_color(theme.muted_foreground).child(
                    "No AI providers configured yet. Click \"Add Provider\" to register one.",
                ));
        }

        // ── Add Provider Button → opens dialog ────────────────────────────
        let provider_select = self.provider_select.clone();
        let model_select = self.model_select.clone();
        let provider_alias_input = self.provider_alias_input.clone();
        let model_override_input = self.model_override_input.clone();
        let api_key_input = self.api_key_input.clone();
        let base_url_input = self.base_url_input.clone();

        container =
            container.child(
                h_flex().child(
                    Button::new("btn-add-provider")
                        .primary()
                        .label("＋  Add Provider")
                        .on_click(move |_ev, window, cx| {
                            let view_save = view.clone();
                            let p_sel = provider_select.clone();
                            let m_sel = model_select.clone();
                            let p_alias = provider_alias_input.clone();
                            let m_override = model_override_input.clone();
                            let a_inp = api_key_input.clone();
                            let b_inp = base_url_input.clone();

                            window.open_dialog(cx, move |dialog, _window, _cx| {
                                let view_save2 = view_save.clone();

                                dialog
                                    .title("Add AI Provider")
                                    .w(px(520.))
                                    .child(
                                        v_form()
                                            .gap(px(12.))
                                            .py(px(8.))
                                            // Provider name (PRD §8.1: P0=Anthropic/OpenAI/Google, P1=iFlow/OpenCode, P2=Custom)
                                            .child(
                                                field().label("Provider").required(true).child(
                                                    Select::new(&p_sel)
                                                        .placeholder("Select Provider (P0–P2)"),
                                                ),
                                            )
                                            .child(
                                                field()
                                                    .label("Display Name")
                                                    .child(Input::new(&p_alias)),
                                            )
                                            // Model selection (auto-populated based on provider)
                                            .child(field().label("Template Model").child(
                                                Select::new(&m_sel).placeholder("Select Model"),
                                            ))
                                            .child(
                                                field()
                                                    .label("Model / Deployment ID")
                                                    .child(Input::new(&m_override)),
                                            )
                                            // Base URL (auto-filled per provider, editable for custom)
                                            .child(
                                                field()
                                                    .label("Base URL / Endpoint")
                                                    .child(Input::new(&b_inp)),
                                            )
                                            // API key reference (raw credentials are not persisted)
                                            .child(
                                                field()
                                                    .label("API Key Reference")
                                                    .child(Input::new(&a_inp).mask_toggle()),
                                            ),
                                    )
                                    .footer(move |_, _, _, _| {
                                        let view_save3 = view_save2.clone();
                                        vec![
                                            Button::new("cancel-provider")
                                                .label("Cancel")
                                                .on_click(|_, window, cx| {
                                                    window.close_dialog(cx);
                                                })
                                                .into_any_element(),
                                            Button::new("save-provider")
                                                .primary()
                                                .label("Save Provider")
                                                .on_click(move |_ev, window, cx| {
                                                    view_save3.update(
                                                        cx,
                                                        |this: &mut CustomProviderSection, cx| {
                                                            this.save_provider(window, cx)
                                                        },
                                                    );
                                                })
                                                .into_any_element(),
                                        ]
                                    })
                            });
                        }),
                ),
            );

        container
    }
}
