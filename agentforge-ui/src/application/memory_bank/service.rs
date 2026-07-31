use crate::core::models::memory_bank::*;
use crate::core::traits::database::DatabasePort;
use std::sync::Arc;

pub struct MemoryBankService {
    db: Arc<dyn DatabasePort>,
}

impl MemoryBankService {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self { db }
    }

    pub fn create_item(&self, item: &MemoryBankItem) -> anyhow::Result<()> {
        self.db.create_memory_bank_item(item)
    }

    pub fn update_item(&self, item: &MemoryBankItem) -> anyhow::Result<()> {
        self.db.update_memory_bank_item(item)
    }

    pub fn delete_item(&self, id: &str) -> anyhow::Result<()> {
        self.db.delete_memory_bank_item(id)
    }

    pub fn get_item(&self, id: &str) -> anyhow::Result<Option<MemoryBankItem>> {
        self.db.get_memory_bank_item(id)
    }

    pub fn list_items(&self, instance_id: &str) -> anyhow::Result<Vec<MemoryBankItem>> {
        self.db.list_memory_bank_items(instance_id)
    }

    pub fn list_items_by_category(
        &self,
        instance_id: &str,
        category: &str,
    ) -> anyhow::Result<Vec<MemoryBankItem>> {
        self.db
            .list_memory_bank_items_by_category(instance_id, category)
    }

    pub fn list_active_items(&self, instance_id: &str) -> anyhow::Result<Vec<MemoryBankItem>> {
        self.db.list_active_memory_bank_items(instance_id)
    }

    pub fn update_position(&self, id: &str, x: f32, y: f32) -> anyhow::Result<()> {
        self.db.update_memory_bank_item_position(id, x, y)
    }

    pub fn create_link(&self, link: &MemoryBankLink) -> anyhow::Result<()> {
        self.db.create_memory_bank_link(link)
    }

    pub fn delete_link(&self, id: &str) -> anyhow::Result<()> {
        self.db.delete_memory_bank_link(id)
    }

    pub fn list_links(&self, instance_id: &str) -> anyhow::Result<Vec<MemoryBankLink>> {
        self.db.list_memory_bank_links(instance_id)
    }

    pub fn create_snapshot(&self, snapshot: &MemoryBankSnapshot) -> anyhow::Result<()> {
        self.db.create_memory_bank_snapshot(snapshot)
    }

    pub fn list_snapshots(&self, instance_id: &str) -> anyhow::Result<Vec<MemoryBankSnapshot>> {
        self.db.list_memory_bank_snapshots(instance_id)
    }

    /// Build context string for LLM injection, prioritized by ContextPriority.
    /// Returns formatted markdown suitable for system prompt injection.
    pub fn build_context_for_instance(
        &self,
        instance_id: &str,
        max_tokens: usize,
    ) -> anyhow::Result<String> {
        let items = self.db.list_active_memory_bank_items(instance_id)?;
        if items.is_empty() {
            return Ok(String::new());
        }

        // Sort by priority weight descending
        let mut sorted_items = items;
        sorted_items.sort_by(|a, b| {
            b.context_priority
                .weight()
                .cmp(&a.context_priority.weight())
        });

        let mut result = String::from("## 📋 Working Memory (Memory Bank)\n\n");
        let mut current_tokens: usize = 20; // header tokens

        for item in &sorted_items {
            let entry = format!(
                "### {} — {}\n{}\n\n",
                item.category.display_name(),
                item.title,
                item.content
            );
            let entry_tokens = MemoryBankItem::estimate_tokens(&entry);
            if current_tokens + entry_tokens > max_tokens {
                break;
            }
            result.push_str(&entry);
            current_tokens += entry_tokens;
        }

        Ok(result)
    }

    /// Auto-update progress after a task completes.
    pub fn auto_update_progress(
        &self,
        instance_id: &str,
        summary: &str,
        created_by: &str,
    ) -> anyhow::Result<()> {
        // Find or create the Progress item
        let progress_items = self
            .db
            .list_memory_bank_items_by_category(instance_id, "progress")?;
        if let Some(mut item) = progress_items.into_iter().next() {
            item.content = format!("{}\n- {}", item.content, summary);
            item.updated_at = chrono::Utc::now();
            item.content_hash = MemoryBankItem::compute_hash(&item.content);
            item.token_count = MemoryBankItem::estimate_tokens(&item.content);
            self.db.update_memory_bank_item(&item)?;
        } else {
            let item = MemoryBankItem::new(
                instance_id,
                MemoryBankCategory::Progress,
                "Task Progress",
                &format!("- {}", summary),
                created_by,
            );
            self.db.create_memory_bank_item(&item)?;
        }
        Ok(())
    }

    /// Initialize default Memory Bank items for a new project/instance.
    pub fn initialize_for_instance(&self, instance_id: &str) -> anyhow::Result<()> {
        let existing = self.db.list_memory_bank_items(instance_id)?;
        if !existing.is_empty() {
            return Ok(()); // Already initialized
        }

        let defaults = [
            (MemoryBankCategory::ProjectBrief, "Project Brief", "# Project Brief\n\nDescribe the project goals, requirements, and scope here."),
            (MemoryBankCategory::SystemArchitecture, "System Architecture", "# System Architecture\n\nDocument the tech stack, coding conventions, and architectural decisions."),
            (MemoryBankCategory::ActiveContext, "Active Context", "# Active Context\n\nWhat is being worked on right now? Current focus and immediate next steps."),
            (MemoryBankCategory::Progress, "Progress", "# Progress\n\n- [ ] Initial setup"),
            (MemoryBankCategory::LessonsLearned, "Lessons Learned", "# Lessons Learned\n\nRecord mistakes, insights, and patterns discovered during the project."),
            (MemoryBankCategory::DecisionLog, "Decision Log", "# Decision Log\n\nTrack key design and architecture decisions with rationale."),
        ];

        for (i, (category, title, content)) in defaults.iter().enumerate() {
            let mut item = MemoryBankItem::new(instance_id, *category, title, content, "system");
            // Spread items in a circle for initial canvas layout
            let angle = (i as f32) * std::f32::consts::TAU / defaults.len() as f32;
            item.position_x = 400.0 + angle.cos() * 200.0;
            item.position_y = 300.0 + angle.sin() * 150.0;
            self.db.create_memory_bank_item(&item)?;
        }

        Ok(())
    }

    /// Get total token count across all active items for an instance.
    pub fn total_tokens(&self, instance_id: &str) -> anyhow::Result<usize> {
        let items = self.db.list_active_memory_bank_items(instance_id)?;
        Ok(items.iter().map(|i| i.token_count).sum())
    }

    /// Get item count per category for dashboard display.
    pub fn category_summary(
        &self,
        instance_id: &str,
    ) -> anyhow::Result<Vec<(MemoryBankCategory, usize)>> {
        let items = self.db.list_memory_bank_items(instance_id)?;
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for item in &items {
            *counts
                .entry(item.category.as_str().to_string())
                .or_insert(0) += 1;
        }
        let result = MemoryBankCategory::all()
            .iter()
            .map(|cat| (*cat, *counts.get(cat.as_str()).unwrap_or(&0)))
            .collect();
        Ok(result)
    }
}
