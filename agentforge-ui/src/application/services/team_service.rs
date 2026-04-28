use crate::core::models::*;
use crate::core::traits::database::DatabasePort;
use std::sync::Arc;

pub struct TeamService {
    db: Arc<dyn DatabasePort>,
}

impl TeamService {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self { db }
    }

    pub fn list_teams(&self) -> Result<Vec<Team>, crate::core::errors::CoreError> {
        self.db.list_teams().map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }

    pub fn get_team_agents(&self, team_id: &str) -> Result<Vec<Agent>, crate::core::errors::CoreError> {
        let agent_ids = self.db.get_team_agents(team_id).map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))?;
        let agents = agent_ids
            .into_iter()
            .filter_map(|id: String| self.db.get_agent(&id).ok().flatten())
            .collect();
        Ok(agents)
    }

    pub fn list_instances(&self) -> Result<Vec<Instance>, crate::core::errors::CoreError> {
        self.db.list_instances().map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }

    pub fn update_instance_name(&self, instance_id: &str, name: &str) -> Result<(), crate::core::errors::CoreError> {
        self.db
            .update_instance_name(instance_id, name)
            .map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }
    
    pub fn list_sessions_for_instance(&self, instance_id: &str) -> Result<Vec<SessionRecord>, crate::core::errors::CoreError> {
        self.db.list_sessions_for_instance(instance_id).map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }
    
    pub fn get_instance_agents(&self, instance_id: &str) -> Result<Vec<String>, crate::core::errors::CoreError> {
        self.db.get_instance_agents(instance_id).map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }
    
    pub fn create_session_for_instance(&self, instance_id: &str, agent_id: &str) -> Result<String, crate::core::errors::CoreError> {
        self.db.create_session_for_instance(instance_id, agent_id).map_err(|e| crate::core::errors::CoreError::Database(e.to_string()))
    }
}
