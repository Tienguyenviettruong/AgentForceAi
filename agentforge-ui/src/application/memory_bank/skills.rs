use crate::application::skills::framework::{Skill, SkillInput, SkillMetadata, SkillOutput};
use async_trait::async_trait;

pub struct UpdateMemoryBankSkill;

#[async_trait]
impl Skill for UpdateMemoryBankSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "memory_bank.update".to_string(),
            name: "Update Memory Bank".to_string(),
            description: "Updates the project's working memory with progress, decisions, or lessons learned.".to_string(),
            instructions: "After completing a significant task, update the Memory Bank with: progress changes, new decisions made, or lessons learned. Parameters: category (project_brief|system_architecture|active_context|progress|lessons_learned|decision_log), title, content.".to_string(),
            version: "1.0".to_string(),
            category: "MemoryBank".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let category = input
            .parameters
            .get("category")
            .cloned()
            .unwrap_or_else(|| "active_context".to_string());
        let title = input
            .parameters
            .get("title")
            .cloned()
            .unwrap_or_else(|| "Update".to_string());
        let content = input.parameters.get("content").cloned().unwrap_or_default();

        let result = format!(
            "Memory Bank update queued: [{}] {} — {} chars",
            category,
            title,
            content.len()
        );

        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}

pub struct ReadMemoryBankSkill;

#[async_trait]
impl Skill for ReadMemoryBankSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "memory_bank.read".to_string(),
            name: "Read Memory Bank".to_string(),
            description: "Reads the current state of the project's working memory.".to_string(),
            instructions: "Read the Memory Bank to understand project context. Parameters: category (optional, reads all if omitted).".to_string(),
            version: "1.0".to_string(),
            category: "MemoryBank".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let category = input.parameters.get("category").cloned();
        let result = format!(
            "Memory Bank read request: category={}",
            category.as_deref().unwrap_or("all")
        );

        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}
