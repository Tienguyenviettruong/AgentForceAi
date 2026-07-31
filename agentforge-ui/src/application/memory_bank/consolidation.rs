use crate::application::memory_bank::service::MemoryBankService;
use crate::application::services::knowledge_service::KnowledgeService;
use crate::core::models::memory_bank::*;
use crate::knowledge::core::{KnowledgeItem, RetentionPolicy, Tag};

const CONSOLIDATION_TOKEN_THRESHOLD: usize = 8000;
const STALE_DAYS_THRESHOLD: i64 = 30;

pub struct MemoryBankConsolidator;

impl MemoryBankConsolidator {
    /// Check if consolidation is needed and perform it.
    pub fn consolidate_if_needed(
        memory_service: &MemoryBankService,
        knowledge_service: &KnowledgeService,
        instance_id: &str,
    ) -> anyhow::Result<usize> {
        let total_tokens = memory_service.total_tokens(instance_id)?;
        if total_tokens < CONSOLIDATION_TOKEN_THRESHOLD {
            return Ok(0);
        }

        let items = memory_service.list_active_items(instance_id)?;
        let now = chrono::Utc::now();
        let mut consolidated_count = 0;

        for item in items {
            let age_days = now.signed_duration_since(item.updated_at).num_days();

            // Consolidate stale LessonsLearned and DecisionLog items
            let should_consolidate = match item.category {
                MemoryBankCategory::LessonsLearned | MemoryBankCategory::DecisionLog => {
                    age_days > STALE_DAYS_THRESHOLD
                }
                MemoryBankCategory::Progress => item.status == MemoryBankStatus::Archived,
                _ => false,
            };

            if should_consolidate {
                // Push to Knowledge Base
                let knowledge_item = KnowledgeItem::new(
                    &format!("[MemoryBank] {}", item.title),
                    &item.content,
                    vec![
                        Tag("memory_bank".to_string()),
                        Tag(item.category.as_str().to_string()),
                    ],
                    RetentionPolicy::KeepForever,
                );
                let _ = knowledge_service.upsert_knowledge_item(&knowledge_item);

                // Archive the Memory Bank item
                let mut archived = item.clone();
                archived.status = MemoryBankStatus::Archived;
                archived.updated_at = now;
                let _ = memory_service.update_item(&archived);

                consolidated_count += 1;
            }
        }

        Ok(consolidated_count)
    }
}
