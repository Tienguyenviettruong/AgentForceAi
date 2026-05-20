use crate::core::traits::database::DatabasePort;
use crate::knowledge::core::KnowledgeItem;
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
