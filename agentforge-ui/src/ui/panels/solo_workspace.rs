mod chat;
mod model;
mod remote;
mod remote_view;
mod sections;
mod sidebar;

use gpui::{
    px, AppContext, Context, Entity, IntoElement, ListAlignment, ListState, ParentElement, Render,
    SharedString, Styled, Subscription, Window,
};
use gpui_component::{
    h_flex,
    input::{InputEvent, InputState},
    select::SelectState,
    slider::{SliderEvent, SliderState},
    webview::WebView,
    ActiveTheme as _, IndexPath,
};

use self::model::{SoloConversation, SoloMessage, SoloMessageMetadata, SoloProject, SoloSection};
use std::collections::HashSet;

fn solo_model_options(cx: &gpui::App) -> Vec<SharedString> {
    let mut options = vec![SharedString::from("Auto")];
    if let Ok(providers) = crate::AppState::global(cx).db.list_providers() {
        for provider in providers {
            let label = format!("{} / {}", provider.provider_name, provider.model);
            if !options.iter().any(|item| item.as_ref() == label) {
                options.push(SharedString::from(label));
            }
        }
    }
    options
}

pub struct SoloWorkspacePanel {
    prompt_input: Entity<InputState>,
    webview_url_input: Entity<InputState>,
    messages: Vec<SoloMessage>,
    projects: Vec<SoloProject>,
    conversations: Vec<SoloConversation>,
    active_conversation_id: Option<usize>,
    active_project_id: Option<String>,
    active_section: SoloSection,
    next_conversation_id: usize,
    history_sidebar_open: bool,
    solo_chat_list_state: ListState,
    solo_is_thinking: bool,
    solo_response_generation: usize,
    solo_expanded_messages: HashSet<usize>,
    solo_expanded_activities: HashSet<(usize, usize)>,
    solo_webview_open: bool,
    solo_webview: Option<Entity<WebView>>,
    solo_webview_error: Option<String>,
    solo_webview_generation: usize,
    solo_webview_width: f32,
    solo_webview_resizing: bool,
    solo_attachments: Vec<String>,
    solo_model_select: Entity<SelectState<Vec<SharedString>>>,
    solo_effort_slider: Entity<SliderState>,
    solo_build_enabled: bool,
    solo_skills: Vec<crate::application::skills::SkillMetadata>,
    solo_selected_skill_ids: Vec<String>,
    solo_selected_mcp_tool_ids: Vec<String>,
    solo_mention_query: Option<String>,
    solo_mention_start: Option<usize>,
    solo_mention_selection_index: usize,
    remote: remote::RemoteState,
    _solo_subscriptions: Vec<Subscription>,
}

