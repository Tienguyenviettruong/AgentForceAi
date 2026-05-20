use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub user_id: Option<String>,
    pub resource: String,
    pub details: String,
}

use crate::core::traits::database::DatabasePort;
use std::sync::Arc;

pub struct AuditLogger {
    events: Vec<AuditEvent>,
    db: Arc<dyn DatabasePort>,
}

impl AuditLogger {
    /// Initialize a new Audit Logger asynchronously.
    pub async fn new(db: Arc<dyn DatabasePort>) -> Result<Self> {
        // In a real application, this would load previous events from a secure datastore
        Ok(Self {
            events: Vec::new(),
            db,
        })
    }

    /// Log a security or system event asynchronously.
    pub async fn log(
        &mut self,
        action: &str,
        user_id: Option<&str>,
        resource: &str,
        details: &str,
    ) -> Result<()> {
        let event = AuditEvent {
            timestamp: Utc::now(),
            action: action.to_string(),
            user_id: user_id.map(|s| s.to_string()),
            resource: resource.to_string(),
            details: details.to_string(),
        };

        self.events.push(event.clone());

        // Persist the event to a datastore asynchronously
        self.persist_event(&event).await?;

        Ok(())
    }

    /// Fetch all audit events asynchronously.
    pub async fn get_events(&self) -> Result<Vec<AuditEvent>> {
        Ok(self.events.clone())
    }

    /// Persist the latest event to storage (stubbed for now).
    async fn persist_event(&self, event: &AuditEvent) -> Result<()> {
        let db = self.db.clone();
        let ev = event.clone();

        // Use tokio::task::spawn_blocking to run the synchronous SQLite operation
        tokio::task::spawn_blocking(move || db.insert_audit_log(&ev)).await??;

        Ok(())
    }
}
