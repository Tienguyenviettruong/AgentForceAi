use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub id: Uuid,
    pub name: String,
    pub steps: Vec<String>,
    pub required_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisTask {
    pub id: Uuid,
    pub raw_data: Vec<String>,
    pub template_id: Option<Uuid>,
    pub summary: Option<String>,
}

impl SynthesisTask {
    pub fn new(raw_data: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            raw_data,
            template_id: None,
            summary: None,
        }
    }

    pub fn assign_template(&mut self, template: &WorkflowTemplate) {
        self.template_id = Some(template.id);
    }

    pub async fn synthesize(&mut self) -> Result<String, String> {
        if self.raw_data.is_empty() {
            return Err("No data to synthesize".to_string());
        }

        // Mock synthesis process
        let synthesized_text = format!(
            "Synthesized {} distinct pieces of information into a cohesive summary.",
            self.raw_data.len()
        );
        self.summary = Some(synthesized_text.clone());
        Ok(synthesized_text)
    }
}
