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

    fn config_string_array(&self, key: &str) -> Vec<String> {
        let Some(config) = self.config.as_deref() else {
            return Vec::new();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(config) else {
            return Vec::new();
        };
        let Some(raw) = value.get(key) else {
            return Vec::new();
        };
        match raw {
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            serde_json::Value::String(text) => text
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            _ => Vec::new(),
        }
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

    pub fn responsibilities(&self) -> Vec<String> {
        self.config_string_array("responsibilities")
    }

    pub fn competencies(&self) -> Vec<String> {
        self.config_string_array("competencies")
    }

    pub fn allowed_task_types(&self) -> Vec<String> {
        self.config_string_array("allowed_task_types")
    }

    pub fn disallowed_task_types(&self) -> Vec<String> {
        self.config_string_array("disallowed_task_types")
    }

    pub fn profile_for_prompt(&self) -> String {
        let responsibilities = self.responsibilities();
        let competencies = self.competencies();
        let allowed = self.allowed_task_types();
        let disallowed = self.disallowed_task_types();
        let details = self
            .profile_details()
            .unwrap_or_else(|| "not configured".to_string());
        format!(
            "{} | routing_role={} | position={} | responsibilities={} | competencies={} | allowed_task_types={} | disallowed_task_types={} | details={}",
            self.name,
            self.routing_role(),
            self.profile_position(),
            if responsibilities.is_empty() { "not configured".to_string() } else { responsibilities.join(", ") },
            if competencies.is_empty() { "not configured".to_string() } else { competencies.join(", ") },
            if allowed.is_empty() { "not configured".to_string() } else { allowed.join(", ") },
            if disallowed.is_empty() { "not configured".to_string() } else { disallowed.join(", ") },
            details
        )
    }
}
