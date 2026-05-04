use crate::core::traits::database::DatabasePort;
use crate::db::{Provider, ProviderTemplate};
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
    v_flex, WindowExt,
};

fn sanitize_base_url(s: &str) -> String {
    s.trim().trim_matches('`').trim().to_string()
}

// ── Component ─────────────────────────────────────────────────────────────

pub struct CustomProviderSection {
    provider_select: Entity<SelectState<Vec<SharedString>>>,
    model_select: Entity<SelectState<Vec<SharedString>>>,
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
        let api_key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("API Key (leave empty if not required)")
                .masked(true)
        });
        let base_url_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Base URL (auto-filled per provider)")
        });

        let section = Self {
            provider_select: provider_select.clone(),
            model_select: model_select.clone(),
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
                        let models: Vec<SharedString> = template.models.iter().map(|m: &String| SharedString::from(m.clone())).collect();
                        let url = sanitize_base_url(&template.default_base_url);

                        this.model_select.update(cx, |state, cx| {
                            state.set_items(models, window, cx);
                            state.set_selected_index(None, window, cx);
                        });
                        this.base_url_input.update(cx, |state, cx| {
                            state.set_value(url, window, cx);
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
        let model = self.model_select.read(cx).selected_value();
        let api_key = self.api_key_input.read(cx).text().to_string();
        let base_url = sanitize_base_url(&self.base_url_input.read(cx).text().to_string());

        if provider_name.is_none() || model.is_none() {
            window.push_notification(
                (
                    NotificationType::Error,
                    "Please select a provider and model.",
                ),
                cx,
            );
            return;
        }

        let p_name = provider_name.unwrap().to_string();
        let m_name = model.unwrap().to_string();
        let adapter = self.provider_templates
            .iter()
            .find(|t| t.label == p_name)
            .map(|t| t.adapter.clone())
            .unwrap_or_else(|| "CustomAdapter".to_string());
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
            api_key_ref: if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            },
            status: "available".to_string(),
        };

        let db = crate::AppState::global(cx).db.clone();
        if let Ok(_) = db.insert_provider(&provider) {
            self.custom_providers = db.list_providers().unwrap_or_default();

            // Reset form fields
            self.provider_select
                .update(cx, |s, cx| s.set_selected_index(None, window, cx));
            self.model_select
                .update(cx, |s, cx| s.set_selected_index(None, window, cx));
            self.api_key_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("API Key (leave empty if not required)")
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
}

impl Render for CustomProviderSection {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let mut container = v_flex().gap_4().w_full();

        // ── Provider Table ────────────────────────────────────────────────
        if !self.custom_providers.is_empty() {
            let mut table = v_flex()
                .w_full()
                .border_1()
                .border_color(theme.border)
                .rounded_md();

            // Header row
            table = table.child(
                h_flex()
                    .w_full()
                    .bg(theme.secondary)
                    .p(px(8.))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .w(px(180.))
                            .child("Provider")
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        div()
                            .w(px(160.))
                            .child("Protocol")
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(div().flex_1().child("Model").font_weight(FontWeight::BOLD))
                    .child(
                        div()
                            .w(px(90.))
                            .child("Status")
                            .font_weight(FontWeight::BOLD),
                    ),
            );

            for p in &self.custom_providers {
                let protocol = self.provider_templates
                    .iter()
                    .find(|t| t.label == p.provider_name)
                    .map(|t| t.protocol.clone())
                    .unwrap_or_else(|| "REST API".to_string());
                table = table.child(
                    h_flex()
                        .w_full()
                        .p(px(8.))
                        .border_b_1()
                        .border_color(theme.border)
                        .child(div().w(px(180.)).child(p.provider_name.clone()))
                        .child(div().w(px(160.)).child(protocol))
                        .child(div().flex_1().child(p.model.clone()))
                        .child(div().w(px(90.)).child(p.status.clone())),
                );
            }
            container = container.child(table);
        } else {
            container =
                container.child(div().text_color(theme.muted_foreground).child(
                    "No AI providers configured yet. Click \"Add Provider\" to register one.",
                ));
        }

        // ── Add Provider Button → opens dialog ────────────────────────────
        let view = cx.entity().clone();
        let provider_select = self.provider_select.clone();
        let model_select = self.model_select.clone();
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
                                            // Model selection (auto-populated based on provider)
                                            .child(field().label("Model").required(true).child(
                                                Select::new(&m_sel).placeholder("Select Model"),
                                            ))
                                            // Base URL (auto-filled per provider, editable for custom)
                                            .child(
                                                field()
                                                    .label("Base URL / Endpoint")
                                                    .child(Input::new(&b_inp)),
                                            )
                                            // API Key (masked, optional for self-hosted)
                                            .child(
                                                field()
                                                    .label("API Key")
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
