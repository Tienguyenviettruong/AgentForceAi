#[derive(Clone, Debug)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub system_prompt: Option<String>,
    pub config: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Agent {
    fn config_value(&self, key: &str) -> Option<String> {
        let config = self.config.as_deref()?;
        let value = serde_json::from_str::<serde_json::Value>(config).ok()?;
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned)
    }

    pub fn routing_role(&self) -> String {
        self.config_value("role")
            .unwrap_or_else(|| self.name.clone())
    }

    pub fn profile_position(&self) -> String {
        self.config_value("position")
            .or_else(|| self.config_value("role"))
            .unwrap_or_else(|| self.name.clone())
    }

    pub fn profile_details(&self) -> Option<String> {
        self.config_value("details")
    }
}
