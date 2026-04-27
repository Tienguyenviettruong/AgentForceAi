use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RetentionPolicy {
    KeepForever,
    ExpireAfterDays(u32),
    ArchiveAfterDays(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Tag(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub tags: Vec<Tag>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub retention_policy: RetentionPolicy,
    pub vault_path: Option<String>,
}

impl KnowledgeItem {
    pub fn new(title: &str, content: &str, tags: Vec<Tag>, policy: RetentionPolicy) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.to_string(),
            content: content.to_string(),
            tags,
            created_at: now,
            updated_at: now,
            retention_policy: policy,
            vault_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brain {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub items: HashMap<Uuid, KnowledgeItem>,
}

impl Brain {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            items: HashMap::new(),
        }
    }

    pub async fn add_item(&mut self, item: KnowledgeItem) -> Result<(), String> {
        self.items.insert(item.id, item);
        Ok(())
    }

    pub async fn get_item(&self, id: &Uuid) -> Option<KnowledgeItem> {
        self.items.get(id).cloned()
    }

    pub async fn update_item(&mut self, id: &Uuid, mut item: KnowledgeItem) -> Result<(), String> {
        if self.items.contains_key(id) {
            item.updated_at = Utc::now();
            self.items.insert(*id, item);
            Ok(())
        } else {
            Err("Item not found".to_string())
        }
    }

    pub async fn delete_item(&mut self, id: &Uuid) -> Result<(), String> {
        if self.items.remove(id).is_some() {
            Ok(())
        } else {
            Err("Item not found".to_string())
        }
    }

    pub async fn get_by_tag(&self, tag: &Tag) -> Vec<KnowledgeItem> {
        self.items
            .values()
            .filter(|item| item.tags.contains(tag))
            .cloned()
            .collect()
    }

    pub async fn apply_retention_policies(&mut self) -> Result<usize, String> {
        let now = Utc::now();
        let mut to_remove = Vec::new();

        for (id, item) in &self.items {
            match item.retention_policy {
                RetentionPolicy::KeepForever => {}
                RetentionPolicy::ExpireAfterDays(days) => {
                    let duration = now.signed_duration_since(item.created_at);
                    if duration.num_days() > days as i64 {
                        to_remove.push(*id);
                    }
                }
                RetentionPolicy::ArchiveAfterDays(days) => {
                    let duration = now.signed_duration_since(item.created_at);
                    if duration.num_days() > days as i64 {
                        to_remove.push(*id);
                    }
                }
            }
        }

        let removed_count = to_remove.len();
        for id in to_remove {
            self.items.remove(&id);
        }

        Ok(removed_count)
    }

    pub async fn export_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self).map_err(|e| e.to_string())
    }

    pub async fn import_json(data: &str) -> Result<Self, String> {
        serde_json::from_str(data).map_err(|e| e.to_string())
    }
}
