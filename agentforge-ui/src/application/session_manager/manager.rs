use super::ConcurrencyGuard;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents a conversation session context.
#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub agent_id: String,
    pub context: String,
    pub status: String,
}

/// SessionManagerV2 manages session lifecycles and recovery
/// (Task 1.28: Implement SessionManagerV2)
/// (Task 1.30: Implement session persistence and recovery)
pub struct SessionManagerV2 {
    db: Arc<dyn crate::core::traits::database::DatabasePort>,
    active_sessions: Arc<RwLock<HashMap<String, Session>>>,
    concurrency_guard: ConcurrencyGuard,
}

impl SessionManagerV2 {
    pub fn new(
        db: Arc<dyn crate::core::traits::database::DatabasePort>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            db,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            concurrency_guard: ConcurrencyGuard::new(max_concurrent),
        }
    }

    pub async fn create_session(&self, agent_id: &str) -> Result<String> {
        let _permit = self.concurrency_guard.acquire().await?;

        let session_id = uuid::Uuid::new_v4().to_string();
        let session = Session {
            id: session_id.clone(),
            agent_id: agent_id.to_string(),
            context: String::new(),
            status: "active".to_string(),
        };

        self.db
            .ensure_session(&session_id, agent_id, None)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut lock = self.active_sessions.write().await;
        lock.insert(session_id.clone(), session);

        Ok(session_id)
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let lock = self.active_sessions.read().await;
        lock.get(session_id).cloned()
    }

    pub async fn end_session(&self, session_id: &str) -> Result<()> {
        let mut lock = self.active_sessions.write().await;
        if let Some(mut session) = lock.remove(session_id) {
            session.status = "completed".to_string();
            self.db
                .touch_session(session_id)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            self.concurrency_guard.release();
        }
        Ok(())
    }

    pub async fn append_turn(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        metadata: Option<&str>,
    ) -> Result<()> {
        self.db
            .append_conversation_turn(session_id, role, content, metadata)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        self.db
            .touch_session(session_id)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }

    pub async fn get_turns(&self, session_id: &str) -> Result<Vec<crate::providers::ChatMessage>> {
        self.db
            .get_conversation_turns(session_id)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}
