#[derive(Debug, Clone)]
pub struct CollaborationCaseRecord {
    pub id: String,
    pub legacy_correlation_id: Option<String>,
    pub origin_run_id: Option<String>,
    pub parent_case_id: Option<String>,
    pub owner_instance_id: String,
    pub target_instance_id: String,
    pub owner_agent_id: Option<String>,
    pub state: String,
    pub priority: String,
    pub risk_level: String,
    pub objective: String,
    pub acceptance_json: String,
    pub constraints_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HandoffPackageRecord {
    pub id: String,
    pub case_id: String,
    pub run_id: Option<String>,
    pub from_instance_id: String,
    pub to_instance_id: String,
    pub from_agent_id: Option<String>,
    pub to_agent_id: Option<String>,
    pub objective: String,
    pub acceptance_json: String,
    pub constraints_json: String,
    pub context_refs_json: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct CaseReadbackRecord {
    pub id: String,
    pub case_id: String,
    pub handoff_id: String,
    pub agent_id: String,
    pub understanding: String,
    pub assumptions_json: String,
    pub questions_json: String,
    pub status: String,
    pub accepted_by: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CaseDecisionRecord {
    pub id: String,
    pub case_id: String,
    pub run_id: Option<String>,
    pub author_agent_id: String,
    pub decision: String,
    pub rationale: String,
    pub alternatives_json: String,
    pub evidence_refs_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct CaseDeliverableRecord {
    pub id: String,
    pub case_id: String,
    pub run_id: Option<String>,
    pub agent_id: String,
    pub title: String,
    pub artifact_refs_json: String,
    pub acceptance_evidence_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CaseReviewRecord {
    pub id: String,
    pub case_id: String,
    pub deliverable_id: String,
    pub reviewer_agent_id: String,
    pub verdict: String,
    pub findings_json: String,
    pub required_actions_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct CaseConsensusRecord {
    pub id: String,
    pub case_id: String,
    pub proposal: String,
    pub status: String,
    pub quorum_rule_json: String,
    pub resolution: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CaseConsensusVoteRecord {
    pub id: String,
    pub consensus_id: String,
    pub voter_id: String,
    pub vote: String,
    pub rationale: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct CaseEscalationRecord {
    pub id: String,
    pub case_id: String,
    pub raised_by: String,
    pub reason: String,
    pub severity: String,
    pub status: String,
    pub resolved_by: Option<String>,
    pub resolution: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DelegatedGrantRecord {
    pub id: String,
    pub case_id: String,
    pub run_id: Option<String>,
    pub grantor_actor_id: String,
    pub grantee_agent_id: String,
    pub allowed_tools_json: String,
    pub allowed_mcp_json: String,
    pub workspace_scope_json: String,
    pub token_limit: Option<i64>,
    pub cost_limit: Option<f64>,
    pub expires_at: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AgentCompetencyRecord {
    pub agent_id: String,
    pub competency_key: String,
    pub score: f64,
    pub evidence_count: i64,
    pub confidence: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct RoutingDecisionRecord {
    pub id: String,
    pub case_id: String,
    pub selected_agent_id: String,
    pub competency_key: String,
    pub score_snapshot_json: String,
    pub rationale: String,
    pub created_at: String,
}
