use super::identity::{AgentIdentity, VisualIdentity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub base_prompt: String,
    pub default_identity: AgentIdentity,
    pub visual_identity: VisualIdentity,
}

impl AgentTemplate {
    pub fn predefined_archetypes() -> Vec<Self> {
        vec![
            Self {
                id: "developer".to_string(),
                name: "Software Developer".to_string(),
                description: "Expert in writing, reviewing, and debugging code.".to_string(),
                base_prompt: "You are an expert software developer.".to_string(),
                default_identity: AgentIdentity::new("Logical, concise, and helpful."),
                visual_identity: VisualIdentity {
                    avatar_url: None,
                    theme_color: Some("#00FF00".to_string()),
                    icon: Some("code".to_string()),
                },
            },
            Self {
                id: "researcher".to_string(),
                name: "Research Assistant".to_string(),
                description: "Specializes in gathering and summarizing information.".to_string(),
                base_prompt: "You are a meticulous researcher.".to_string(),
                default_identity: AgentIdentity::new("Curious, detailed, and objective."),
                visual_identity: VisualIdentity {
                    avatar_url: None,
                    theme_color: Some("#0000FF".to_string()),
                    icon: Some("search".to_string()),
                },
            },
            Self {
                id: "reviewer".to_string(),
                name: "Code Reviewer".to_string(),
                description: "Analyzes code for bugs, style issues, and performance.".to_string(),
                base_prompt: "You are a strict but fair code reviewer.".to_string(),
                default_identity: AgentIdentity::new("Detail-oriented and communicative."),
                visual_identity: VisualIdentity {
                    avatar_url: None,
                    theme_color: Some("#FF0000".to_string()),
                    icon: Some("check".to_string()),
                },
            },
        ]
    }
}
