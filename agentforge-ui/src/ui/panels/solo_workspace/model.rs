use gpui::Hsla;
use gpui_component::IconName;

#[derive(Clone)]
pub(super) struct SoloMessage {
    pub(super) role: &'static str,
    pub(super) content: String,
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
    pub(super) project_id: Option<&'static str>,
    pub(super) title: String,
    pub(super) messages: Vec<SoloMessage>,
}

#[derive(Clone, Copy)]
pub(super) struct SoloProject {
    pub(super) id: &'static str,
    pub(super) name: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SoloSection {
    Chat,
    Memory,
    Automations,
    Triggers,
    Remote,
}

pub(super) fn solo_projects() -> [SoloProject; 3] {
    [
        SoloProject {
            id: "agentforce-ui",
            name: "AgentForce UI",
        },
        SoloProject {
            id: "research-notes",
            name: "Research Notes",
        },
        SoloProject {
            id: "personal-ops",
            name: "Personal Ops",
        },
    ]
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
