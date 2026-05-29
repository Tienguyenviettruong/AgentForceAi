use crate::core::traits::database::DatabasePort;
use crate::knowledge::core::{KnowledgeItem, KnowledgeRecordKind, RetentionPolicy, Tag};
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;

pub struct KnowledgeService {
    db: Arc<dyn DatabasePort>,
}

impl KnowledgeService {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self { db }
    }

    pub fn get_all_knowledge_items(
        &self,
    ) -> Result<Vec<KnowledgeItem>, crate::core::errors::CoreError> {
        self.db
            .get_all_knowledge_items()
            .map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }

    pub fn get_all_records(&self) -> Result<Vec<KnowledgeItem>, crate::core::errors::CoreError> {
        let mut records = self.get_all_knowledge_items()?;
        let memories = self
            .db
            .get_all_knowledge_entries()
            .map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))?;
        records.extend(memories.into_iter().map(Self::memory_as_record));
        let mut seen_artifacts = HashSet::new();
        for run in self
            .db
            .list_recent_orchestration_runs(100)
            .unwrap_or_default()
        {
            for artifact in self.db.list_artifacts_for_run(&run.id).unwrap_or_default() {
                if seen_artifacts.insert(artifact.id.clone()) {
                    records.push(Self::artifact_as_record(artifact));
                }
            }
        }
        records.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(records)
    }

    fn memory_as_record(entry: crate::core::models::knowledge::KnowledgeEntry) -> KnowledgeItem {
        KnowledgeItem {
            id: uuid::Uuid::parse_str(&entry.id).unwrap_or_default(),
            record_kind: KnowledgeRecordKind::Memory,
            title: entry.title,
            content_hash: Some(KnowledgeItem::content_hash(&entry.content)),
            content: entry.content,
            tags: entry.tags.into_iter().map(Tag).collect(),
            created_at: entry.created_at,
            updated_at: entry.created_at,
            retention_policy: RetentionPolicy::KeepForever,
            vault_path: None,
            source_kind: "agent_memory".to_string(),
            source_uri_normalized: None,
            origin_run_id: entry.run_id,
            origin_session_id: entry.session_id,
            origin_instance_id: entry.instance_id,
            origin_agent_id: Some(entry.agent_id),
        }
    }

    fn artifact_as_record(artifact: crate::core::models::ArtifactRecord) -> KnowledgeItem {
        let created_at = DateTime::parse_from_rfc3339(&artifact.created_at)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let content = format!(
            "# Generated Artifact\n\nPath: {}\nKind: {}\nContent hash: {}\n",
            artifact.path, artifact.artifact_kind, artifact.content_hash
        );
        KnowledgeItem {
            id: uuid::Uuid::parse_str(&artifact.id).unwrap_or_default(),
            record_kind: KnowledgeRecordKind::Artifact,
            title: std::path::Path::new(&artifact.path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Generated artifact")
                .to_string(),
            content_hash: Some(artifact.content_hash),
            content,
            tags: vec![
                Tag("artifact".to_string()),
                Tag(artifact.artifact_kind.clone()),
            ],
            created_at,
            updated_at: created_at,
            retention_policy: RetentionPolicy::KeepForever,
            vault_path: Some(artifact.path),
            source_kind: "generated_artifact".to_string(),
            source_uri_normalized: None,
            origin_run_id: artifact.run_id,
            origin_session_id: artifact.session_id,
            origin_instance_id: Some(artifact.instance_id),
            origin_agent_id: artifact.agent_id,
        }
    }

    pub fn search_knowledge(
        &self,
        query: &str,
    ) -> Result<Vec<KnowledgeItem>, crate::core::errors::CoreError> {
        self.db
            .search_knowledge(query)
            .map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }

    pub fn search_knowledge_fts(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<KnowledgeItem>, crate::core::errors::CoreError> {
        self.db
            .search_knowledge_fts(query, limit)
            .map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }

    pub fn upsert_knowledge_item(
        &self,
        item: &KnowledgeItem,
    ) -> Result<(), crate::core::errors::CoreError> {
        self.db
            .upsert_knowledge_item(item)
            .map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }
}
