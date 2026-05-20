use crate::core::traits::database::DatabasePort;
use crate::teambus::routing::TeamMessage;
use anyhow::Result;
use std::sync::Arc;

pub struct MessageStore {
    db: Arc<dyn crate::core::traits::database::DatabasePort>,
}

impl MessageStore {
    pub fn new(db: Arc<dyn crate::core::traits::database::DatabasePort>) -> Self {
        Self { db }
    }

    pub fn save_message(&self, msg: &TeamMessage) -> Result<()> {
        self.db.insert_team_message(msg)
    }

    pub fn get_messages_for_team(
        &self,
        team_instance_id: &str,
        limit: u32,
    ) -> Result<Vec<TeamMessage>> {
        self.db
            .get_team_messages_for_instance(team_instance_id, limit)
    }

    pub fn update_delivery_status(&self, message_id: &str, status: &str) -> Result<()> {
        self.db
            .update_team_message_delivery_status(message_id, status)
    }
}
