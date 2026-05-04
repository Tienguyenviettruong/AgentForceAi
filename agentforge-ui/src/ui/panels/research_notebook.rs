use gpui::EventEmitter;
use gpui::{div, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled, Window};
use gpui_component::dock::PanelEvent;
use gpui_component::dock::{Panel, TitleStyle};
use gpui_component::StyledExt;
use gpui_component::scroll::ScrollableElement;
use gpui_component::{
    button::Button,
    h_flex, v_flex,
    input::{Input, InputState},
    theme::ActiveTheme,
};
use std::sync::Arc;
use crate::infrastructure::mcp::ActionRecorder;
use crate::providers::BaseProviderAdapter;

pub struct ResearchNotebookPanel {
    focus_handle: gpui::FocusHandle,
    search_query: String,
    search_input: Entity<InputState>,
    is_searching: bool,
    search_results: Vec<SearchResult>,
    scratchpad: String,
    save_status: Option<String>,
    action_recorder: Arc<ActionRecorder>,
}

#[derive(Clone)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn strip_tags(input: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in input.chars() {
        if c == '<' {
            in_tag = true;
            continue;
        }
        if c == '>' {
            in_tag = false;
            out.push(' ');
            continue;
        }
        if !in_tag {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

async fn ddg_search(query: &str) -> anyhow::Result<Vec<SearchResult>> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );
    let client = reqwest::Client::new();
    let html = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
        )
        .send()
        .await?
        .text()
        .await?;

    let mut results = Vec::new();
    let mut pos = 0usize;
    while let Some(ix) = html[pos..].find("result__a") {
        let ix = pos + ix;
        let href_ix = match html[ix..].find("href=\"") {
            Some(v) => ix + v + 6,
            None => break,
        };
        let href_end = match html[href_ix..].find('"') {
            Some(v) => href_ix + v,
            None => break,
        };
        let href = html[href_ix..href_end].to_string();

        let title_start = match html[href_end..].find('>') {
            Some(v) => href_end + v + 1,
            None => break,
        };
        let title_end = match html[title_start..].find("</a>") {
            Some(v) => title_start + v,
            None => break,
        };
        let title_raw = &html[title_start..title_end];
        let title = strip_tags(title_raw);

        let mut snippet = String::new();
        if let Some(sn_ix) = html[title_end..].find("result__snippet") {
            let sn_ix = title_end + sn_ix;
            if let Some(sn_gt) = html[sn_ix..].find('>') {
                let sn_start = sn_ix + sn_gt + 1;
                if let Some(sn_end) = html[sn_start..].find("</") {
                    let sn_end = sn_start + sn_end;
                    snippet = strip_tags(&html[sn_start..sn_end]);
                }
            }
        }

        if !title.is_empty() && !href.is_empty() {
            results.push(SearchResult {
                title,
                url: href,
                snippet,
            });
        }

        if results.len() >= 6 {
            break;
        }
        pos = title_end;
    }

    Ok(results)
}

fn build_adapter(provider: &crate::db::Provider) -> Option<Arc<dyn BaseProviderAdapter>> {
    match provider.adapter_type.as_str() {
        "AnthropicAdapter" => {
            let mut a = crate::providers::claude::ClaudeAdapter::new();
            a.initialize(provider).ok()?;
            Some(Arc::new(a))
        }
        "OpenAIAdapter" => {
            let mut a = crate::providers::codex::CodexAdapter::new();
            a.initialize(provider).ok()?;
            Some(Arc::new(a))
        }
        "GeminiAdapter" => {
            let mut a = crate::providers::gemini::GeminiAdapter::new();
            a.initialize(provider).ok()?;
            Some(Arc::new(a))
        }
        "OpenCodeAdapter" => {
            let mut a = crate::providers::opencode::OpenCodeAdapter::new();
            a.initialize(provider).ok()?;
            Some(Arc::new(a))
        }
        _ => None,
    }
}

