use uuid::Uuid;
use tokio::sync::{RwLock, Mutex};
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use anyhow::Result;

// 3.29: Governance Policy Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernancePolicy {
    pub max_concurrent_agents: usize,
    pub max_tokens_per_run: usize,
    pub require_approval_for_sensitive_ops: bool,
    pub allowed_tools: Vec<String>,
}

impl Default for GovernancePolicy {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 10,
            max_tokens_per_run: 100_000,
            require_approval_for_sensitive_ops: true,
            allowed_tools: vec![],
        }
    }
}

// 3.31: Human Approval Workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub operation: String,
    pub requested_by: Uuid,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
}

// 3.32: Governance Audit Trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub event_type: String,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

// 3.33: Token Budget Enforcement
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub total_allocated: usize,
    pub consumed: usize,
}

// 3.35: Orchestration Template System
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationTemplate {
    pub id: Uuid,
    pub name: String,
    pub steps: Vec<String>,
}

// 3.37: Orchestration Metrics Collection
#[derive(Debug, Clone, Default)]
pub struct OrchestrationMetrics {
    pub total_runs: usize,
    pub successful_runs: usize,
    pub failed_runs: usize,
    pub total_tokens_consumed: usize,
}

// 3.39: Orchestration Pause/Resume
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrchestrationState {
    Running,
    Paused,
    Completed,
    Failed,
}

pub struct GovernanceManager {
    policy: Arc<RwLock<GovernancePolicy>>,
    approval_requests: Arc<RwLock<HashMap<Uuid, ApprovalRequest>>>,
    audit_trail: Arc<RwLock<Vec<AuditEvent>>>,
    budgets: Arc<RwLock<HashMap<Uuid, TokenBudget>>>,
    templates: Arc<RwLock<HashMap<Uuid, OrchestrationTemplate>>>,
    metrics: Arc<RwLock<OrchestrationMetrics>>,
    orchestration_states: Arc<RwLock<HashMap<Uuid, OrchestrationState>>>,
    // For 3.36 concurrent execution limit
    active_orchestrations: Arc<Mutex<usize>>,
    // Database port for persisting audit events to SQLite
    db: Option<Arc<dyn crate::core::traits::database::DatabasePort>>,
}

