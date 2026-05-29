use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    pub id: String,
    pub max_tokens: usize,
    pub current_tokens: usize,
    pub messages: Vec<ContextMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tokens: usize,
}

impl ContextWindow {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            max_tokens,
            current_tokens: 0,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str, tokens: usize) -> Result<(), String> {
        if tokens > self.max_tokens {
            return Err("Message tokens exceed maximum context window size".to_string());
        }

        if self.current_tokens + tokens > self.max_tokens {
            self.evict_oldest_messages(tokens);
        }

        let msg = ContextMessage {
            id: Uuid::new_v4().to_string(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            tokens,
        };

        self.messages.push(msg);
        self.current_tokens += tokens;

        Ok(())
    }

    fn evict_oldest_messages(&mut self, required_tokens: usize) {
        while self.current_tokens + required_tokens > self.max_tokens && !self.messages.is_empty() {
            let removed = self.messages.remove(0);
            self.current_tokens -= removed.tokens;
        }
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.current_tokens = 0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub format: String, // E.g., Markdown, JSON
    pub structure: Vec<String>, // Keys or sections
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Briefing {
    pub id: String,
    pub template_id: String,
    pub context_window_id: String,
    pub data: HashMap<String, String>,
    pub generated_at: DateTime<Utc>,
}

pub struct BriefingManager {
    pub templates: HashMap<String, BriefingTemplate>,
    pub active_briefings: HashMap<String, Briefing>,
    pub context_windows: HashMap<String, ContextWindow>,
}

impl Default for BriefingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BriefingManager {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            active_briefings: HashMap::new(),
            context_windows: HashMap::new(),
        }
    }

    pub fn register_template(&mut self, template: BriefingTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    pub fn create_context_window(&mut self, max_tokens: usize) -> String {
        let cw = ContextWindow::new(max_tokens);
        let id = cw.id.clone();
        self.context_windows.insert(id.clone(), cw);
        id
    }

    pub fn generate_briefing(
        &mut self,
        template_id: &str,
        context_window_id: &str,
        data: HashMap<String, String>,
    ) -> Result<Briefing, String> {
        let template = self.templates.get(template_id).ok_or("Template not found")?;
        let _cw = self.context_windows.get(context_window_id).ok_or("Context Window not found")?;

        // Validate data against template structure
        for section in &template.structure {
            if !data.contains_key(section) {
                return Err(format!("Missing section: {}", section));
            }
        }

        let briefing = Briefing {
            id: Uuid::new_v4().to_string(),
            template_id: template_id.to_string(),
            context_window_id: context_window_id.to_string(),
            data,
            generated_at: Utc::now(),
        };

        self.active_briefings.insert(briefing.id.clone(), briefing.clone());
        Ok(briefing)
    }

    pub fn update_context(
        &mut self,
        context_window_id: &str,
        role: &str,
        content: &str,
        tokens: usize,
    ) -> Result<(), String> {
        let cw = self.context_windows.get_mut(context_window_id).ok_or("Context Window not found")?;
        cw.add_message(role, content, tokens)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_window_eviction() {
        let mut cw = ContextWindow::new(100);
        cw.add_message("user", "msg1", 40).unwrap();
        cw.add_message("assistant", "msg2", 40).unwrap();
        assert_eq!(cw.current_tokens, 80);
        assert_eq!(cw.messages.len(), 2);

        // This should evict msg1
        cw.add_message("user", "msg3", 40).unwrap();
        assert_eq!(cw.current_tokens, 80);
        assert_eq!(cw.messages.len(), 2);
        assert_eq!(cw.messages[0].content, "msg2");
        assert_eq!(cw.messages[1].content, "msg3");
    }

    #[test]
    fn test_briefing_generation() {
        let mut manager = BriefingManager::new();

        let template = BriefingTemplate {
            id: "t1".to_string(),
            name: "Task Briefing".to_string(),
            description: "Standard task briefing".to_string(),
            format: "Markdown".to_string(),
            structure: vec!["Objective".to_string(), "Constraints".to_string()],
        };
        manager.register_template(template);

        let cw_id = manager.create_context_window(1000);

        let mut data = HashMap::new();
        data.insert("Objective".to_string(), "Fix bug".to_string());
        data.insert("Constraints".to_string(), "No downtime".to_string());

        let briefing = manager.generate_briefing("t1", &cw_id, data).unwrap();
        assert_eq!(briefing.template_id, "t1");
        assert_eq!(briefing.context_window_id, cw_id);
    }
}