impl ResearchNotebookPanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        let action_recorder = Arc::new(ActionRecorder::new(db));
        let search_input = cx.new(|cx| InputState::new(_window, cx).placeholder("Search query"));
        
        Self {
            focus_handle: cx.focus_handle(),
            search_query: String::new(),
            search_input,
            is_searching: false,
            search_results: Vec::new(),
            scratchpad: "## Research Notes\n\nDraft your synthesized research here before saving to the knowledge base...".to_string(),
            save_status: None,
            action_recorder,
        }
    }
}

impl Panel for ResearchNotebookPanel {
    fn panel_name(&self) -> &'static str {
        "Working Memory"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for ResearchNotebookPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ResearchNotebookPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let action_recorder = self.action_recorder.clone();
        let db = crate::AppState::global(cx).db.clone();

        let header = h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .p_4()
            .border_b_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .flex_1()
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.search_input).appearance(false)),
                    ),
            )
            .child(
                Button::new("btn-trigger-search")
                    .label(if self.is_searching { "Gathering Data..." } else { "Auto-Search via Agent" })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.is_searching {
                            let query = this.search_input.read(cx).text().to_string();
                            let query = query.trim().to_string();
                            if query.is_empty() {
                                return;
                            }
                            this.is_searching = true;
                            this.search_query = query.clone();
                            cx.notify();
                            
                            let view = cx.entity().clone();
                            let action_recorder = action_recorder.clone();
                            let db = db.clone();
                            
                            cx.spawn(async move |_, cx| {
                                let results = ddg_search(&query).await.unwrap_or_default();
                                action_recorder.record_action(
                                    "web_search".to_string(),
                                    serde_json::json!({"query": query}).to_string(),
                                    format!("Found {} results", results.len()),
                                );

                                let provider = db.list_providers().ok().and_then(|ps| {
                                    ps.into_iter().find(|p| p.status == "available")
                                });
                                let adapter = provider
                                    .as_ref()
                                    .and_then(|p| build_adapter(p));
                                let _ = action_recorder.generate_iflow_and_save(adapter).await;

                                let _ = cx.update(|cx| {
                                    let _ = view.update(cx, |this: &mut Self, cx| {
                                        this.is_searching = false;
                                        this.search_results = results;
                                        this.scratchpad.push_str("\n\n### Found sources via Agent\n");
                                        for r in &this.search_results {
                                            this.scratchpad.push_str(&format!("- [{}]({}): {}\n", r.title, r.url, r.snippet));
                                        }
                                        cx.notify();
                                    });
                                });
                            }).detach();
                        }
                    }))
            );

        let mut results_list = v_flex().w_full().gap_3();
        if self.is_searching {
            results_list = results_list.child(
                h_flex().w_full().justify_center().py_8().child(
                    div().text_sm().text_color(theme.muted_foreground).child("Agents are gathering and synthesizing information...")
                )
            );
        } else if self.search_results.is_empty() {
            results_list = results_list.child(
                h_flex().w_full().justify_center().py_8().child(
                    div().text_sm().text_color(theme.muted_foreground).child("Click 'Auto-Search via Agent' to begin research.")
                )
            );
        } else {
            for result in &self.search_results {
                results_list = results_list.child(
                    v_flex()
                        .p_3()
                        .rounded_md()
                        .bg(theme.secondary)
                        .border_1()
                        .border_color(theme.border)
                        .child(div().text_sm().font_bold().text_color(theme.foreground).child(result.title.clone()))
                        .child(div().text_xs().text_color(gpui::green()).mt_1().child(result.url.clone()))
                        .child(div().text_xs().text_color(theme.muted_foreground).mt_2().child(result.snippet.clone()))
                );
            }
        }

        let left_pane = v_flex()
            .w_1_3()
            .h_full()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                v_flex().p_3().border_b_1().border_color(theme.border).bg(theme.secondary).child(
                    div().text_sm().font_bold().text_color(theme.foreground).child("Web Sources (Intake)")
                )
            )
            .child(
                div().flex_1().overflow_y_scrollbar().p_4().child(results_list)
            );

        let right_pane = v_flex()
            .w_2_3()
            .h_full()
            .bg(theme.background)
            .child(
                h_flex()
                    .justify_between()
                    .p_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .bg(theme.secondary)
                    .child(div().text_sm().font_bold().text_color(theme.foreground).child("Scratchpad (Synthesis)"))
                    .child(
                        h_flex().gap_3().items_center()
                            .child(div().text_xs().text_color(gpui::green()).child(self.save_status.clone().unwrap_or_default()))
                            .child(
                                Button::new("btn-save-obsidian")
                                    .label("Save to Obsidian")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let content = this.scratchpad.clone();
                                        let query = this.search_query.clone();
                                        this.save_status = Some("Saving...".to_string());
                                        cx.notify();
                                        let view = cx.entity().clone();
                                        let db = crate::AppState::global(cx).db.clone();
                                        let vault_path = std::env::var("AGENTFORGE_OBSIDIAN_VAULT")
                                            .ok()
                                            .or_else(|| db.get_setting("obsidian_vault_path").ok().flatten());
                                        cx.spawn(async move |_, cx| {
                                            let mut saved = false;
                                            if let Some(vault) = vault_path {
                                                let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
                                                let file_name = if query.trim().is_empty() {
                                                    format!("research_{}.md", ts)
                                                } else {
                                                    let mut s = query
                                                        .chars()
                                                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                                                        .collect::<String>();
                                                    while s.contains("__") {
                                                        s = s.replace("__", "_");
                                                    }
                                                    format!("research_{}_{}.md", ts, s.trim_matches('_'))
                                                };
                                                let path = std::path::PathBuf::from(vault)
                                                    .join("Research")
                                                    .join(file_name);
                                                if let Some(parent) = path.parent() {
                                                    let _ = std::fs::create_dir_all(parent);
                                                }
                                                if std::fs::write(&path, content.as_bytes()).is_ok() {
                                                    crate::infrastructure::fs::obsidian_adapter::sync_obsidian_file(&path, &*db).await;
                                                    saved = true;
                                                }
                                            } else {
                                                let item = crate::knowledge::core::KnowledgeItem::new(
                                                    if query.trim().is_empty() { "Research Notes" } else { query.trim() },
                                                    &content,
                                                    Vec::new(),
                                                    crate::knowledge::core::RetentionPolicy::KeepForever,
                                                );
                                                if db.upsert_knowledge_item(&item).is_ok() {
                                                    saved = true;
                                                }
                                            }

                                            let _ = cx.update(|cx| {
                                                let _ = view.update(cx, |this: &mut Self, cx| {
                                                    this.save_status = Some(if saved {
                                                        "Saved successfully!".to_string()
                                                    } else {
                                                        "Save failed.".to_string()
                                                    });
                                                    cx.notify();
                                                });
                                            });

                                            cx.background_executor().timer(std::time::Duration::from_secs(3)).await;
                                            let _ = cx.update(|cx| {
                                                let _ = view.update(cx, |this: &mut Self, cx| {
                                                    this.save_status = None;
                                                    cx.notify();
                                                });
                                            });
                                        })
                                        .detach();
                                    }))
                            )
                    )
            )
            .child(
                div().flex_1().overflow_y_scrollbar().p_4().child(
                    div().text_sm().text_color(theme.foreground).child(self.scratchpad.clone())
                )
            );

        v_flex()
            .size_full()
            .bg(theme.background)
            .child(header)
            .child(
                h_flex().w_full().flex_1().child(left_pane).child(right_pane)
            )
    }
}

impl EventEmitter<PanelEvent> for ResearchNotebookPanel {}