impl GovernanceManager {
    pub fn new(policy: GovernancePolicy) -> Self {
        Self {
            policy: Arc::new(RwLock::new(policy)),
            approval_requests: Arc::new(RwLock::new(HashMap::new())),
            audit_trail: Arc::new(RwLock::new(Vec::new())),
            budgets: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(OrchestrationMetrics::default())),
            orchestration_states: Arc::new(RwLock::new(HashMap::new())),
            active_orchestrations: Arc::new(Mutex::new(0)),
            db: None,
        }
    }

    /// Set the database port for persisting governance audit events to SQLite
    pub fn with_db(mut self, db: Arc<dyn crate::core::traits::database::DatabasePort>) -> Self {
        self.db = Some(db);
        self
    }

    // 3.29 Check policy constraints
    pub async fn check_can_start_agent(&self) -> Result<()> {
        let policy = self.policy.read().await;
        let active = *self.active_orchestrations.lock().await;
        if active >= policy.max_concurrent_agents {
            Err(anyhow::anyhow!("Max concurrent agents limit reached"))
        } else {
            Ok(())
        }
    }

    // 3.31 Human approval workflow
    pub async fn request_approval(&self, operation: String, requested_by: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        let request = ApprovalRequest {
            id,
            operation,
            requested_by,
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
        };
        self.approval_requests.write().await.insert(id, request);
        id
    }

    pub async fn resolve_approval(&self, id: Uuid, approved: bool, reason: Option<String>) -> Result<()> {
        let mut requests = self.approval_requests.write().await;
        if let Some(req) = requests.get_mut(&id) {
            req.status = if approved {
                ApprovalStatus::Approved
            } else {
                ApprovalStatus::Rejected(reason.unwrap_or_default())
            };
            
            // 3.32 Log audit event
            self.log_audit_event(
                if approved { "APPROVAL_GRANTED".to_string() } else { "APPROVAL_REJECTED".to_string() },
                format!("Approval {} for operation {}", if approved { "granted" } else { "rejected" }, req.operation)
            ).await;
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Approval request not found"))
        }
    }

    // 3.32 Governance audit trail — now persists to both in-memory and SQLite
    pub async fn log_audit_event(&self, event_type: String, description: String) {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            event_type: event_type.clone(),
            description: description.clone(),
            timestamp: Utc::now(),
        };
        self.audit_trail.write().await.push(event);

        // Also persist to database if available
        if let Some(ref db) = self.db {
            let db_event = crate::infrastructure::security::audit::AuditEvent {
                timestamp: Utc::now(),
                action: format!("governance:{}", event_type),
                user_id: None,
                resource: "governance".to_string(),
                details: description,
            };
            let _ = db.insert_audit_log(&db_event);
        }
    }

    pub async fn get_audit_trail(&self) -> Vec<AuditEvent> {
        self.audit_trail.read().await.clone()
    }

    // 3.33 Token budget enforcement
    pub async fn allocate_budget(&self, run_id: Uuid, tokens: usize) -> Result<()> {
        let policy = self.policy.read().await;
        if tokens > policy.max_tokens_per_run {
            return Err(anyhow::anyhow!("Requested tokens exceed policy limit"));
        }
        
        self.budgets.write().await.insert(run_id, TokenBudget {
            total_allocated: tokens,
            consumed: 0,
        });
        Ok(())
    }

    pub async fn consume_tokens(&self, run_id: Uuid, tokens: usize) -> Result<()> {
        let mut budgets = self.budgets.write().await;
        if let Some(budget) = budgets.get_mut(&run_id) {
            if budget.consumed + tokens > budget.total_allocated {
                return Err(anyhow::anyhow!("Token budget exceeded"));
            }
            budget.consumed += tokens;
            
            // 3.37 Update metrics
            let mut metrics = self.metrics.write().await;
            metrics.total_tokens_consumed += tokens;
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Budget not found for run"))
        }
    }

    // 3.34 Orchestration replay and debugging
    pub async fn get_replay_data(&self, run_id: Uuid) -> Result<Vec<AuditEvent>> {
        let trail = self.audit_trail.read().await;
        Ok(trail.iter().filter(|e| e.description.contains(&run_id.to_string())).cloned().collect())
    }

    // 3.35 Orchestration template system
    pub async fn register_template(&self, name: String, steps: Vec<String>) -> Uuid {
        let id = Uuid::new_v4();
        let template = OrchestrationTemplate {
            id,
            name,
            steps,
        };
        self.templates.write().await.insert(id, template);
        id
    }

    pub async fn get_template(&self, id: Uuid) -> Option<OrchestrationTemplate> {
        self.templates.read().await.get(&id).cloned()
    }

    // 3.36 Concurrent orchestration execution
    pub async fn start_orchestration(&self, run_id: Uuid) -> Result<()> {
        self.check_can_start_agent().await?;
        
        let mut active = self.active_orchestrations.lock().await;
        *active += 1;
        
        self.orchestration_states.write().await.insert(run_id, OrchestrationState::Running);
        
        let mut metrics = self.metrics.write().await;
        metrics.total_runs += 1;
        
        Ok(())
    }

    pub async fn end_orchestration(&self, run_id: Uuid, success: bool) -> Result<()> {
        let mut active = self.active_orchestrations.lock().await;
        if *active > 0 {
            *active -= 1;
        }
        
        let mut states = self.orchestration_states.write().await;
        if let Some(state) = states.get_mut(&run_id) {
            *state = if success { OrchestrationState::Completed } else { OrchestrationState::Failed };
        }
        
        let mut metrics = self.metrics.write().await;
        if success {
            metrics.successful_runs += 1;
        } else {
            metrics.failed_runs += 1;
        }
        
        Ok(())
    }

    // 3.37 Metrics collection
    pub async fn get_metrics(&self) -> OrchestrationMetrics {
        self.metrics.read().await.clone()
    }

    // 3.39 Orchestration pause/resume
    pub async fn pause_orchestration(&self, run_id: Uuid) -> Result<()> {
        let mut states = self.orchestration_states.write().await;
        if let Some(state) = states.get_mut(&run_id) {
            if *state == OrchestrationState::Running {
                *state = OrchestrationState::Paused;
                self.log_audit_event("ORCHESTRATION_PAUSED".to_string(), format!("Run {} paused", run_id)).await;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Orchestration is not running"))
            }
        } else {
            Err(anyhow::anyhow!("Orchestration not found"))
        }
    }

    pub async fn resume_orchestration(&self, run_id: Uuid) -> Result<()> {
        let mut states = self.orchestration_states.write().await;
        if let Some(state) = states.get_mut(&run_id) {
            if *state == OrchestrationState::Paused {
                *state = OrchestrationState::Running;
                self.log_audit_event("ORCHESTRATION_RESUMED".to_string(), format!("Run {} resumed", run_id)).await;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Orchestration is not paused"))
            }
        } else {
            Err(anyhow::anyhow!("Orchestration not found"))
        }
    }
}

impl Default for GovernanceManager {
    fn default() -> Self {
        Self::new(GovernancePolicy::default())
    }
}
