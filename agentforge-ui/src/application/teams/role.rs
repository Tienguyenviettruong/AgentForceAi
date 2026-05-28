use crate::core::traits::database::DatabasePort;
use anyhow::Result;
use std::sync::Arc;

pub struct Role {
    pub id: String,
    pub team_id: String,
    pub name: String,
    pub permissions: Option<String>,
    pub capabilities: Option<String>,
}

pub struct RoleManager {
    db: Arc<dyn DatabasePort>,
}

impl RoleManager {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self { db }
    }

    pub fn create_role(&self, role: &Role) -> Result<()> {
        self.db.create_role(role)
    }

    pub fn update_permissions(&self, role_id: &str, permissions: &str) -> Result<()> {
        self.db.update_role_permissions(role_id, permissions)
    }

    pub fn check_permission(&self, role_id: &str, required_permission: &str) -> Result<bool> {
        self.db.check_role_permission(role_id, required_permission)
    }

    pub fn load_roles(&self) -> Result<()> {
        // Routing roles are user-managed business data; security grants use security_roles.
        Ok(())
    }
}
