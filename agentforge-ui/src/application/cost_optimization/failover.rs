use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderStatus {
    Healthy,
    Degraded,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub cost_per_token: f64,
    pub priority: u32,
    pub status: ProviderStatus,
    pub load: f64, // 0.0 to 1.0
}

impl ProviderConfig {
    pub fn new(id: &str, name: &str, cost_per_token: f64, priority: u32) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            cost_per_token,
            priority,
            status: ProviderStatus::Healthy,
            load: 0.0,
        }
    }
}

pub struct FailoverRouter {
    providers: HashMap<String, ProviderConfig>,
}

impl Default for FailoverRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl FailoverRouter {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register_provider(&mut self, provider: ProviderConfig) {
        self.providers.insert(provider.id.clone(), provider);
    }

    pub fn update_status(&mut self, id: &str, status: ProviderStatus) {
        if let Some(provider) = self.providers.get_mut(id) {
            provider.status = status;
        }
    }

    pub fn update_load(&mut self, id: &str, load: f64) {
        if let Some(provider) = self.providers.get_mut(id) {
            provider.load = load.clamp(0.0, 1.0);
        }
    }

    /// Selects the best provider based on status, priority, load, and cost
    pub fn select_provider(&self) -> Option<ProviderConfig> {
        let mut available: Vec<&ProviderConfig> = self
            .providers
            .values()
            .filter(|p| p.status == ProviderStatus::Healthy && p.load < 0.9)
            .collect();

        if available.is_empty() {
            // Fallback to degraded if no healthy options
            available = self
                .providers
                .values()
                .filter(|p| p.status == ProviderStatus::Degraded)
                .collect();
        }

        if available.is_empty() {
            return None;
        }

        // Sort by priority (higher is better), then cost (lower is better), then load
        available.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.cost_per_token.partial_cmp(&b.cost_per_token).unwrap())
                .then_with(|| a.load.partial_cmp(&b.load).unwrap())
        });

        available.first().copied().cloned()
    }
}
