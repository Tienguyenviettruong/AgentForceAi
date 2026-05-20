use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditAction {
    pub agent_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub field_path: String,
    pub new_value: serde_json::Value,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSession {
    pub session_id: String,
    pub document_state: serde_json::Value,
    pub edit_history: Vec<EditAction>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Review,
    Completed,
}

pub struct CollaborationManager {
    sessions: RwLock<HashMap<String, Arc<RwLock<DocumentSession>>>>,
}

impl Default for CollaborationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CollaborationManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_session(&self, initial_state: serde_json::Value) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = DocumentSession {
            session_id: session_id.clone(),
            document_state: initial_state,
            edit_history: Vec::new(),
            status: SessionStatus::Active,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), Arc::new(RwLock::new(session)));

        session_id
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Arc<RwLock<DocumentSession>>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    pub async fn apply_edit(&self, session_id: &str, action: EditAction) -> Result<()> {
        let session_arc = self
            .get_session(session_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let mut session = session_arc.write().await;

        if session.status != SessionStatus::Active {
            return Err(anyhow::anyhow!("Session is not active"));
        }

        // Apply edit to the JSON document state
        // For simplicity, we assume field_path is a top-level key.
        // In a full implementation, field_path could be a JSON Pointer.
        if let Some(obj) = session.document_state.as_object_mut() {
            obj.insert(action.field_path.clone(), action.new_value.clone());
        } else {
            return Err(anyhow::anyhow!("Document state is not a JSON object"));
        }

        session.edit_history.push(action);

        Ok(())
    }

    pub async fn update_status(&self, session_id: &str, status: SessionStatus) -> Result<()> {
        let session_arc = self
            .get_session(session_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let mut session = session_arc.write().await;
        session.status = status;

        Ok(())
    }
}
