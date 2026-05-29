use crate::core::models::{
    Agent, Instance, Provider, RunEventRecord, Task, WorkflowExecutionRecord,
};
use crate::infrastructure::message_bus::routing::TeamMessage;
use crate::infrastructure::security::audit::AuditEvent;

use super::DatabasePort;

pub trait ProviderConfigPort: Send + Sync {
    fn get_provider_by_name(&self, provider_name: &str) -> anyhow::Result<Option<Provider>>;
    fn list_providers(&self) -> anyhow::Result<Vec<Provider>>;
}

impl<T: DatabasePort + ?Sized> ProviderConfigPort for T {
    fn get_provider_by_name(&self, provider_name: &str) -> anyhow::Result<Option<Provider>> {
        DatabasePort::get_provider_by_name(self, provider_name)
    }

    fn list_providers(&self) -> anyhow::Result<Vec<Provider>> {
        DatabasePort::list_providers(self)
    }
}

pub trait WorkerCoordinationPort: ProviderConfigPort {
    fn list_instances(&self) -> anyhow::Result<Vec<Instance>>;
    fn get_instance_agents(&self, instance_id: &str) -> anyhow::Result<Vec<String>>;
    fn get_agent(&self, agent_id: &str) -> anyhow::Result<Option<Agent>>;
    fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>>;
    fn list_tasks_for_instance(&self, instance_id: &str) -> anyhow::Result<Vec<Task>>;
    fn is_task_unblocked(&self, task_id: &str) -> anyhow::Result<bool>;
    fn claim_task_for_instance(
        &self,
        task_id: &str,
        agent_id: &str,
        instance_id: &str,
    ) -> anyhow::Result<bool>;
    fn mark_task_completed(&self, task_id: &str) -> anyhow::Result<()>;
    fn mark_task_failed(&self, task_id: &str) -> anyhow::Result<()>;
    fn mark_task_waiting_approval(&self, task_id: &str) -> anyhow::Result<()>;
    fn insert_team_message(&self, msg: &TeamMessage) -> anyhow::Result<()>;
    fn get_workflow_execution(
        &self,
        execution_id: &str,
    ) -> anyhow::Result<Option<WorkflowExecutionRecord>>;
}

impl<T: DatabasePort + ?Sized> WorkerCoordinationPort for T {
    fn list_instances(&self) -> anyhow::Result<Vec<Instance>> {
        DatabasePort::list_instances(self)
    }

    fn get_instance_agents(&self, instance_id: &str) -> anyhow::Result<Vec<String>> {
        DatabasePort::get_instance_agents(self, instance_id)
    }

    fn get_agent(&self, agent_id: &str) -> anyhow::Result<Option<Agent>> {
        DatabasePort::get_agent(self, agent_id)
    }

    fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>> {
        DatabasePort::get_setting(self, key)
    }

    fn list_tasks_for_instance(&self, instance_id: &str) -> anyhow::Result<Vec<Task>> {
        DatabasePort::list_tasks_for_instance(self, instance_id)
    }

    fn is_task_unblocked(&self, task_id: &str) -> anyhow::Result<bool> {
        DatabasePort::is_task_unblocked(self, task_id)
    }

    fn claim_task_for_instance(
        &self,
        task_id: &str,
        agent_id: &str,
        instance_id: &str,
    ) -> anyhow::Result<bool> {
        DatabasePort::claim_task_for_instance(self, task_id, agent_id, instance_id)
    }

    fn mark_task_completed(&self, task_id: &str) -> anyhow::Result<()> {
        DatabasePort::mark_task_completed(self, task_id)
    }

    fn mark_task_failed(&self, task_id: &str) -> anyhow::Result<()> {
        DatabasePort::mark_task_failed(self, task_id)
    }

    fn mark_task_waiting_approval(&self, task_id: &str) -> anyhow::Result<()> {
        DatabasePort::mark_task_waiting_approval(self, task_id)
    }

    fn insert_team_message(&self, msg: &TeamMessage) -> anyhow::Result<()> {
        DatabasePort::insert_team_message(self, msg)
    }

    fn get_workflow_execution(
        &self,
        execution_id: &str,
    ) -> anyhow::Result<Option<WorkflowExecutionRecord>> {
        DatabasePort::get_workflow_execution(self, execution_id)
    }
}

pub trait GovernanceRuntimePort: Send + Sync {
    fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>>;
    fn get_total_tokens_for_run(&self, run_id: &str) -> anyhow::Result<usize>;
    fn insert_run_event(&self, event: &RunEventRecord) -> anyhow::Result<()>;
    fn insert_audit_log(&self, event: &AuditEvent) -> anyhow::Result<()>;
}

impl<T: DatabasePort + ?Sized> GovernanceRuntimePort for T {
    fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>> {
        DatabasePort::get_setting(self, key)
    }

    fn get_total_tokens_for_run(&self, run_id: &str) -> anyhow::Result<usize> {
        DatabasePort::get_total_tokens_for_run(self, run_id)
    }

    fn insert_run_event(&self, event: &RunEventRecord) -> anyhow::Result<()> {
        DatabasePort::insert_run_event(self, event)
    }

    fn insert_audit_log(&self, event: &AuditEvent) -> anyhow::Result<()> {
        DatabasePort::insert_audit_log(self, event)
    }
}
