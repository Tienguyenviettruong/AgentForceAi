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
    v_flex, IconName, IndexPath, Sizable, WindowExt,
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
            crate::AppState::notify_providers_changed(cx);

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
            .filter(|account| account.starts_with("custom-provider-"))
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

    fn update_provider(
        &mut self,
        provider_id: String,
        provider_type_select: Entity<SelectState<Vec<SharedString>>>,
        display_name_input: Entity<InputState>,
        model_input: Entity<InputState>,
        endpoint_input: Entity<InputState>,
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

        let provider_name = sanitize_display_name(&display_name_input.read(cx).text().to_string());
        let model = sanitize_model_id(&model_input.read(cx).text().to_string());
        let endpoint = sanitize_base_url(&endpoint_input.read(cx).text().to_string());
        let selected_provider_type = provider_type_select
            .read(cx)
            .selected_value()
            .map(ToString::to_string)
            .unwrap_or_default();
        let adapter_type = self
            .provider_templates
            .iter()
            .find(|template| template.label == selected_provider_type)
            .map(|template| template.adapter.clone())
            .unwrap_or_else(|| existing_provider.adapter_type.clone());

        if provider_name.is_empty() || model.is_empty() {
            window.push_notification(
                (
                    NotificationType::Error,
                    "Display name and model/deployment ID are required.",
                ),
                cx,
            );
            return;
        }
        if adapter_type == "CustomAdapter" && endpoint.is_empty() {
            window.push_notification(
                (
                    NotificationType::Error,
                    "A Base URL / endpoint is required for a custom provider.",
                ),
                cx,
            );
            return;
        }
        if self.custom_providers.iter().any(|provider| {
            provider.id != existing_provider.id
                && provider.provider_name.eq_ignore_ascii_case(&provider_name)
                && provider.model.eq_ignore_ascii_case(&model)
        }) {
            window.push_notification(
                (
                    NotificationType::Error,
                    "Another provider already uses this display name and model.",
                ),
                cx,
            );
            return;
        }

        let api_key = key_input.read(cx).text().to_string();
        let api_key_ref = if api_key.trim().is_empty() {
            existing_provider.api_key_ref.clone()
        } else {
            let Some(reference) = self.build_api_key_ref(
                &api_key,
                existing_provider.api_key_ref.as_deref(),
                window,
                cx,
            ) else {
                return;
            };
            Some(reference)
        };

        let mut updated_provider = existing_provider.clone();
        updated_provider.provider_name = provider_name;
        updated_provider.model = model;
        updated_provider.adapter_type = adapter_type;
        updated_provider.command = (!endpoint.is_empty()).then_some(endpoint);
        updated_provider.api_key_ref = api_key_ref;
        updated_provider.status = "available".to_string();

        let db = crate::AppState::global(cx).db.clone();
        match db.update_provider(&updated_provider) {
            Ok(()) => {
                self.custom_providers = db.list_providers().unwrap_or_default();
                crate::AppState::notify_providers_changed(cx);
                if existing_provider.api_key_ref != updated_provider.api_key_ref {
                    if let Err(error) =
                        Self::delete_owned_secret(existing_provider.api_key_ref.as_deref())
                    {
                        window.push_notification(
                            (
                                NotificationType::Warning,
                                SharedString::from(format!(
                                    "Provider updated, but the previous credential could not be removed: {}",
                                    error
                                )),
                            ),
                            cx,
                        );
                    }
                }
                window.close_dialog(cx);
                window.push_notification(
                    (NotificationType::Success, "Provider updated successfully."),
                    cx,
                );
                cx.notify();
            }
            Err(error) => {
                window.push_notification(
                    (
                        NotificationType::Error,
                        gpui::SharedString::from(format!("Failed to update provider: {}", error)),
                    ),
                    cx,
                );
            }
        }
    }

    fn delete_owned_secret(api_key_ref: Option<&str>) -> anyhow::Result<()> {
        let Some(account) = secret_account_from_ref(api_key_ref)
            .filter(|account| account.starts_with("custom-provider-"))
        else {
            return Ok(());
        };
        smol::block_on(async {
            let keychain = crate::infrastructure::security::keychain::Keychain::new().await?;
            keychain
                .delete_secret(
                    crate::infrastructure::security::keychain::SECURE_SECRET_SERVICE,
                    &account,
                )
                .await
        })
    }

    fn delete_provider(
        &mut self,
        provider_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(provider) = self
            .custom_providers
            .iter()
            .find(|provider| provider.id == provider_id)
            .cloned()
        else {
            window.push_notification((NotificationType::Error, "Provider was not found."), cx);
            return;
        };

        let db = crate::AppState::global(cx).db.clone();
        match db.delete_provider(&provider.id) {
            Ok(()) => {
                self.custom_providers = db.list_providers().unwrap_or_default();
                crate::AppState::notify_providers_changed(cx);
                let credential_cleanup = Self::delete_owned_secret(provider.api_key_ref.as_deref());
                window.close_dialog(cx);
                if let Err(error) = credential_cleanup {
                    window.push_notification(
                        (
                            NotificationType::Warning,
                            SharedString::from(format!(
                                "Provider deleted, but its credential could not be removed: {}",
                                error
                            )),
                        ),
                        cx,
                    );
                } else {
                    window.push_notification(
                        (NotificationType::Success, "Provider deleted successfully."),
                        cx,
                    );
                }
                cx.notify();
            }
            Err(error) => window.push_notification(
                (
                    NotificationType::Error,
                    SharedString::from(format!("Failed to delete provider: {}", error)),
                ),
                cx,
            ),
        }
    }

    fn open_edit_provider_dialog(
        &self,
        provider: Provider,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut provider_type_options = self
            .provider_templates
            .iter()
            .map(|template| SharedString::from(template.label.clone()))
            .collect::<Vec<_>>();
        let selected_type_index = self
            .provider_templates
            .iter()
            .position(|template| template.adapter == provider.adapter_type)
            .unwrap_or_else(|| {
                provider_type_options.push(SharedString::from(provider.adapter_type.clone()));
                provider_type_options.len() - 1
            });
        let provider_type_select = cx.new(|cx| {
            SelectState::new(
                provider_type_options,
                Some(IndexPath::new(selected_type_index)),
                window,
                cx,
            )
        });
        let display_name_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Provider display name");
            state.set_value(provider.provider_name.clone(), window, cx);
            state
        });
        let model_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Model or deployment ID");
            state.set_value(provider.model.clone(), window, cx);
            state
        });
        let endpoint_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Base URL / endpoint");
            state.set_value(provider.command.clone().unwrap_or_default(), window, cx);
            state
        });
        let key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Leave blank to keep the current credential")
                .masked(true)
        });
        let current_credential = credential_label(provider.api_key_ref.as_deref());
        let provider_id = provider.id.clone();
        let view = cx.entity().clone();
        let muted_foreground = cx.theme().muted_foreground;

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let provider_type_select_for_footer = provider_type_select.clone();
            let display_name_for_footer = display_name_input.clone();
            let model_for_footer = model_input.clone();
            let endpoint_for_footer = endpoint_input.clone();
            let key_for_footer = key_input.clone();
            let provider_id_for_footer = provider_id.clone();
            let view_for_footer = view.clone();

            dialog
                .title("Edit AI Provider")
                .w(px(560.))
                .child(
                    v_form()
                        .gap(px(12.))
                        .py(px(8.))
                        .child(
                            field()
                                .label("Provider Type")
                                .required(true)
                                .child(Select::new(&provider_type_select)),
                        )
                        .child(
                            field()
                                .label("Display Name")
                                .required(true)
                                .child(Input::new(&display_name_input)),
                        )
                        .child(
                            field()
                                .label("Model / Deployment ID")
                                .required(true)
                                .child(Input::new(&model_input)),
                        )
                        .child(
                            field()
                                .label("Base URL / Endpoint")
                                .child(Input::new(&endpoint_input)),
                        )
                        .child(
                            field().label("New API Key / Reference").child(
                                v_flex()
                                    .gap(px(5.))
                                    .child(Input::new(&key_input).mask_toggle())
                                    .child(div().text_xs().text_color(muted_foreground).child(
                                        format!(
                                            "Current: {}. Leave blank to keep it.",
                                            current_credential
                                        ),
                                    )),
                            ),
                        ),
                )
                .footer(move |_, _, _, _| {
                    let provider_type_select = provider_type_select_for_footer.clone();
                    let display_name_input = display_name_for_footer.clone();
                    let model_input = model_for_footer.clone();
                    let endpoint_input = endpoint_for_footer.clone();
                    let key_input = key_for_footer.clone();
                    let provider_id = provider_id_for_footer.clone();
                    let view = view_for_footer.clone();
                    vec![
                        Button::new("cancel-edit-provider")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx))
                            .into_any_element(),
                        Button::new("save-provider-changes")
                            .primary()
                            .label("Save Changes")
                            .on_click(move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.update_provider(
                                        provider_id.clone(),
                                        provider_type_select.clone(),
                                        display_name_input.clone(),
                                        model_input.clone(),
                                        endpoint_input.clone(),
                                        key_input.clone(),
                                        window,
                                        cx,
                                    );
                                });
                            })
                            .into_any_element(),
                    ]
                })
        });
    }

    fn open_delete_provider_dialog(
        &self,
        provider: Provider,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let provider_id = provider.id.clone();
        let provider_identity = format!("{} / {}", provider.provider_name, provider.model);
        let view = cx.entity().clone();
        let muted_foreground = cx.theme().muted_foreground;
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let provider_id_for_footer = provider_id.clone();
            let view_for_footer = view.clone();
            dialog
                .title("Delete AI Provider")
                .w(px(480.))
                .child(
                    v_flex()
                        .gap(px(10.))
                        .py(px(8.))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(provider_identity.clone()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(muted_foreground)
                                .child("This removes the provider from every model picker. This action cannot be undone."),
                        ),
                )
                .footer(move |_, _, _, _| {
                    let provider_id = provider_id_for_footer.clone();
                    let view = view_for_footer.clone();
                    vec![
                        Button::new("cancel-delete-provider")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx))
                            .into_any_element(),
                        Button::new("confirm-delete-provider")
                            .danger()
                            .label("Delete Provider")
                            .on_click(move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.delete_provider(provider_id.clone(), window, cx);
                                });
                            })
                            .into_any_element(),
                    ]
                })
        });
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
                let provider_for_edit = p.clone();
                let provider_for_delete = p.clone();
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
                            h_flex()
                                .w(px(150.))
                                .justify_end()
                                .gap(px(6.))
                                .child(
                                    Button::new(("edit-provider", ix))
                                        .small()
                                        .icon(IconName::Settings2)
                                        .label("Edit")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.open_edit_provider_dialog(
                                                provider_for_edit.clone(),
                                                window,
                                                cx,
                                            );
                                        })),
                                )
                                .child(
                                    Button::new(("delete-provider", ix))
                                        .small()
                                        .danger()
                                        .icon(IconName::Delete)
                                        .tooltip("Delete provider")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.open_delete_provider_dialog(
                                                provider_for_delete.clone(),
                                                window,
                                                cx,
                                            );
                                        })),
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
