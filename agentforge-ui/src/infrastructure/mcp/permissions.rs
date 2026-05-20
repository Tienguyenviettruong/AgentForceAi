use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct PermissionConfig {
    pub allowed_tools: Vec<String>,
    pub max_requests_per_minute: u32,
}

struct RateLimiter {
    requests: Vec<Instant>,
    max_requests: u32,
    window: Duration,
}

impl RateLimiter {
    fn new(max_requests: u32) -> Self {
        Self {
            requests: Vec::new(),
            max_requests,
            window: Duration::from_secs(60),
        }
    }

    fn check_and_record(&mut self) -> bool {
        let now = Instant::now();
        // Remove requests outside the window
        self.requests
            .retain(|&time| now.duration_since(time) < self.window);

        if self.requests.len() >= self.max_requests as usize {
            false
        } else {
            self.requests.push(now);
            true
        }
    }
}

pub struct McpPermissionManager {
    // agent_id -> PermissionConfig
    configs: Arc<RwLock<HashMap<String, PermissionConfig>>>,
    // agent_id -> RateLimiter
    rate_limiters: Arc<RwLock<HashMap<String, RateLimiter>>>,
}

impl Default for McpPermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl McpPermissionManager {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_agent_permissions(&self, agent_id: &str, config: PermissionConfig) {
        let max_req = config.max_requests_per_minute;
        self.configs
            .write()
            .unwrap()
            .insert(agent_id.to_string(), config);
        self.rate_limiters
            .write()
            .unwrap()
            .insert(agent_id.to_string(), RateLimiter::new(max_req));
    }

    pub fn can_execute(&self, agent_id: &str, tool_name: &str) -> Result<(), String> {
        let configs = self.configs.read().unwrap();
        if let Some(config) = configs.get(agent_id) {
            if !config.allowed_tools.contains(&tool_name.to_string())
                && !config.allowed_tools.contains(&"*".to_string())
            {
                return Err(format!(
                    "Agent {} is not permitted to use tool {}",
                    agent_id, tool_name
                ));
            }
        } else {
            // Default deny
            return Err(format!(
                "No permission configuration found for agent {}",
                agent_id
            ));
        }

        let mut limiters = self.rate_limiters.write().unwrap();
        if let Some(limiter) = limiters.get_mut(agent_id) {
            if !limiter.check_and_record() {
                return Err(format!("Rate limit exceeded for agent {}", agent_id));
            }
        }

        Ok(())
    }
}
