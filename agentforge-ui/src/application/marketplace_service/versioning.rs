use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub release_notes: String,
    pub release_date: String,
    pub is_yanked: bool,
}

pub struct VersionManager {
    tool_versions: HashMap<String, Vec<VersionInfo>>,
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            tool_versions: HashMap::new(),
        }
    }

    pub fn add_version(&mut self, tool_id: &str, version: VersionInfo) {
        self.tool_versions
            .entry(tool_id.to_string())
            .or_default()
            .push(version);
    }

    pub fn get_versions(&self, tool_id: &str) -> Result<Vec<VersionInfo>> {
        self.tool_versions
            .get(tool_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No versions found for tool"))
    }

    pub fn get_latest_version(&self, tool_id: &str) -> Result<VersionInfo> {
        let versions = self.get_versions(tool_id)?;
        versions
            .into_iter()
            .rfind(|v| !v.is_yanked)
            .ok_or_else(|| anyhow::anyhow!("No valid versions available"))
    }
}
