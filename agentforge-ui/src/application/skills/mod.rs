pub mod communication;
pub mod core;
pub mod framework;
pub mod research;

pub use communication::*;
pub use core::*;
pub use framework::*;
pub use research::*;

use std::sync::Arc;

pub fn builtin_skill_catalog() -> Vec<SkillMetadata> {
    vec![
        CodeReviewSkill.metadata(),
        CodeGenerateSkill.metadata(),
        CodeDebugSkill.metadata(),
        WebSearchSkill.metadata(),
        DocumentAnalysisSkill.metadata(),
        DataExtractionSkill.metadata(),
        SummarizeSkill.metadata(),
        TranslateSkill.metadata(),
        ExplainSkill.metadata(),
    ]
}

pub async fn initialize_skills() -> SkillRegistry {
    let registry = SkillRegistry::new();

    // Core Skills
    registry.register(Arc::new(CodeReviewSkill)).await;
    registry.register(Arc::new(CodeGenerateSkill)).await;
    registry.register(Arc::new(CodeDebugSkill)).await;

    // Research Skills
    registry.register(Arc::new(WebSearchSkill)).await;
    registry.register(Arc::new(DocumentAnalysisSkill)).await;
    registry.register(Arc::new(DataExtractionSkill)).await;

    // Communication Skills
    registry.register(Arc::new(SummarizeSkill)).await;
    registry.register(Arc::new(TranslateSkill)).await;
    registry.register(Arc::new(ExplainSkill)).await;

    registry
}
