use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub version: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub result: String,
    pub success: bool,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait Skill: Send + Sync {
    fn metadata(&self) -> SkillMetadata;
    async fn execute(&self, input: SkillInput) -> SkillOutput;
}

pub struct SkillRegistry {
    skills: RwLock<HashMap<String, Arc<dyn Skill>>>,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, skill: Arc<dyn Skill>) {
        let meta = skill.metadata();
        self.skills.write().await.insert(meta.id.clone(), skill);
    }

    pub async fn get_skill(&self, id: &str) -> Option<Arc<dyn Skill>> {
        self.skills.read().await.get(id).cloned()
    }

    pub async fn discover_skills(&self) -> Vec<SkillMetadata> {
        let mut discovered = Vec::new();
        for skill in self.skills.read().await.values() {
            discovered.push(skill.metadata());
        }
        discovered
    }

    pub async fn execute_skill(&self, id: &str, input: SkillInput) -> Result<SkillOutput, String> {
        let skill = self
            .get_skill(id)
            .await
            .ok_or_else(|| format!("Skill {} not found", id))?;
        Ok(skill.execute(input).await)
    }
}