impl SoloWorkspacePanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let prompt_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(4)
                .soft_wrap(true)
                .placeholder("Ask anything...")
        });
        let webview_url_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Enter URL...");
            state.set_value("https://www.google.com", window, cx);
            state
        });
        let model_options = solo_model_options(cx);
        let solo_model_select = cx.new(|cx| {
            SelectState::new(model_options, Some(IndexPath::new(0)), window, cx).searchable(true)
        });
        let solo_effort_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(4.0)
                .step(1.0)
                .default_value(1.0)
        });
        let effort_subscription = cx.subscribe(
            &solo_effort_slider,
            |_this, _slider, event: &SliderEvent, cx| {
                if matches!(event, SliderEvent::Change(_)) {
                    cx.notify();
                }
            },
        );
        let db = crate::AppState::global(cx).db.clone();
        let projects = db
            .list_solo_projects()
            .unwrap_or_default()
            .into_iter()
            .map(|project| SoloProject {
                id: project.id,
                name: project.name,
            })
            .collect::<Vec<_>>();
        let conversations = db
            .list_solo_conversations()
            .unwrap_or_default()
            .into_iter()
            .map(|conversation| {
                let messages = db
                    .get_solo_messages(conversation.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|message| {
                        let metadata = message
                            .metadata
                            .as_deref()
                            .and_then(|value| {
                                serde_json::from_str::<SoloMessageMetadata>(value).ok()
                            })
                            .unwrap_or_default();
                        SoloMessage {
                            role: match message.role.as_str() {
                                "assistant" => "assistant",
                                "system" => "system",
                                _ => "user",
                            },
                            content: message.content,
                            attachments: metadata.attachments,
                            model: metadata.model,
                            speed: metadata.speed,
                            tools: metadata.tools,
                            activities: metadata.activities,
                        }
                    })
                    .collect();
                SoloConversation {
                    id: conversation.id.max(0) as usize,
                    project_id: conversation.project_id,
                    title: conversation.title,
                    messages,
                }
            })
            .collect::<Vec<_>>();
        let next_conversation_id = conversations
            .iter()
            .map(|conversation| conversation.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let mut panel = Self {
            prompt_input,
            webview_url_input,
            messages: Vec::new(),
            projects,
            conversations,
            active_conversation_id: None,
            active_project_id: None,
            active_section: SoloSection::Chat,
            next_conversation_id,
            history_sidebar_open: true,
            solo_chat_list_state: ListState::new(0, ListAlignment::Bottom, px(200.)),
            solo_is_thinking: false,
            solo_response_generation: 0,
            solo_expanded_messages: HashSet::new(),
            solo_expanded_activities: HashSet::new(),
            solo_webview_open: false,
            solo_webview: None,
            solo_webview_error: None,
            solo_webview_generation: 0,
            solo_webview_width: 720.0,
            solo_webview_resizing: false,
            solo_attachments: Vec::new(),
            solo_model_select,
            solo_effort_slider,
            solo_build_enabled: true,
            solo_skills: crate::application::skills::builtin_skill_catalog(),
            solo_selected_skill_ids: Vec::new(),
            solo_selected_mcp_tool_ids: Vec::new(),
            solo_mention_query: None,
            solo_mention_start: None,
            solo_mention_selection_index: 0,
            remote: remote::RemoteState::default(),
            _solo_subscriptions: vec![effort_subscription],
        };
        let prompt_subscription = cx.subscribe_in(
            &panel.prompt_input,
            window,
            |this, _, event: &InputEvent, _window, cx| match event {
                InputEvent::Change => this.sync_solo_mention_query(cx),
                InputEvent::Blur => {
                    if this.solo_mention_query.is_some() {
                        cx.notify();
                    }
                }
                _ => {}
            },
        );
        panel._solo_subscriptions.push(prompt_subscription);
        let providers_revision = crate::AppState::global(cx).providers_revision.clone();
        let providers_subscription = cx.observe_in(
            &providers_revision,
            window,
            |this, _revision, window, cx| {
                this.refresh_solo_model_options(window, cx);
            },
        );
        panel._solo_subscriptions.push(providers_subscription);
        panel
    }

    fn refresh_solo_model_options(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let options = solo_model_options(cx);
        let current_selection = self
            .solo_model_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| SharedString::from("Auto"));
        let next_selection = options
            .iter()
            .find(|option| option.as_ref() == current_selection.as_ref())
            .cloned()
            .unwrap_or_else(|| SharedString::from("Auto"));

        self.solo_model_select.update(cx, |state, cx| {
            state.set_items(options, window, cx);
            state.set_selected_value(&next_selection, window, cx);
        });
        cx.notify();
    }
}

impl Render for SoloWorkspacePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let content = match self.active_section {
            SoloSection::Chat => self.chat_content(window, cx),
            SoloSection::Memory => self.memory_page(cx),
            SoloSection::Automations => self.automations_page(cx),
            SoloSection::Triggers => self.triggers_page(cx),
            SoloSection::Remote => self.remote_page(cx),
        };

        let mut root = h_flex()
            .size_full()
            .bg(theme.background)
            .overflow_hidden()
            .child(self.primary_sidebar(cx));

        if self.active_section == SoloSection::Chat && self.history_sidebar_open {
            root = root.child(self.history_sidebar(cx));
        }

        root.child(content)
    }
}
