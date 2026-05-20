use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct SharedResource {
    pub resource_id: String,
    pub owner_team: String,
    pub allowed_teams: HashSet<String>,
    pub data: String, // Or any generic resource representation
}

pub struct CoordinationCenter {
    resources: Arc<RwLock<HashMap<String, SharedResource>>>,
    knowledge_base: Arc<RwLock<HashMap<String, String>>>, // topic -> knowledge
}

impl Default for CoordinationCenter {
    fn default() -> Self {
        Self::new()
    }
}

impl CoordinationCenter {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            knowledge_base: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_resource(&self, resource: SharedResource) -> Result<()> {
        let mut resources = self.resources.write().unwrap();
        resources.insert(resource.resource_id.clone(), resource);
        Ok(())
    }

    pub fn access_resource(
        &self,
        resource_id: &str,
        requesting_team: &str,
    ) -> Result<SharedResource> {
        let resources = self.resources.read().unwrap();
        let resource = resources
            .get(resource_id)
            .ok_or_else(|| anyhow::anyhow!("Resource not found"))?;

        if resource.owner_team == requesting_team
            || resource.allowed_teams.contains(requesting_team)
        {
            Ok(resource.clone())
        } else {
            Err(anyhow::anyhow!("Access denied to resource"))
        }
    }

    pub fn share_knowledge(&self, topic: &str, content: &str) -> Result<()> {
        let mut kb = self.knowledge_base.write().unwrap();
        kb.insert(topic.to_string(), content.to_string());
        Ok(())
    }

    pub fn query_knowledge(&self, topic: &str) -> Option<String> {
        let kb = self.knowledge_base.read().unwrap();
        kb.get(topic).cloned()
    }
}
