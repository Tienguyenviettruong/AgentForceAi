use crate::application::orchestration::modes::OperatingMode;
use crate::application::orchestration::role_policy;
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::security::audit::AuditEvent;
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

pub const LOCAL_DESKTOP_ACTOR_ID: &str = "local-user";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolRisk {
    ReadOnly,
    ControlledMutation,
    Sensitive,
}

#[derive(Debug)]
pub struct ToolRequest<'a> {
    pub tool_name: &'a str,
    pub payload: &'a serde_json::Value,
    pub instance_id: &'a str,
    pub session_id: Option<&'a str>,
    pub run_id: Option<&'a str>,
    pub invocation_id: Option<&'a str>,
    pub delegated_agent_id: &'a str,
    pub is_mcp: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PolicyDecision {
    Allowed,
    ApprovalRequired { request_id: String },
    Denied(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PathScope {
    InWorkspace,
    ExternalAbsolute,
    MissingWorkspace,
    WorkspaceUnavailable,
    ParentTraversal,
}

pub struct ToolExecutionGateway {
    db: Arc<dyn DatabasePort>,
}

impl ToolExecutionGateway {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self { db }
    }

    pub fn risk_for(tool_name: &str, is_mcp: bool) -> ToolRisk {
        if is_mcp {
            return ToolRisk::Sensitive;
        }
        match tool_name {
            "read_file" | "analyze_file" => ToolRisk::ReadOnly,
            "save_to_knowledge"
            | "declare_consensus"
            | "handoff_to_team"
            | "create_subtasks"
            | "submit_readback"
            | "record_decision"
            | "submit_deliverable"
            | "record_review"
            | "raise_escalation"
            | "record_evaluation"
            | "record_feedback"
            | "create_learning_candidate" => ToolRisk::ControlledMutation,
            _ => ToolRisk::Sensitive,
        }
    }

    pub fn authorize_runtime(&self, request: &ToolRequest<'_>) -> PolicyDecision {
        let risk = Self::risk_for(request.tool_name, request.is_mcp);
        let Some(run_id) = request.run_id else {
            let reason =
                "Tool denied: the operation is not attached to a traceable run.".to_string();
            self.write_decision(request, None, "denied", &reason);
            return PolicyDecision::Denied(reason);
        };
        let run = match self.db.get_orchestration_run(run_id) {
            Ok(Some(run)) => run,
            _ => {
                let reason = "Mutation denied: associated run was not found.".to_string();
                self.write_decision(request, None, "denied", &reason);
                return PolicyDecision::Denied(reason);
            }
        };
        if run.instance_id != request.instance_id {
            let reason = "Tool denied: run and instance context do not match.".to_string();
            self.write_decision(request, None, "denied", &reason);
            return PolicyDecision::Denied(reason);
        }
        let actor_id = run
            .initiated_by
            .as_deref()
            .map(Self::normalize_actor_id)
            .unwrap_or_default();
        if actor_id.is_empty() {
            let reason = "Mutation denied: run has no initiating security actor.".to_string();
            self.write_decision(request, None, "denied", &reason);
            return PolicyDecision::Denied(reason);
        }
        let invocation_id = match self.seal_invocation(request, &run) {
            Ok(invocation_id) => invocation_id,
            Err(reason) => {
                self.write_decision(request, Some(actor_id), "denied", &reason);
                return PolicyDecision::Denied(reason);
            }
        };

        let permission = format!("tool:execute:{}", request.tool_name);
        if !self
            .db
            .check_actor_permission(actor_id, &permission)
            .unwrap_or(false)
        {
            let reason = format!(
                "Permission denied: actor {} lacks {}.",
                actor_id, permission
            );
            self.write_decision(request, Some(actor_id), "denied", &reason);
            let _ = self.db.update_tool_invocation_status(
                &invocation_id,
                "denied",
                None,
                Some(&reason),
            );
            return PolicyDecision::Denied(reason);
        }

        let delegated_case = match self.db.get_collaboration_case_for_run(run_id) {
            Ok(case) => case,
            Err(error) => {
                let reason = format!(
                    "Tool denied: delegated case scope could not be verified: {}.",
                    error
                );
                self.write_decision(request, Some(actor_id), "denied", &reason);
                let _ = self.db.update_tool_invocation_status(
                    &invocation_id,
                    "denied",
                    None,
                    Some(&reason),
                );
                return PolicyDecision::Denied(reason);
            }
        };
        let grant = match self
            .db
            .get_active_delegated_grant(request.delegated_agent_id, run_id)
        {
            Ok(grant) => grant,
            Err(error) => {
                let reason = format!(
                    "Tool denied: delegated grant scope could not be verified: {}.",
                    error
                );
                self.write_decision(request, Some(actor_id), "denied", &reason);
                let _ = self.db.update_tool_invocation_status(
                    &invocation_id,
                    "denied",
                    None,
                    Some(&reason),
                );
                return PolicyDecision::Denied(reason);
            }
        };
        let is_receiving_delegated_run = delegated_case
            .as_ref()
            .is_some_and(|case| case.target_instance_id == request.instance_id);
        if is_receiving_delegated_run && grant.is_none() {
            let reason = "Delegated tool denied: the receiving run has no active persisted grant."
                .to_string();
            self.write_decision(request, Some(actor_id), "denied", &reason);
            let _ = self.db.update_tool_invocation_status(
                &invocation_id,
                "denied",
                None,
                Some(&reason),
            );
            return PolicyDecision::Denied(reason);
        }
        if let Some(grant) = grant {
            if delegated_case
                .as_ref()
                .is_some_and(|case| case.id != grant.case_id)
            {
                let reason =
                    "Delegated tool denied: grant and collaboration case do not match.".to_string();
                self.write_decision(request, Some(actor_id), "denied", &reason);
                let _ = self.db.update_tool_invocation_status(
                    &invocation_id,
                    "denied",
                    None,
                    Some(&reason),
                );
                return PolicyDecision::Denied(reason);
            }
            let allowed_json = if request.is_mcp {
                &grant.allowed_mcp_json
            } else {
                &grant.allowed_tools_json
            };
            let allowed = serde_json::from_str::<Vec<String>>(allowed_json).unwrap_or_default();
            if !allowed
                .iter()
                .any(|name| name == "all" || name == request.tool_name)
            {
                let reason = format!(
                    "Delegated grant denied tool '{}': it is outside the case mandate.",
                    request.tool_name
                );
                self.write_decision(request, Some(actor_id), "denied", &reason);
                let _ = self.db.update_tool_invocation_status(
                    &invocation_id,
                    "denied",
                    None,
                    Some(&reason),
                );
                return PolicyDecision::Denied(reason);
            }
            if let Some(limit) = grant.token_limit {
                let used = self.db.get_total_tokens_for_run(run_id).unwrap_or(0) as i64;
                if used >= limit {
                    let reason = format!(
                        "Delegated grant token budget exhausted: {} / {} tokens.",
                        used, limit
                    );
                    self.write_decision(request, Some(actor_id), "denied", &reason);
                    let _ = self.db.update_tool_invocation_status(
                        &invocation_id,
                        "denied",
                        None,
                        Some(&reason),
                    );
                    return PolicyDecision::Denied(reason);
                }
            }
        }

        let mode =
            OperatingMode::from_storage(&run.mode).unwrap_or(OperatingMode::HumanInteraction);
        let file_scope_approval_required = match self.file_scope_approval_required(request) {
            Ok(required) => required,
            Err(reason) => {
                self.write_decision(request, Some(actor_id), "denied", &reason);
                let _ = self.db.update_tool_invocation_status(
                    &invocation_id,
                    "denied",
                    None,
                    Some(&reason),
                );
                return PolicyDecision::Denied(reason);
            }
        };
        let delegated_agent_role = self
            .db
            .get_agent(request.delegated_agent_id)
            .ok()
            .flatten()
            .map(|agent| agent.routing_role())
            .unwrap_or_default();
        if !role_policy::tool_allowed_for_role(&delegated_agent_role, request.tool_name) {
            let reason = role_policy::tool_denied_message(&delegated_agent_role, request.tool_name);
            self.write_decision(request, Some(actor_id), "denied", &reason);
            let _ = self.db.update_tool_invocation_status(
                &invocation_id,
                "denied",
                None,
                Some(&reason),
            );
            return PolicyDecision::Denied(reason);
        }
        if risk == ToolRisk::ReadOnly && !file_scope_approval_required {
            self.write_decision(request, Some(actor_id), "allowed", "read-only operation");
            let _ = self
                .db
                .update_tool_invocation_status(&invocation_id, "authorized", None, None);
            return PolicyDecision::Allowed;
        }

        let autonomous_sensitive_allowed = self
            .db
            .get_setting("governance_autonomous_sensitive_allowed")
            .ok()
            .flatten()
            .as_deref()
            == Some("true");
        let autonomous_workspace_file_mutation =
            matches!(request.tool_name, "write_file" | "edit_file")
                && !file_scope_approval_required
                && !role_policy::is_coordinator_role(&delegated_agent_role);
        let requires_approval = match mode {
            OperatingMode::HumanInteraction => true,
            OperatingMode::Supervision => {
                risk == ToolRisk::Sensitive || file_scope_approval_required
            }
            OperatingMode::Autonomous => {
                if file_scope_approval_required {
                    true
                } else if autonomous_workspace_file_mutation {
                    false
                } else if Self::is_filesystem_tool(request.tool_name)
                    && risk == ToolRisk::Sensitive
                    && !autonomous_sensitive_allowed
                {
                    true
                } else if risk == ToolRisk::Sensitive && !autonomous_sensitive_allowed {
                    let reason =
                        "Autonomous policy denied a sensitive operation; enable it explicitly or switch to Supervision for approval."
                            .to_string();
                    self.write_decision(request, Some(actor_id), "denied", &reason);
                    let _ = self.db.update_tool_invocation_status(
                        &invocation_id,
                        "denied",
                        None,
                        Some(&reason),
                    );
                    return PolicyDecision::Denied(reason);
                } else {
                    false
                }
            }
        };

        if requires_approval {
            let operation = self.operation_key(request, &run.mode);
            match self
                .db
                .get_approval_request_for_operation(run_id, &operation)
            {
                Ok(Some(existing)) if existing.status == "approved" => {
                    self.write_decision(
                        request,
                        Some(actor_id),
                        "allowed",
                        "approved sensitive operation",
                    );
                    let _ = self.db.update_tool_invocation_status(
                        &invocation_id,
                        "authorized",
                        Some(&existing.id),
                        None,
                    );
                    return PolicyDecision::Allowed;
                }
                Ok(Some(existing)) if existing.status == "pending" => {
                    let _ = self.db.update_tool_invocation_status(
                        &invocation_id,
                        "awaiting_approval",
                        Some(&existing.id),
                        None,
                    );
                    return PolicyDecision::ApprovalRequired {
                        request_id: existing.id,
                    };
                }
                Ok(Some(existing)) if existing.status == "rejected" => {
                    let reason = "Sensitive operation rejected by governance.".to_string();
                    self.write_decision(request, Some(actor_id), "denied", &reason);
                    let _ = self.db.update_tool_invocation_status(
                        &invocation_id,
                        "rejected",
                        Some(&existing.id),
                        Some(&reason),
                    );
                    return PolicyDecision::Denied(reason);
                }
                _ => {}
            }
            let approval_id = uuid::Uuid::new_v4().to_string();
            let _ = self
                .db
                .create_approval_request(&crate::core::models::ApprovalRequestRecord {
                    id: approval_id.clone(),
                    run_id: run_id.to_string(),
                    operation: operation.clone(),
                    requested_by: Some(actor_id.to_string()),
                    status: "pending".to_string(),
                    resolved_by: None,
                    decision_reason: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    resolved_at: None,
                });
            self.write_decision(
                request,
                Some(actor_id),
                "approval_required",
                &format!("Sensitive operation awaiting approval: {}", operation),
            );
            let _ = self.db.update_tool_invocation_status(
                &invocation_id,
                "awaiting_approval",
                Some(&approval_id),
                None,
            );
            return PolicyDecision::ApprovalRequired {
                request_id: approval_id,
            };
        }

        self.write_decision(
            request,
            Some(actor_id),
            "allowed",
            match mode {
                OperatingMode::HumanInteraction => "approved human-interaction mutation",
                OperatingMode::Supervision => "authorized supervised controlled mutation",
                OperatingMode::Autonomous => "authorized autonomous policy operation",
            },
        );
        let _ = self
            .db
            .update_tool_invocation_status(&invocation_id, "authorized", None, None);
        PolicyDecision::Allowed
    }

    pub fn can_execute_interactively(&self, actor_id: &str, tool_name: &str) -> bool {
        self.db
            .check_actor_permission(
                Self::normalize_actor_id(actor_id),
                &format!("tool:execute:{}", tool_name),
            )
            .unwrap_or(false)
    }

    pub fn record_interactive_decision(&self, actor_id: &str, tool_name: &str, allowed: bool) {
        let _ = self.db.insert_audit_log(&AuditEvent {
            timestamp: chrono::Utc::now(),
            action: if allowed {
                "tool_gateway_interactive_allowed".to_string()
            } else {
                "tool_gateway_interactive_denied".to_string()
            },
            user_id: Some(Self::normalize_actor_id(actor_id).to_string()),
            resource: tool_name.to_string(),
            details: "Interactive user invocation through Tool Execution Gateway".to_string(),
        });
    }

    pub fn enforce_run_budget(&self, run_id: Option<&str>) -> Result<(), String> {
        crate::application::orchestration::governance::GovernanceManager::enforce_run_budget_for_db(
            self.db.as_ref(),
            run_id,
        )
    }

    pub fn resolve_workspace_path(
        &self,
        instance_id: &str,
        requested_path: &str,
    ) -> Result<PathBuf, String> {
        self.resolve_workspace_path_internal(instance_id, requested_path, false)
    }

    pub fn resolve_authorized_tool_path(
        &self,
        request: &ToolRequest<'_>,
        requested_path: &str,
    ) -> Result<PathBuf, String> {
        let Some(run_id) = request.run_id else {
            return self.resolve_workspace_path(request.instance_id, requested_path);
        };
        let approved = self.is_request_approved(request, run_id);
        self.resolve_workspace_path_internal(request.instance_id, requested_path, approved)
    }

    fn normalize_actor_id(actor_id: &str) -> &str {
        if actor_id == "user" {
            LOCAL_DESKTOP_ACTOR_ID
        } else {
            actor_id
        }
    }

    fn operation_key(&self, request: &ToolRequest<'_>, mode: &str) -> String {
        let hash = Sha256::digest(request.payload.to_string().as_bytes());
        let mode = OperatingMode::from_storage(mode)
            .unwrap_or(OperatingMode::HumanInteraction)
            .storage_value();
        let summary = Self::operation_summary(request);
        if summary.is_empty() {
            format!(
                "tool:{}:payload={:x}:mode={}",
                request.tool_name, hash, mode
            )
        } else {
            format!(
                "tool:{}:{}:payload={:x}:mode={}",
                request.tool_name, summary, hash, mode
            )
        }
    }

    fn file_scope_approval_required(&self, request: &ToolRequest<'_>) -> Result<bool, String> {
        let Some(path) = Self::filesystem_path_arg(request) else {
            return Ok(false);
        };
        match self.classify_workspace_path(request.instance_id, path) {
            PathScope::InWorkspace => Ok(false),
            PathScope::ExternalAbsolute | PathScope::WorkspaceUnavailable => Ok(true),
            PathScope::MissingWorkspace => {
                if Path::new(path).is_absolute() {
                    Ok(true)
                } else {
                    Err(
                        "File operation denied: no workspace is configured; select a workspace or use an absolute path that can be approved."
                            .to_string(),
                    )
                }
            }
            PathScope::ParentTraversal => {
                Err("File operation denied: parent-directory traversal is not allowed.".to_string())
            }
        }
    }

    fn resolve_workspace_path_internal(
        &self,
        instance_id: &str,
        requested_path: &str,
        allow_approved_external: bool,
    ) -> Result<PathBuf, String> {
        let requested = Path::new(requested_path);
        if Self::has_parent_traversal(requested) {
            return Err(
                "File operation denied: parent-directory traversal is not allowed.".to_string(),
            );
        }

        let workspace = self.configured_workspace(instance_id);
        let candidate = match &workspace {
            Some(workspace) if !requested.is_absolute() => workspace.join(requested),
            _ => requested.to_path_buf(),
        };

        match self.classify_workspace_path(instance_id, requested_path) {
            PathScope::InWorkspace => Ok(candidate),
            PathScope::ExternalAbsolute => {
                if allow_approved_external {
                    Ok(candidate)
                } else {
                    Err(
                        "File operation denied: path is outside the configured workspace and requires approval."
                            .to_string(),
                    )
                }
            }
            PathScope::WorkspaceUnavailable => {
                if allow_approved_external {
                    Ok(candidate)
                } else {
                    Err(
                        "File operation denied: workspace or target path is unavailable and requires approval."
                            .to_string(),
                    )
                }
            }
            PathScope::MissingWorkspace => {
                if requested.is_absolute() && allow_approved_external {
                    Ok(candidate)
                } else {
                    Err(
                        "File operation denied: no workspace is configured; select a workspace or use an absolute path that can be approved."
                            .to_string(),
                    )
                }
            }
            PathScope::ParentTraversal => {
                Err("File operation denied: parent-directory traversal is not allowed.".to_string())
            }
        }
    }

    fn classify_workspace_path(&self, instance_id: &str, requested_path: &str) -> PathScope {
        let requested = Path::new(requested_path);
        if Self::has_parent_traversal(requested) {
            return PathScope::ParentTraversal;
        }
        let Some(workspace) = self.configured_workspace(instance_id) else {
            return if requested.is_absolute() {
                PathScope::ExternalAbsolute
            } else {
                PathScope::MissingWorkspace
            };
        };
        let workspace_comparable = match Self::comparable_path(&workspace) {
            Ok(path) => path,
            Err(_) => return PathScope::WorkspaceUnavailable,
        };
        let candidate = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            workspace.join(requested)
        };
        let candidate_comparable = match Self::comparable_path(&candidate) {
            Ok(path) => path,
            Err(_) => return PathScope::WorkspaceUnavailable,
        };
        if candidate_comparable.starts_with(&workspace_comparable) {
            PathScope::InWorkspace
        } else {
            PathScope::ExternalAbsolute
        }
    }

    fn configured_workspace(&self, instance_id: &str) -> Option<PathBuf> {
        self.db
            .get_setting(&format!("workspace_{}", instance_id))
            .ok()
            .flatten()
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
    }

    fn comparable_path(path: &Path) -> Result<PathBuf, std::io::Error> {
        if path.exists() {
            path.canonicalize()
        } else if let Some(parent) = path.parent() {
            if parent.exists() {
                parent
                    .canonicalize()
                    .map(|parent| parent.join(path.file_name().unwrap_or_default()))
            } else {
                Ok(path.to_path_buf())
            }
        } else {
            Ok(path.to_path_buf())
        }
    }

    fn has_parent_traversal(path: &Path) -> bool {
        path.components()
            .any(|component| matches!(component, Component::ParentDir))
    }

    fn is_filesystem_tool(tool_name: &str) -> bool {
        matches!(
            tool_name,
            "read_file" | "analyze_file" | "write_file" | "edit_file" | "run_cli"
        )
    }

    fn filesystem_path_arg<'a>(request: &'a ToolRequest<'_>) -> Option<&'a str> {
        match request.tool_name {
            "read_file" | "analyze_file" | "write_file" | "edit_file" => request
                .payload
                .get("path")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty()),
            "run_cli" => Some(
                request
                    .payload
                    .get("cwd")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or("."),
            ),
            _ => None,
        }
    }

    fn operation_summary(request: &ToolRequest<'_>) -> String {
        let mut parts = Vec::new();
        if let Some(path) = Self::filesystem_path_arg(request) {
            parts.push(format!("path={}", Self::compact_label(path, 96)));
        }
        if request.tool_name == "run_cli" {
            if let Some(command) = request
                .payload
                .get("command")
                .and_then(|value| value.as_str())
            {
                parts.push(format!("command={}", Self::compact_label(command, 64)));
            }
        }
        parts.join(";")
    }

    fn compact_label(value: &str, max_chars: usize) -> String {
        let trimmed = value.trim();
        if trimmed.chars().count() <= max_chars {
            return trimmed.to_string();
        }
        let head: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}...", head)
    }

    fn is_request_approved(&self, request: &ToolRequest<'_>, run_id: &str) -> bool {
        let Ok(Some(run)) = self.db.get_orchestration_run(run_id) else {
            return false;
        };
        let operation = self.operation_key(request, &run.mode);
        self.db
            .get_approval_request_for_operation(run_id, &operation)
            .ok()
            .flatten()
            .is_some_and(|request| request.status == "approved")
    }

    fn seal_invocation(
        &self,
        request: &ToolRequest<'_>,
        run: &crate::core::models::OrchestrationRunRecord,
    ) -> Result<String, String> {
        let raw_id = request
            .invocation_id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or("tool-call");
        let invocation_id = if raw_id.starts_with(&format!("{}:", run.id)) {
            raw_id.to_string()
        } else {
            format!("{}:{}", run.id, raw_id)
        };
        let payload = request.payload.to_string();
        let payload_hash = format!("{:x}", Sha256::digest(payload.as_bytes()));
        let associated_data = format!("{}:{}:{}", run.id, request.tool_name, invocation_id);
        let sealed_payload = crate::infrastructure::security::keychain::seal_sensitive_payload(
            &payload,
            &associated_data,
        )
        .map_err(|error| {
            format!(
                "Tool denied: invocation payload encryption failed: {}",
                error
            )
        })?;
        let now = chrono::Utc::now().to_rfc3339();
        self.db
            .upsert_tool_invocation(&crate::core::models::ToolInvocationRecord {
                id: invocation_id.clone(),
                run_id: run.id.clone(),
                tool_name: request.tool_name.to_string(),
                sealed_payload_json: sealed_payload,
                payload_hash,
                mode: run.mode.clone(),
                status: "sealed".to_string(),
                approval_request_id: None,
                result: None,
                created_at: now.clone(),
                updated_at: now,
            })
            .map_err(|error| format!("Tool denied: invocation could not be sealed: {}.", error))?;
        Ok(invocation_id)
    }

    fn write_decision(
        &self,
        request: &ToolRequest<'_>,
        actor_id: Option<&str>,
        decision: &str,
        reason: &str,
    ) {
        let details = format!(
            "decision={} instance={} session={} run={} delegated_agent={}; {}",
            decision,
            request.instance_id,
            request.session_id.unwrap_or("none"),
            request.run_id.unwrap_or("none"),
            request.delegated_agent_id,
            reason
        );
        let _ = self.db.insert_audit_log(&AuditEvent {
            timestamp: chrono::Utc::now(),
            action: format!("tool_gateway_{}", decision),
            user_id: actor_id.map(str::to_string),
            resource: request.tool_name.to_string(),
            details: details.clone(),
        });
        if let Some(run_id) = request.run_id {
            let _ = self
                .db
                .insert_run_event(&crate::core::models::RunEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    run_id: run_id.to_string(),
                    event_type: format!("tool_policy_{}", decision),
                    actor_type: "policy".to_string(),
                    actor_id: actor_id.map(str::to_string),
                    task_id: None,
                    payload: Some(details),
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
        }
    }
}
