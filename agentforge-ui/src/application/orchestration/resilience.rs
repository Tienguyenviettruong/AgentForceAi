use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub id: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub reset_timeout_secs: i64,
    pub last_failure_time: Option<DateTime<Utc>>,
}

impl CircuitBreaker {
    pub fn new(id: String, failure_threshold: u32, reset_timeout_secs: i64) -> Self {
        Self {
            id,
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            reset_timeout_secs,
            last_failure_time: None,
        }
    }

    pub fn record_success(&mut self) {
        if self.state == CircuitState::HalfOpen {
            self.reset();
        } else if self.state == CircuitState::Closed {
            self.failure_count = 0; // reset on success
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Utc::now());

        if self.state == CircuitState::Closed && self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        } else if self.state == CircuitState::HalfOpen {
            self.state = CircuitState::Open;
        }
    }

    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if Utc::now() - last_failure > Duration::seconds(self.reset_timeout_secs) {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true, // allow one execution to test
        }
    }

    fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.last_failure_time = None;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackAction {
    pub id: String,
    pub description: String,
    pub state_snapshot: String, // serialized state
    pub timestamp: DateTime<Utc>,
}

pub struct RollbackManager {
    pub actions: Vec<RollbackAction>,
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RollbackManager {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn create_checkpoint(&mut self, description: String, state_snapshot: String) -> String {
        let action = RollbackAction {
            id: uuid::Uuid::new_v4().to_string(),
            description,
            state_snapshot,
            timestamp: Utc::now(),
        };
        let id = action.id.clone();
        self.actions.push(action);
        id
    }

    pub fn rollback_to(&mut self, checkpoint_id: &str) -> Result<String, String> {
        let index = self.actions.iter().position(|a| a.id == checkpoint_id).ok_or("Checkpoint not found")?;
        let state = self.actions[index].state_snapshot.clone();

        // Remove subsequent checkpoints
        self.actions.truncate(index + 1);

        Ok(state)
    }

    pub fn rollback_latest(&mut self) -> Result<String, String> {
        if let Some(action) = self.actions.pop() {
            Ok(action.state_snapshot)
        } else {
            Err("No checkpoints available".to_string())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub id: String,
    pub source: String,
    pub message: String,
    pub severity: ErrorSeverity,
    pub timestamp: DateTime<Utc>,
    pub escalated: bool,
}

pub struct ErrorEscalation {
    pub events: Vec<ErrorEvent>,
    pub escalation_threshold: u32,
}

impl ErrorEscalation {
    pub fn new(escalation_threshold: u32) -> Self {
        Self {
            events: Vec::new(),
            escalation_threshold,
        }
    }

    pub fn report_error(&mut self, source: String, message: String, severity: ErrorSeverity) -> ErrorEvent {
        let mut event = ErrorEvent {
            id: uuid::Uuid::new_v4().to_string(),
            source,
            message,
            severity: severity.clone(),
            timestamp: Utc::now(),
            escalated: false,
        };

        if severity == ErrorSeverity::Critical {
            event.escalated = true;
            self.escalate_error(&event);
        } else {
            let recent_errors = self.events.iter()
                .filter(|e| e.source == event.source && e.timestamp > Utc::now() - Duration::minutes(5))
                .count();

            if recent_errors as u32 >= self.escalation_threshold {
                event.escalated = true;
                self.escalate_error(&event);
            }
        }

        self.events.push(event.clone());
        event
    }

    fn escalate_error(&self, _event: &ErrorEvent) {
        // Here we could trigger a UI notification or log to a central monitoring system
        // For now, it's a stub to represent the escalation action.
    }

    pub fn get_unresolved_errors(&self) -> Vec<ErrorEvent> {
        self.events.clone()
    }
}

pub struct ResilienceManager {
    pub circuit_breakers: HashMap<String, CircuitBreaker>,
    pub rollback_manager: RollbackManager,
    pub error_escalation: ErrorEscalation,
}

impl ResilienceManager {
    pub fn new(escalation_threshold: u32) -> Self {
        Self {
            circuit_breakers: HashMap::new(),
            rollback_manager: RollbackManager::new(),
            error_escalation: ErrorEscalation::new(escalation_threshold),
        }
    }

    pub fn register_circuit_breaker(&mut self, id: String, threshold: u32, timeout_secs: i64) {
        self.circuit_breakers.insert(id.clone(), CircuitBreaker::new(id, threshold, timeout_secs));
    }

    pub fn get_circuit_breaker(&mut self, id: &str) -> Option<&mut CircuitBreaker> {
        self.circuit_breakers.get_mut(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new("cb1".to_string(), 3, 60);

        assert!(cb.can_execute());

        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state, CircuitState::Open);
        assert!(!cb.can_execute());

        // Wait timeout logically or change state manually for test
        // By reducing reset_timeout_secs to 0 just for testing behavior or simulate time
        cb.reset_timeout_secs = -1; // so it's instantly half open

        assert!(cb.can_execute());
        assert_eq!(cb.state, CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[test]
    fn test_rollback_manager() {
        let mut rm = RollbackManager::new();
        let cp1 = rm.create_checkpoint("init".to_string(), "state1".to_string());
        let cp2 = rm.create_checkpoint("step1".to_string(), "state2".to_string());

        assert_eq!(rm.actions.len(), 2);

        let state = rm.rollback_to(&cp1).unwrap();
        assert_eq!(state, "state1");
        assert_eq!(rm.actions.len(), 1);

        let state2 = rm.rollback_latest().unwrap();
        assert_eq!(state2, "state1");
        assert_eq!(rm.actions.len(), 0);
    }

    #[test]
    fn test_error_escalation() {
        let mut ee = ErrorEscalation::new(2);

        let e1 = ee.report_error("db".to_string(), "timeout".to_string(), ErrorSeverity::Medium);
        assert!(!e1.escalated);

        let e2 = ee.report_error("db".to_string(), "timeout again".to_string(), ErrorSeverity::Medium);
        assert!(!e2.escalated);

        let e3 = ee.report_error("db".to_string(), "timeout thrice".to_string(), ErrorSeverity::Medium);
        assert!(e3.escalated);

        let e4 = ee.report_error("auth".to_string(), "breach".to_string(), ErrorSeverity::Critical);
        assert!(e4.escalated);
    }
}
