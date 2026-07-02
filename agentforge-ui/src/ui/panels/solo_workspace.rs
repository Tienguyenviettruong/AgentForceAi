mod chat;
mod model;
mod sections;
mod sidebar;

use gpui::{App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window};
use gpui_component::{h_flex, input::InputState, ActiveTheme as _};

use self::model::{SoloConversation, SoloMessage, SoloSection};

pub struct SoloWorkspacePanel {
    prompt_input: Entity<InputState>,
    messages: Vec<SoloMessage>,
    conversations: Vec<SoloConversation>,
    active_conversation_id: Option<usize>,
    active_project_id: Option<&'static str>,
    active_section: SoloSection,
    next_conversation_id: usize,
    history_sidebar_open: bool,
}

impl SoloWorkspacePanel {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        let prompt_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(4)
                .soft_wrap(true)
                .placeholder("Ask anything...")
        });

        Self {
            prompt_input,
            messages: Vec::new(),
            conversations: Vec::new(),
            active_conversation_id: None,
            active_project_id: None,
            active_section: SoloSection::Chat,
            next_conversation_id: 1,
            history_sidebar_open: true,
        }
    }
}

impl Render for SoloWorkspacePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let content = match self.active_section {
            SoloSection::Chat => self.chat_content(cx),
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
