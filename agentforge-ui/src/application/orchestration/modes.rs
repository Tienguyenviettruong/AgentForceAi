use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OperatingMode {
    /// Task 3.06: Direct agent-human chat interface
    #[default]
    HumanInteraction,
    /// Task 3.07: Agent-agent collaboration with human monitor/intervene
    Supervision,
    /// Task 3.08: Fully autonomous execution
    Autonomous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeTransitionEvent {
    pub from: OperatingMode,
    pub to: OperatingMode,
    pub reason: String,
    pub timestamp: String,
}

/// Task 3.09: Mode logic
pub struct ModeManager {
    current_mode: OperatingMode,
    history: Vec<ModeTransitionEvent>,
}

impl ModeManager {
    pub fn new(initial_mode: OperatingMode) -> Self {
        Self {
            current_mode: initial_mode,
            history: Vec::new(),
        }
    }

    pub fn current_mode(&self) -> OperatingMode {
        self.current_mode
    }

    pub fn can_transition(&self, new_mode: OperatingMode) -> bool {
        // Any specific transition rules can go here. For now, allow all.
        self.current_mode != new_mode
    }

    pub fn transition_to(&mut self, new_mode: OperatingMode, reason: &str) -> Result<(), String> {
        if !self.can_transition(new_mode) {
            return Err(format!(
                "Cannot transition from {:?} to {:?}",
                self.current_mode, new_mode
            ));
        }

        let event = ModeTransitionEvent {
            from: self.current_mode,
            to: new_mode,
            reason: reason.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.current_mode = new_mode;
        self.history.push(event);
        Ok(())
    }

    pub fn history(&self) -> &[ModeTransitionEvent] {
        &self.history
    }
}
