#[derive(Clone, Debug)]
pub struct OrchestrationRunRecord {
    pub id: String,
    pub session_id: String,
    pub instance_id: String,
    pub initiated_by: Option<String>,
    pub goal: String,
    pub mode: String,
    pub status: String,
    pub workflow_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct RunEventRecord {
    pub id: String,
    pub run_id: String,
    pub event_type: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub task_id: Option<String>,
    pub payload: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug)]
pub struct ApprovalRequestRecord {
    pub id: String,
    pub run_id: String,
    pub operation: String,
    pub requested_by: Option<String>,
    pub status: String,
    pub resolved_by: Option<String>,
    pub decision_reason: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToolInvocationRecord {
    pub id: String,
    pub run_id: String,
    pub tool_name: String,
    pub sealed_payload_json: String,
    pub payload_hash: String,
    pub mode: String,
    pub status: String,
    pub approval_request_id: Option<String>,
    pub result: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct ModeTransitionRecord {
    pub id: String,
    pub instance_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub from_mode: String,
    pub to_mode: String,
    pub reason: Option<String>,
    pub policy_version: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug)]
pub struct CapabilitySelectionRecord {
    pub id: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub capability_kind: String,
    pub capability_id: String,
    pub enabled: bool,
    pub selected_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct LlmContextSnapshotRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
    pub instance_id: String,
    pub agent_id: String,
    pub mode: Option<String>,
    pub selected_capabilities_json: String,
    pub context_hash: String,
    pub character_count: usize,
    pub created_at: String,
}

#[derive(Clone, Debug)]
pub struct LlmContextSourceRecord {
    pub id: String,
    pub snapshot_id: String,
    pub source_kind: String,
    pub source_id: String,
    pub source_hash: String,
    pub rank: Option<i64>,
    pub character_count: usize,
    pub trust_level: String,
    pub created_at: String,
}

#[derive(Clone, Debug)]
pub struct ArtifactRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
    pub instance_id: String,
    pub agent_id: Option<String>,
    pub invocation_id: Option<String>,
    pub artifact_kind: String,
    pub path: String,
    pub content_hash: String,
    pub created_at: String,
}
