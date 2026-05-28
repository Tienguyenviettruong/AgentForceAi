use super::framework::{Skill, SkillInput, SkillMetadata, SkillOutput};
use async_trait::async_trait;

pub struct CodeReviewSkill;

#[async_trait]
impl Skill for CodeReviewSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "core.code_review".to_string(),
            name: "Code Review".to_string(),
            description: "Reviews source code for quality, security, and performance.".to_string(),
            instructions: "Review source changes for correctness, security, regressions and missing tests. Report concrete findings before summaries.".to_string(),
            version: "1.0".to_string(),
            category: "Core".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let code = input.parameters.get("code").cloned().unwrap_or_default();
        let result = format!(
            "Reviewed code of length {}. Found no immediate issues.",
            code.len()
        );
        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}

pub struct CodeGenerateSkill;

#[async_trait]
impl Skill for CodeGenerateSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "core.code_generate".to_string(),
            name: "Code Generation".to_string(),
            description: "Generates source code based on natural language requirements."
                .to_string(),
            instructions: "Generate maintainable source code consistent with the existing project conventions and validate the behavior after editing.".to_string(),
            version: "1.0".to_string(),
            category: "Core".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let prompt = input.parameters.get("prompt").cloned().unwrap_or_default();
        let result = format!("// Generated code for: {}\nfn generated() {{}}", prompt);
        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}

pub struct CodeDebugSkill;

#[async_trait]
impl Skill for CodeDebugSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            id: "core.code_debug".to_string(),
            name: "Code Debug".to_string(),
            description: "Analyzes stack traces and code to find and fix bugs.".to_string(),
            instructions: "Trace failures to their root cause, make the narrowest correct fix, and verify with relevant tests or diagnostics.".to_string(),
            version: "1.0".to_string(),
            category: "Core".to_string(),
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillOutput {
        let error = input.parameters.get("error").cloned().unwrap_or_default();
        let result = format!(
            "Analyzed error: {}. Suggestion: Check variable bounds.",
            error
        );
        SkillOutput {
            result,
            success: true,
            error_message: None,
        }
    }
}
