use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::docs::formats::FormatManager;
use crate::docs::templates::TemplateManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub default_format: String,
    pub output_dir: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            default_format: "markdown".to_string(),
            output_dir: "./outputs/docs".to_string(),
        }
    }
}

pub struct DocumentEngine {
    pub config: EngineConfig,
    pub template_manager: Arc<RwLock<TemplateManager>>,
    pub format_manager: Arc<RwLock<FormatManager>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRequest {
    pub template_name: String,
    pub content_data: serde_json::Value,
    pub format: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentResult {
    pub content: Vec<u8>,
    pub format: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl DocumentEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            template_manager: Arc::new(RwLock::new(TemplateManager::new())),
            format_manager: Arc::new(RwLock::new(FormatManager::new())),
        }
    }

    pub async fn generate_document(&self, request: DocumentRequest) -> Result<DocumentResult> {
        // 1. Get template
        let templates: tokio::sync::RwLockReadGuard<'_, TemplateManager> =
            self.template_manager.read().await;
        let template = templates
            .get_template(&request.template_name)
            .ok_or_else(|| anyhow::anyhow!("Template not found: {}", request.template_name))?;

        // 2. Render template
        let rendered_content = template.render(&request.content_data)?;

        // 3. Format output
        let format_name = request
            .format
            .unwrap_or_else(|| self.config.default_format.clone());
        let formats: tokio::sync::RwLockReadGuard<'_, FormatManager> =
            self.format_manager.read().await;
        let formatter = formats
            .get_formatter(&format_name)
            .ok_or_else(|| anyhow::anyhow!("Format not supported: {}", format_name))?;

        let final_content = formatter.format(&rendered_content)?;

        Ok(DocumentResult {
            content: final_content,
            format: format_name,
            generated_at: chrono::Utc::now(),
        })
    }
}
