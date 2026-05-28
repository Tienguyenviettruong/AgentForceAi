use super::framework::{Skill, SkillInput, SkillMetadata, SkillOutput};
use async_trait::async_trait;

pub struct SummarizeSkill;

#[async_trait]
impl Skill for SummarizeSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "communication.summarize".to_string(),
            name: "Summarize".to_string(),
            description: "Condenses long text into key points.".to_string(),
            instructions: "Summarize the supplied material accurately and preserve decisions, risks and unresolved questions.".to_string(),
            version: "1.0".to_string(),
            category: "Communication".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let text = input.parameters.get("text").cloned().unwrap_or_default();
        let result = format!(
            "Summary of {} characters: It discusses important concepts.",
            text.len()
        );
        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}

pub struct TranslateSkill;

#[async_trait]
impl Skill for TranslateSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "communication.translate".to_string(),
            name: "Translate".to_string(),
            description: "Translates text between languages.".to_string(),
            instructions: "Translate faithfully while preserving technical identifiers, formatting and intended tone.".to_string(),
            version: "1.0".to_string(),
            category: "Communication".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let text = input.parameters.get("text").cloned().unwrap_or_default();
        let target_lang = input
            .parameters
            .get("target_lang")
            .cloned()
            .unwrap_or_else(|| "English".to_string());
        let result = format!(
            "Translated to {}: [Translation of {}]",
            target_lang,
            text.chars().take(10).collect::<String>()
        );
        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}

pub struct ExplainSkill;

#[async_trait]
impl Skill for ExplainSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "communication.explain".to_string(),
            name: "Explain".to_string(),
            description: "Explains complex concepts in simple terms.".to_string(),
            instructions: "Explain concepts precisely using the current context, separating verified behavior from proposals.".to_string(),
            version: "1.0".to_string(),
            category: "Communication".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let concept = input.parameters.get("concept").cloned().unwrap_or_default();
        let result = format!(
            "Explanation of '{}': It is a fundamental mechanism.",
            concept
        );
        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}
