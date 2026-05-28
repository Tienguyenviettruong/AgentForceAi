use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Task 3.05: Orchestration State Machine Enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OrchestrationState {
    #[default]
    Planning,
    Executing,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionEvent {
    pub from: OrchestrationState,
    pub to: OrchestrationState,
    pub timestamp: String,
    pub reason: String,
}

/// Task 3.05: Orchestration State Machine
#[derive(Debug, Default)]
pub struct OrchestrationStateMachine {
    pub state: OrchestrationState,
    pub history: Vec<StateTransitionEvent>,
}

impl OrchestrationStateMachine {
    pub fn new() -> Self {
        Self {
            state: OrchestrationState::Planning,
            history: Vec::new(),
        }
    }

    pub fn current_state(&self) -> OrchestrationState {
        self.state
    }

    pub fn can_transition_to(&self, new_state: OrchestrationState) -> bool {
        match (self.state, new_state) {
            (OrchestrationState::Planning, OrchestrationState::Executing) => true,
            (OrchestrationState::Planning, OrchestrationState::Failed) => true,
            (OrchestrationState::Executing, OrchestrationState::Paused) => true,
            (OrchestrationState::Executing, OrchestrationState::Completed) => true,
            (OrchestrationState::Executing, OrchestrationState::Failed) => true,
            (OrchestrationState::Paused, OrchestrationState::Executing) => true,
            (OrchestrationState::Paused, OrchestrationState::Failed) => true,
            // Completed and Failed are terminal states
            _ => false,
        }
    }

    pub fn transition_to(
        &mut self,
        new_state: OrchestrationState,
        reason: &str,
    ) -> Result<(), String> {
        if !self.can_transition_to(new_state) {
            return Err(format!(
                "Invalid state transition from {:?} to {:?}",
                self.state, new_state
            ));
        }

        let event = StateTransitionEvent {
            from: self.state,
            to: new_state,
            timestamp: chrono::Utc::now().to_rfc3339(),
            reason: reason.to_string(),
        };

        self.state = new_state;
        self.history.push(event);
        Ok(())
    }
}

/// Task 3.02, 3.03: DagTask and scheduling priorities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub priority: u32,
    pub deadline: Option<String>,
    pub assignee_id: Option<String>,
}

/// Task 3.04: Dependency Resolver with topological sort and cycle detection
pub struct DependencyResolver;

impl DependencyResolver {
    /// Detects cycles in the DAG and returns topological sort order
    pub fn resolve(tasks: &[DagTask]) -> Result<Vec<String>, String> {
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize all nodes
        for task in tasks {
            in_degree.insert(task.id.clone(), 0);
            adj_list.entry(task.id.clone()).or_default();
        }

        // Build graph
        for task in tasks {
            for dep in &task.dependencies {
                if !in_degree.contains_key(dep) {
                    return Err(format!("Dependency {} not found for task {}", dep, task.id));
                }
                adj_list
                    .entry(dep.clone())
                    .or_default()
                    .push(task.id.clone());
                *in_degree.get_mut(&task.id).unwrap() += 1;
            }
        }

        let mut queue: VecDeque<String> = VecDeque::new();
        for (id, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(id.clone());
            }
        }

        let mut sorted_order = Vec::new();

        while let Some(node) = queue.pop_front() {
            sorted_order.push(node.clone());
            if let Some(neighbors) = adj_list.get(&node) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        if sorted_order.len() != tasks.len() {
            return Err("Cycle detected in DAG task dependencies".to_string());
        }

        Ok(sorted_order)
    }
}

/// Task 3.01: Orchestrator Core Engine
#[derive(Default)]
pub struct Orchestrator {
    pub state_machine: OrchestrationStateMachine,
    pub tasks: HashMap<String, DagTask>,
    pub execution_order: Vec<String>,
    pub completed_tasks: HashSet<String>,
    pub failed_tasks: HashSet<String>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Task 3.02: Task decomposition algorithm (LLM-based)
    /// Decomposes a high-level goal into a DAG of tasks using the LLM provider.
    pub async fn decompose_goal(
        &mut self,
        _goal: &str,
        _db: std::sync::Arc<dyn crate::core::traits::database::DatabasePort>,
        _chat_history: Vec<crate::providers::ChatMessage>,
        _provider_config: crate::db::Provider,
        _instance_id: &str,
    ) -> Result<Vec<DagTask>, String> {
        if self.state_machine.current_state() != OrchestrationState::Planning {
            return Err("Can only decompose tasks in Planning state".to_string());
        }
        Err(
            "Legacy direct task decomposition is disabled. Create a persisted iFlow and execute its AgentTask through AgentExecutor so capability selection, context snapshots, policy and approvals are enforced."
                .to_string(),
        )
    }
    /// Load tasks (after decomposition)
    pub fn load_tasks(&mut self, dag_tasks: Vec<DagTask>) -> Result<(), String> {
        if self.state_machine.current_state() != OrchestrationState::Planning {
            return Err("Can only load tasks in Planning state".to_string());
        }

        // Validate and resolve dependencies
        let sorted = DependencyResolver::resolve(&dag_tasks)?;

        self.tasks.clear();
        for task in dag_tasks {
            self.tasks.insert(task.id.clone(), task);
        }
        self.execution_order = sorted;

        Ok(())
    }

    pub fn start_execution(&mut self) -> Result<(), String> {
        self.state_machine
            .transition_to(OrchestrationState::Executing, "Starting execution")?;
        Ok(())
    }

    pub fn pause_execution(&mut self, reason: &str) -> Result<(), String> {
        self.state_machine
            .transition_to(OrchestrationState::Paused, reason)?;
        Ok(())
    }

    pub fn resume_execution(&mut self) -> Result<(), String> {
        self.state_machine
            .transition_to(OrchestrationState::Executing, "Resuming execution")?;
        Ok(())
    }

    /// Task 3.03: Task scheduling (get next available tasks)
    pub fn get_next_tasks(&self) -> Vec<DagTask> {
        if self.state_machine.current_state() != OrchestrationState::Executing {
            return Vec::new();
        }

        let mut available = Vec::new();
        for task_id in &self.execution_order {
            if self.completed_tasks.contains(task_id) || self.failed_tasks.contains(task_id) {
                continue;
            }

            let task = self.tasks.get(task_id).unwrap();
            let all_deps_met = task
                .dependencies
                .iter()
                .all(|dep| self.completed_tasks.contains(dep));

            if all_deps_met {
                available.push(task.clone());
            }
        }

        // Sort by priority (higher first), then by deadline if any
        available.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| match (&a.deadline, &b.deadline) {
                    (Some(d1), Some(d2)) => d1.cmp(d2),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                })
        });

        available
    }

    pub fn mark_task_completed(&mut self, task_id: &str) -> Result<(), String> {
        if !self.tasks.contains_key(task_id) {
            return Err("Task not found".to_string());
        }
        self.completed_tasks.insert(task_id.to_string());

        self.check_overall_status();
        Ok(())
    }

    pub fn mark_task_failed(&mut self, task_id: &str) -> Result<(), String> {
        if !self.tasks.contains_key(task_id) {
            return Err("Task not found".to_string());
        }
        self.failed_tasks.insert(task_id.to_string());

        // Simple fail-fast for the orchestrator
        let _ = self.state_machine.transition_to(
            OrchestrationState::Failed,
            &format!("Task {} failed", task_id),
        );
        Ok(())
    }

    fn check_overall_status(&mut self) {
        if self.completed_tasks.len() == self.tasks.len() {
            let _ = self
                .state_machine
                .transition_to(OrchestrationState::Completed, "All tasks completed");
        }
    }
}
