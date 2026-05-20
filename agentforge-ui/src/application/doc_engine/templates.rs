use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait Template: Send + Sync {
    fn name(&self) -> &str;
    fn render(&self, data: &serde_json::Value) -> Result<String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub name: String,
    pub field_type: String, // e.g., "string", "number", "boolean"
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrdTemplate {
    pub custom_fields: Vec<CustomField>,
}

impl Default for PrdTemplate {
    fn default() -> Self {
        Self {
            custom_fields: vec![
                CustomField {
                    name: "project_name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                },
                CustomField {
                    name: "target_audience".to_string(),
                    field_type: "string".to_string(),
                    required: false,
                },
            ],
        }
    }
}

impl Template for PrdTemplate {
    fn name(&self) -> &str {
        "PRD"
    }

    fn render(&self, data: &serde_json::Value) -> Result<String> {
        let project_name = data
            .get("project_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Project");
        let background = data
            .get("background")
            .and_then(|v| v.as_str())
            .unwrap_or("No background provided.");
        let features = data
            .get("features")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| format!("- {}", s))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| "No features listed.".to_string());
        let target_audience = data
            .get("target_audience")
            .and_then(|v| v.as_str())
            .unwrap_or("General");

        let content = format!(
            "# Product Requirements Document (PRD)\n\n\
            ## Project Name: {}\n\n\
            ### Target Audience\n{}\n\n\
            ### Background\n{}\n\n\
            ### Features\n{}\n",
            project_name, target_audience, background, features
        );

        Ok(content)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrsTemplate {
    pub custom_fields: Vec<CustomField>,
}

impl Default for SrsTemplate {
    fn default() -> Self {
        Self {
            custom_fields: vec![CustomField {
                name: "system_name".to_string(),
                field_type: "string".to_string(),
                required: true,
            }],
        }
    }
}

impl Template for SrsTemplate {
    fn name(&self) -> &str {
        "SRS"
    }

    fn render(&self, data: &serde_json::Value) -> Result<String> {
        let system_name = data
            .get("system_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown System");
        let scope = data
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or("No scope provided.");
        let requirements = data
            .get("requirements")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| format!("- {}", s))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| "No requirements listed.".to_string());

        let content = format!(
            "# Software Requirements Specification (SRS)\n\n\
            ## System Name: {}\n\n\
            ### Scope\n{}\n\n\
            ### Functional Requirements\n{}\n",
            system_name, scope, requirements
        );

        Ok(content)
    }
}

pub struct TemplateManager {
    templates: HashMap<String, Box<dyn Template>>,
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateManager {
    pub fn new() -> Self {
        let mut manager = Self {
            templates: HashMap::new(),
        };

        // Register default templates
        manager.register_template(Box::new(PrdTemplate::default()));
        manager.register_template(Box::new(SrsTemplate::default()));

        manager
    }

    pub fn register_template(&mut self, template: Box<dyn Template>) {
        self.templates.insert(template.name().to_string(), template);
    }

    pub fn get_template(&self, name: &str) -> Option<&dyn Template> {
        self.templates.get(name).map(|t| t.as_ref())
    }

    pub fn list_templates(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }
}
