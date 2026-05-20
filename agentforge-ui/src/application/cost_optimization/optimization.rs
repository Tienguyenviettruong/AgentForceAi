use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::core::Budget;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAlert {
    pub id: Uuid,
    pub budget_id: Uuid,
    pub threshold_percentage: f64,
    pub level: AlertLevel,
    pub message: String,
    pub triggered_at: Option<DateTime<Utc>>,
}

impl CostAlert {
    pub fn new(
        budget_id: Uuid,
        threshold_percentage: f64,
        level: AlertLevel,
        message: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            budget_id,
            threshold_percentage,
            level,
            message: message.to_string(),
            triggered_at: None,
        }
    }

    pub fn evaluate(&mut self, budget: &Budget) -> bool {
        if budget.limit <= 0.0 {
            return false;
        }

        let current_percentage = (budget.spent / budget.limit) * 100.0;

        if current_percentage >= self.threshold_percentage && self.triggered_at.is_none() {
            self.triggered_at = Some(Utc::now());
            true
        } else if current_percentage < self.threshold_percentage {
            // Reset if it drops below threshold
            self.triggered_at = None;
            false
        } else {
            false
        }
    }
}

pub struct OptimizationEngine {
    pub alerts: Vec<CostAlert>,
}

impl Default for OptimizationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimizationEngine {
    pub fn new() -> Self {
        Self { alerts: Vec::new() }
    }

    pub fn add_alert(&mut self, alert: CostAlert) {
        self.alerts.push(alert);
    }

    pub fn check_alerts(&mut self, budget: &Budget) -> Vec<&CostAlert> {
        let mut triggered = Vec::new();
        for alert in &mut self.alerts {
            if alert.budget_id == budget.id {
                let just_triggered = alert.evaluate(budget);
                if just_triggered || alert.triggered_at.is_some() {
                    triggered.push(&*alert);
                }
            }
        }
        triggered
    }
}
