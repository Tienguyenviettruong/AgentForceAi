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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KnowledgeRecordKind {
    Document,
    Memory,
    Artifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub instance_id: Option<String>,
    pub run_id: Option<String>,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: Uuid,
    pub record_kind: KnowledgeRecordKind,
    pub title: String,
    pub content: String,
    pub tags: Vec<Tag>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub retention_policy: RetentionPolicy,
    pub vault_path: Option<String>,
    pub source_kind: String,
    pub source_uri_normalized: Option<String>,
    pub content_hash: Option<String>,
    pub origin_run_id: Option<String>,
    pub origin_session_id: Option<String>,
    pub origin_instance_id: Option<String>,
    pub origin_agent_id: Option<String>,
}

impl KnowledgeItem {
    pub fn content_hash(content: &str) -> String {
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in content.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("fnv1a64:{hash:016x}")
    }

    pub fn normalize_file_source(path: &str) -> String {
        let normalized = path
            .trim()
            .replace('\\', "/")
            .trim_start_matches("//?/")
            .to_ascii_lowercase();
        if normalized.starts_with('/') {
            format!("file://{normalized}")
        } else {
            format!("file:///{normalized}")
        }
    }

    pub fn new(title: &str, content: &str, tags: Vec<Tag>, policy: RetentionPolicy) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            record_kind: KnowledgeRecordKind::Document,
            title: title.to_string(),
            content: content.to_string(),
            tags,
            created_at: now,
            updated_at: now,
            retention_policy: policy,
            vault_path: None,
            source_kind: "manual".to_string(),
            source_uri_normalized: None,
            content_hash: Some(Self::content_hash(content)),
            origin_run_id: None,
            origin_session_id: None,
            origin_instance_id: None,
            origin_agent_id: None,
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
