use super::lifecycle::{AgentLifecycleManager, AgentStatus};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct RecoveryPolicy {
    pub max_restarts: u32,
    pub restart_delay: Duration,
    pub current_restarts: u32,
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub last_heartbeat: SystemTime,
    pub is_healthy: bool,
    pub timeout_threshold: Duration,
}

pub struct HealthMonitor {
    pub agent_id: String,
    pub health_check: HealthCheck,
    pub recovery_policy: RecoveryPolicy,
}

impl HealthMonitor {
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            health_check: HealthCheck {
                last_heartbeat: SystemTime::now(),
                is_healthy: true,
                timeout_threshold: Duration::from_secs(30),
            },
            recovery_policy: RecoveryPolicy {
                max_restarts: 3,
                restart_delay: Duration::from_secs(5),
                current_restarts: 0,
            },
        }
    }

    pub fn record_heartbeat(&mut self) {
        self.health_check.last_heartbeat = SystemTime::now();
        self.health_check.is_healthy = true;
    }

    pub fn check_health(&mut self, lifecycle: &mut AgentLifecycleManager) -> Result<(), String> {
        if lifecycle.current_status != AgentStatus::Running {
            return Ok(());
        }

        if let Ok(elapsed) = self.health_check.last_heartbeat.elapsed() {
            if elapsed > self.health_check.timeout_threshold {
                self.health_check.is_healthy = false;
                return self.handle_failure(lifecycle);
            }
        }
        Ok(())
    }

    fn handle_failure(&mut self, lifecycle: &mut AgentLifecycleManager) -> Result<(), String> {
        if self.recovery_policy.current_restarts < self.recovery_policy.max_restarts {
            self.recovery_policy.current_restarts += 1;
            // simulate auto-restart policy by resetting heartbeat and setting state
            self.record_heartbeat();
            lifecycle.current_status = AgentStatus::Running;
            Ok(())
        } else {
            lifecycle.current_status = AgentStatus::Error;
            Err("Max restarts exceeded, agent moved to Error state".to_string())
        }
    }
}
