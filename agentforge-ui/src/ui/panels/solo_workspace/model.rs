use gpui::Hsla;
use gpui_component::IconName;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub(super) struct SoloMessage {
    pub(super) role: &'static str,
    pub(super) content: String,
    pub(super) attachments: Vec<String>,
    pub(super) model: Option<String>,
    pub(super) speed: Option<String>,
    pub(super) tools: Vec<String>,
    pub(super) activities: Vec<SoloActivity>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct SoloMessageMetadata {
    pub(super) attachments: Vec<String>,
    pub(super) model: Option<String>,
    pub(super) speed: Option<String>,
    pub(super) tools: Vec<String>,
    pub(super) activities: Vec<SoloActivity>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum SoloActivityKind {
    Search,
    Tool,
    Command,
    File,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum SoloActivityStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct SoloActivity {
    pub(super) id: String,
    pub(super) kind: SoloActivityKind,
    pub(super) label: String,
    pub(super) detail: Option<String>,
    pub(super) status: SoloActivityStatus,
}

#[derive(Clone, Copy)]
pub(super) enum SoloTool {
    Build,
    Skills,
}

impl SoloTool {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Build => "Build",
            Self::Skills => "Skills",
        }
    }

    pub(super) fn icon(self) -> IconName {
        match self {
            Self::Build => IconName::SquareTerminal,
            Self::Skills => IconName::Settings2,
        }
    }
}

#[derive(Clone)]
pub(super) struct SoloTemplate {
    pub(super) title: &'static str,
    pub(super) detail: &'static str,
    pub(super) prompt: &'static str,
    pub(super) icon: IconName,
    pub(super) base: Hsla,
    pub(super) accent: Hsla,
}

#[derive(Clone)]
pub(super) struct SoloConversation {
    pub(super) id: usize,
    pub(super) project_id: Option<String>,
    pub(super) title: String,
    pub(super) messages: Vec<SoloMessage>,
}

#[derive(Clone)]
pub(super) struct SoloProject {
    pub(super) id: String,
    pub(super) name: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SoloSection {
    Chat,
    Memory,
    Automations,
    Triggers,
    Remote,
}

pub(super) fn solo_templates() -> Vec<SoloTemplate> {
    vec![
        SoloTemplate {
            title: "Plan",
            detail: "Break down personal work",
            prompt: "Create a personal execution plan for: ",
            icon: IconName::ChartPie,
            base: Hsla::from(gpui::rgb(0x2563eb)),
            accent: Hsla::from(gpui::rgb(0x06b6d4)),
        },
        SoloTemplate {
            title: "Research",
            detail: "Summarize a topic",
            prompt: "Research and summarize this topic for me: ",
            icon: IconName::Search,
            base: Hsla::from(gpui::rgb(0x7c3aed)),
            accent: Hsla::from(gpui::rgb(0xec4899)),
        },
        SoloTemplate {
            title: "Build",
            detail: "Code, test, fix",
            prompt: "Work on this coding task in the current project: ",
            icon: IconName::SquareTerminal,
            base: Hsla::from(gpui::rgb(0x059669)),
            accent: Hsla::from(gpui::rgb(0xf59e0b)),
        },
        SoloTemplate {
            title: "Remote",
            detail: "Guarded desktop control",
            prompt: "Use guarded remote control to help me with: ",
            icon: IconName::Eye,
            base: Hsla::from(gpui::rgb(0x475569)),
            accent: Hsla::from(gpui::rgb(0x8b5cf6)),
        },
    ]
}
