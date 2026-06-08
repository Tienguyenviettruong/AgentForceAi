use crate::core::models::{
    AgentCompetencyRecord, CaseConsensusRecord, CaseConsensusVoteRecord, CaseDecisionRecord,
    CaseDeliverableRecord, CaseEscalationRecord, CaseReadbackRecord, CaseReviewRecord,
    CollaborationCaseRecord, DelegatedGrantRecord, HandoffPackageRecord, RoutingDecisionRecord,
};
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::security::audit::AuditEvent;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use uuid::Uuid;

pub struct HandoffInput<'a> {
    pub run_id: Option<&'a str>,
    pub correlation_id: Option<&'a str>,
    pub from_instance_id: &'a str,
    pub to_instance_id: &'a str,
    pub from_agent_id: Option<&'a str>,
    pub objective: &'a str,
    pub acceptance_json: &'a str,
    pub constraints_json: &'a str,
    pub context_refs_json: &'a str,
    pub priority: &'a str,
    pub risk_level: &'a str,
}

pub struct CollaborationService {
    db: Arc<dyn DatabasePort>,
}

fn case_transition_allowed(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("submitted", "readback_pending")
            | ("submitted", "in_progress")
            | ("submitted", "escalated")
            | ("readback_pending", "in_progress")
            | ("readback_pending", "escalated")
            | ("in_progress", "review_pending")
            | ("in_progress", "escalated")
            | ("review_pending", "in_progress")
            | ("review_pending", "consensus_pending")
            | ("review_pending", "escalated")
            | ("consensus_pending", "completed")
            | ("consensus_pending", "in_progress")
            | ("consensus_pending", "escalated")
            | ("escalated", "in_progress")
            | ("escalated", "cancelled")
            | ("escalated", "rejected")
    ) || (!matches!(from, "completed" | "cancelled" | "rejected") && to == "cancelled")
}

impl CollaborationService {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self { db }
    }

    pub fn create_handoff(&self, input: HandoffInput<'_>) -> Result<(String, String, String)> {
        let correlation_id = input
            .correlation_id
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = chrono::Utc::now().to_rfc3339();
        let existing = self
            .db
            .get_collaboration_case_by_correlation_id(&correlation_id)?;
        let case_id = existing
            .as_ref()
            .map(|case| case.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let state = existing
            .as_ref()
            .map(|case| case.state.clone())
            .unwrap_or_else(|| "submitted".to_string());
        self.db
            .upsert_collaboration_case(&CollaborationCaseRecord {
                id: case_id.clone(),
                legacy_correlation_id: Some(correlation_id.clone()),
                origin_run_id: input.run_id.map(str::to_string).or_else(|| {
                    existing
                        .as_ref()
                        .and_then(|case| case.origin_run_id.clone())
                }),
                parent_case_id: existing
                    .as_ref()
                    .and_then(|case| case.parent_case_id.clone()),
                owner_instance_id: existing
                    .as_ref()
                    .map(|case| case.owner_instance_id.clone())
                    .unwrap_or_else(|| input.from_instance_id.to_string()),
                target_instance_id: existing
                    .as_ref()
                    .map(|case| case.target_instance_id.clone())
                    .unwrap_or_else(|| input.to_instance_id.to_string()),
                owner_agent_id: existing
                    .as_ref()
                    .and_then(|case| case.owner_agent_id.clone())
                    .or_else(|| input.from_agent_id.map(str::to_string)),
                state,
                priority: input.priority.to_string(),
                risk_level: input.risk_level.to_string(),
                objective: existing
                    .as_ref()
                    .map(|case| case.objective.clone())
                    .unwrap_or_else(|| input.objective.to_string()),
                acceptance_json: existing
                    .as_ref()
                    .map(|case| case.acceptance_json.clone())
                    .unwrap_or_else(|| input.acceptance_json.to_string()),
                constraints_json: existing
                    .as_ref()
                    .map(|case| case.constraints_json.clone())
                    .unwrap_or_else(|| input.constraints_json.to_string()),
                created_at: existing
                    .as_ref()
                    .map(|case| case.created_at.clone())
                    .unwrap_or_else(|| now.clone()),
                updated_at: now.clone(),
                resolved_at: existing.and_then(|case| case.resolved_at),
            })?;
        let handoff_id = Uuid::new_v4().to_string();
        self.db.insert_handoff_package(&HandoffPackageRecord {
            id: handoff_id.clone(),
            case_id: case_id.clone(),
            run_id: input.run_id.map(str::to_string),
            from_instance_id: input.from_instance_id.to_string(),
            to_instance_id: input.to_instance_id.to_string(),
            from_agent_id: input.from_agent_id.map(str::to_string),
            to_agent_id: None,
            objective: input.objective.to_string(),
            acceptance_json: input.acceptance_json.to_string(),
            constraints_json: input.constraints_json.to_string(),
            context_refs_json: input.context_refs_json.to_string(),
            status: "submitted".to_string(),
            created_at: now,
        })?;
        self.db.upsert_cross_team_case(
            &correlation_id,
            input.from_instance_id,
            input.to_instance_id,
            "HANDOFF_SUBMITTED",
            input.objective,
        )?;
        Ok((case_id, handoff_id, correlation_id))
    }

    pub fn acknowledge_and_readback(
        &self,
        case_id: &str,
        agent_id: &str,
        understanding: &str,
        assumptions_json: &str,
        questions_json: &str,
        accepted_by: Option<&str>,
    ) -> Result<String> {
        let case = self.required_case(case_id)?;
        if !matches!(
            case.state.as_str(),
            "submitted" | "acknowledged" | "readback_pending"
        ) {
            return Err(anyhow!(
                "Readback is invalid while case is in state '{}'.",
                case.state
            ));
        }
        let handoff = self
            .db
            .get_latest_handoff_for_case(case_id)?
            .ok_or_else(|| anyhow!("Readback requires an existing handoff package."))?;
        let accepted = accepted_by.is_some();
        let readback_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.db.insert_case_readback(&CaseReadbackRecord {
            id: readback_id.clone(),
            case_id: case_id.to_string(),
            handoff_id: handoff.id,
            agent_id: agent_id.to_string(),
            understanding: understanding.to_string(),
            assumptions_json: assumptions_json.to_string(),
            questions_json: questions_json.to_string(),
            status: if accepted { "accepted" } else { "submitted" }.to_string(),
            accepted_by: accepted_by.map(str::to_string),
            created_at: now.clone(),
            resolved_at: accepted.then_some(now),
        })?;
        self.transition_case(
            case_id,
            if accepted {
                "in_progress"
            } else {
                "readback_pending"
            },
        )?;
        Ok(readback_id)
    }

    pub fn accept_readback(&self, case_id: &str, readback_id: &str, actor_id: &str) -> Result<()> {
        if !self
            .db
            .check_actor_permission(actor_id, "tool:execute:accept_readback")
            .unwrap_or(false)
        {
            return Err(anyhow!(
                "Only an authorized human actor may accept a delegated readback."
            ));
        }
        let readback = self
            .db
            .list_case_readbacks(case_id)?
            .into_iter()
            .find(|record| record.id == readback_id)
            .ok_or_else(|| anyhow!("Readback does not belong to the requested case."))?;
        if readback.status != "submitted" {
            return Err(anyhow!("Only submitted readbacks can be accepted."));
        }
        let case = self.required_case(case_id)?;
        self.db
            .resolve_case_readback(readback_id, "accepted", Some(actor_id))?;
        self.transition_case(case_id, "in_progress")?;
        for grant in self.db.list_active_delegated_grants_for_case(case_id)? {
            if let Some(run_id) = grant.run_id.as_deref() {
                self.db
                    .update_delegated_grant_status(&grant.id, "replaced")?;
                self.grant_delegated_execution(
                    case_id,
                    run_id,
                    actor_id,
                    &grant.grantee_agent_id,
                    "[\"read_file\",\"analyze_file\",\"save_to_knowledge\",\"record_decision\",\"create_subtasks\",\"submit_deliverable\",\"record_review\",\"declare_consensus\",\"handoff_to_team\",\"raise_escalation\",\"record_feedback\"]",
                )?;
                self.db
                    .insert_run_event(&crate::core::models::RunEventRecord {
                        id: Uuid::new_v4().to_string(),
                        run_id: run_id.to_string(),
                        event_type: "delegated_readback_accepted".to_string(),
                        actor_type: "user".to_string(),
                        actor_id: Some(actor_id.to_string()),
                        task_id: None,
                        payload: Some(format!("case_id={} readback_id={}", case_id, readback_id)),
                        created_at: chrono::Utc::now().to_rfc3339(),
                    })?;
            }
        }
        self.db.insert_audit_log(&AuditEvent {
            timestamp: chrono::Utc::now(),
            action: "collaboration_readback_accepted".to_string(),
            user_id: Some(actor_id.to_string()),
            resource: case_id.to_string(),
            details: format!(
                "readback_id={} owner_instance={} target_instance={}",
                readback_id, case.owner_instance_id, case.target_instance_id
            ),
        })?;
        Ok(())
    }

    pub fn has_accepted_readback(&self, case_id: &str) -> Result<bool> {
        Ok(self
            .db
            .list_case_readbacks(case_id)?
            .into_iter()
            .any(|readback| readback.status == "accepted"))
    }

    pub fn record_decision(
        &self,
        case_id: &str,
        run_id: Option<&str>,
        agent_id: &str,
        decision: &str,
        rationale: &str,
        evidence_refs_json: &str,
    ) -> Result<String> {
        self.require_work_started(case_id)?;
        let id = Uuid::new_v4().to_string();
        self.db.insert_case_decision(&CaseDecisionRecord {
            id: id.clone(),
            case_id: case_id.to_string(),
            run_id: run_id.map(str::to_string),
            author_agent_id: agent_id.to_string(),
            decision: decision.to_string(),
            rationale: rationale.to_string(),
            alternatives_json: "[]".to_string(),
            evidence_refs_json: evidence_refs_json.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;
        Ok(id)
    }

    pub fn submit_deliverable(
        &self,
        case_id: &str,
        run_id: Option<&str>,
        agent_id: &str,
        title: &str,
        artifact_refs_json: &str,
        acceptance_evidence_json: &str,
    ) -> Result<String> {
        self.require_work_started(case_id)?;
        let refs: Vec<String> = serde_json::from_str(artifact_refs_json)
            .map_err(|_| anyhow!("Deliverable artifact_refs must be a JSON string array."))?;
        if refs.is_empty() {
            return Err(anyhow!("Deliverable must reference at least one artifact."));
        }
        if let Some(run_id) = run_id {
            let artifacts = self.db.list_artifacts_for_run(run_id)?;
            if refs
                .iter()
                .any(|reference| !artifacts.iter().any(|artifact| artifact.id == *reference))
            {
                return Err(anyhow!(
                    "Deliverable contains an artifact that is not recorded for this run."
                ));
            }
        }
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.db.insert_case_deliverable(&CaseDeliverableRecord {
            id: id.clone(),
            case_id: case_id.to_string(),
            run_id: run_id.map(str::to_string),
            agent_id: agent_id.to_string(),
            title: title.to_string(),
            artifact_refs_json: artifact_refs_json.to_string(),
            acceptance_evidence_json: acceptance_evidence_json.to_string(),
            status: "submitted".to_string(),
            created_at: now.clone(),
            updated_at: now,
        })?;
        self.transition_case(case_id, "review_pending")?;
        Ok(id)
    }

    pub fn review_deliverable(
        &self,
        case_id: &str,
        deliverable_id: &str,
        reviewer_agent_id: &str,
        verdict: &str,
        findings_json: &str,
        required_actions_json: &str,
    ) -> Result<String> {
        if !matches!(verdict, "accepted" | "changes_requested" | "rejected") {
            return Err(anyhow!(
                "Review verdict must be accepted, changes_requested, or rejected."
            ));
        }
        let deliverable = self
            .db
            .list_case_deliverables(case_id)?
            .into_iter()
            .find(|deliverable| deliverable.id == deliverable_id)
            .ok_or_else(|| anyhow!("Deliverable does not belong to the requested case."))?;
        if deliverable.agent_id == reviewer_agent_id {
            return Err(anyhow!(
                "A deliverable author cannot review their own deliverable."
            ));
        }
        let id = Uuid::new_v4().to_string();
        self.db.insert_case_review(&CaseReviewRecord {
            id: id.clone(),
            case_id: case_id.to_string(),
            deliverable_id: deliverable_id.to_string(),
            reviewer_agent_id: reviewer_agent_id.to_string(),
            verdict: verdict.to_string(),
            findings_json: findings_json.to_string(),
            required_actions_json: required_actions_json.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;
        self.db
            .update_case_deliverable_status(deliverable_id, verdict)?;
        let outcome_score = match verdict {
            "accepted" => 1.0,
            "changes_requested" => 0.5,
            _ => 0.0,
        };
        let previous = self
            .db
            .list_agent_competencies(Some("delivery_quality"))?
            .into_iter()
            .find(|competency| competency.agent_id == deliverable.agent_id);
        let (score, count) = previous
            .map(|competency| {
                let count = competency.evidence_count + 1;
                (
                    (competency.score * competency.evidence_count as f64 + outcome_score)
                        / count as f64,
                    count,
                )
            })
            .unwrap_or((outcome_score, 1));
        self.db.upsert_agent_competency(&AgentCompetencyRecord {
            agent_id: deliverable.agent_id,
            competency_key: "delivery_quality".to_string(),
            score,
            evidence_count: count,
            confidence: (count as f64 / 10.0).min(1.0),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })?;
        self.transition_case(
            case_id,
            if verdict == "accepted" {
                "consensus_pending"
            } else {
                "in_progress"
            },
        )?;
        Ok(id)
    }

    pub fn record_consensus(
        &self,
        case_id: &str,
        proposal: &str,
        _resolution: Option<&str>,
    ) -> Result<String> {
        let case = self.required_case(case_id)?;
        if case.state != "consensus_pending" {
            return Err(anyhow!("Consensus requires a reviewed deliverable."));
        }
        if proposal.trim().is_empty() {
            return Err(anyhow!("Consensus proposal cannot be empty."));
        }
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.db.insert_case_consensus(&CaseConsensusRecord {
            id: id.clone(),
            case_id: case_id.to_string(),
            proposal: proposal.to_string(),
            status: "proposed".to_string(),
            quorum_rule_json: "{\"rule\":\"human_resolution_or_two_distinct_accept_votes\",\"required_accept_votes\":2}".to_string(),
            resolution: None,
            created_at: now,
            resolved_at: None,
        })?;
        Ok(id)
    }

    pub fn cast_consensus_vote(
        &self,
        case_id: &str,
        consensus_id: &str,
        voter_id: &str,
        vote: &str,
        rationale: &str,
    ) -> Result<()> {
        if !matches!(vote, "accept" | "reject") {
            return Err(anyhow!("Consensus vote must be accept or reject."));
        }
        let case = self.required_case(case_id)?;
        if case.state != "consensus_pending" {
            return Err(anyhow!(
                "Consensus voting requires a pending consensus case."
            ));
        }
        let consensus = self
            .db
            .list_case_consensus_records(case_id)?
            .into_iter()
            .find(|record| record.id == consensus_id && record.status == "proposed")
            .ok_or_else(|| anyhow!("Consensus proposal is not pending for this case."))?;
        self.db
            .insert_case_consensus_vote(&CaseConsensusVoteRecord {
                id: Uuid::new_v4().to_string(),
                consensus_id: consensus.id.clone(),
                voter_id: voter_id.to_string(),
                vote: vote.to_string(),
                rationale: (!rationale.trim().is_empty()).then(|| rationale.to_string()),
                created_at: chrono::Utc::now().to_rfc3339(),
            })?;
        let votes = self.db.list_case_consensus_votes(consensus_id)?;
        if votes.iter().any(|vote| vote.vote == "reject") {
            self.db.resolve_case_consensus(
                consensus_id,
                "rejected",
                "Rejected by an explicit reviewer vote.",
            )?;
            self.transition_case(case_id, "in_progress")?;
        } else if votes.iter().filter(|vote| vote.vote == "accept").count() >= 2 {
            self.db.resolve_case_consensus(
                consensus_id,
                "accepted",
                "Accepted by the persisted two-voter quorum.",
            )?;
            self.transition_case(case_id, "completed")?;
        }
        Ok(())
    }

    pub fn resolve_consensus(
        &self,
        case_id: &str,
        consensus_id: &str,
        actor_id: &str,
        accepted: bool,
        resolution: &str,
    ) -> Result<()> {
        if !self
            .db
            .check_actor_permission(actor_id, "tool:execute:resolve_consensus")
            .unwrap_or(false)
        {
            return Err(anyhow!(
                "Only an authorized human actor may resolve consensus."
            ));
        }
        if resolution.trim().is_empty() {
            return Err(anyhow!("Consensus resolution must state its rationale."));
        }
        let case = self.required_case(case_id)?;
        if case.state != "consensus_pending" {
            return Err(anyhow!(
                "Consensus resolution requires a pending consensus case."
            ));
        }
        if !self
            .db
            .list_case_consensus_records(case_id)?
            .iter()
            .any(|record| record.id == consensus_id && record.status == "proposed")
        {
            return Err(anyhow!("Consensus proposal is not pending for this case."));
        }
        if !self
            .db
            .list_case_consensus_votes(consensus_id)?
            .iter()
            .any(|vote| vote.voter_id == actor_id)
        {
            self.db
                .insert_case_consensus_vote(&CaseConsensusVoteRecord {
                    id: Uuid::new_v4().to_string(),
                    consensus_id: consensus_id.to_string(),
                    voter_id: actor_id.to_string(),
                    vote: if accepted { "accept" } else { "reject" }.to_string(),
                    rationale: Some(resolution.to_string()),
                    created_at: chrono::Utc::now().to_rfc3339(),
                })?;
        }
        self.db.resolve_case_consensus(
            consensus_id,
            if accepted { "accepted" } else { "rejected" },
            resolution,
        )?;
        self.transition_case(case_id, if accepted { "completed" } else { "in_progress" })?;
        self.db.insert_audit_log(&AuditEvent {
            timestamp: chrono::Utc::now(),
            action: "collaboration_consensus_resolved".to_string(),
            user_id: Some(actor_id.to_string()),
            resource: case_id.to_string(),
            details: format!("consensus_id={} accepted={}", consensus_id, accepted),
        })?;
        Ok(())
    }

    pub fn escalate(
        &self,
        case_id: &str,
        raised_by: &str,
        severity: &str,
        reason: &str,
    ) -> Result<String> {
        self.required_case(case_id)?;
        let id = Uuid::new_v4().to_string();
        self.db.insert_case_escalation(&CaseEscalationRecord {
            id: id.clone(),
            case_id: case_id.to_string(),
            raised_by: raised_by.to_string(),
            reason: reason.to_string(),
            severity: severity.to_string(),
            status: "open".to_string(),
            resolved_by: None,
            resolution: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            resolved_at: None,
        })?;
        self.transition_case(case_id, "escalated")?;
        Ok(id)
    }

    pub fn resolve_escalation(
        &self,
        escalation_id: &str,
        actor_id: &str,
        resume_case: bool,
        resolution: &str,
    ) -> Result<()> {
        if !self
            .db
            .check_actor_permission(actor_id, "tool:execute:resolve_escalation")
            .unwrap_or(false)
        {
            return Err(anyhow!(
                "Only an authorized human actor may resolve escalation."
            ));
        }
        if resolution.trim().is_empty() {
            return Err(anyhow!("Escalation resolution cannot be empty."));
        }
        let escalation = self
            .db
            .list_pending_case_escalations(1000)?
            .into_iter()
            .find(|record| record.id == escalation_id)
            .ok_or_else(|| anyhow!("Escalation is not open."))?;
        self.db
            .resolve_case_escalation(escalation_id, actor_id, resolution)?;
        self.transition_case(
            &escalation.case_id,
            if resume_case {
                "in_progress"
            } else {
                "cancelled"
            },
        )?;
        self.db.insert_audit_log(&AuditEvent {
            timestamp: chrono::Utc::now(),
            action: "collaboration_escalation_resolved".to_string(),
            user_id: Some(actor_id.to_string()),
            resource: escalation.case_id,
            details: format!(
                "escalation_id={} resume_case={} resolution={}",
                escalation_id, resume_case, resolution
            ),
        })?;
        Ok(())
    }

    pub fn grant_delegated_execution(
        &self,
        case_id: &str,
        run_id: &str,
        grantor_actor_id: &str,
        agent_id: &str,
        allowed_tools_json: &str,
    ) -> Result<String> {
        if !self.has_accepted_readback(case_id)? {
            return Err(anyhow!(
                "Delegated execution requires an accepted readback."
            ));
        }
        let id = Uuid::new_v4().to_string();
        self.db.insert_delegated_grant(&DelegatedGrantRecord {
            id: id.clone(),
            case_id: case_id.to_string(),
            run_id: Some(run_id.to_string()),
            grantor_actor_id: grantor_actor_id.to_string(),
            grantee_agent_id: agent_id.to_string(),
            allowed_tools_json: allowed_tools_json.to_string(),
            allowed_mcp_json: "[]".to_string(),
            workspace_scope_json: "[]".to_string(),
            token_limit: Some(250_000),
            cost_limit: None,
            expires_at: None,
            status: "active".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;
        Ok(id)
    }

    pub fn grant_readback_only(
        &self,
        case_id: &str,
        run_id: &str,
        grantor_actor_id: &str,
        agent_id: &str,
    ) -> Result<String> {
        self.required_case(case_id)?;
        let id = Uuid::new_v4().to_string();
        self.db.insert_delegated_grant(&DelegatedGrantRecord {
            id: id.clone(),
            case_id: case_id.to_string(),
            run_id: Some(run_id.to_string()),
            grantor_actor_id: grantor_actor_id.to_string(),
            grantee_agent_id: agent_id.to_string(),
            allowed_tools_json:
                "[\"read_file\",\"analyze_file\",\"submit_readback\",\"raise_escalation\"]"
                    .to_string(),
            allowed_mcp_json: "[]".to_string(),
            workspace_scope_json: "[]".to_string(),
            token_limit: Some(50_000),
            cost_limit: None,
            expires_at: None,
            status: "active".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;
        Ok(id)
    }

    pub fn route_by_competency(
        &self,
        case_id: &str,
        competency_key: &str,
        eligible_agent_ids: &[String],
    ) -> Result<Option<String>> {
        let candidates: Vec<AgentCompetencyRecord> = self
            .db
            .list_agent_competencies(Some(competency_key))?
            .into_iter()
            .filter(|competency| eligible_agent_ids.contains(&competency.agent_id))
            .collect();
        let selected = candidates.first().cloned();
        if let Some(competency) = selected {
            self.db.insert_routing_decision(&RoutingDecisionRecord {
                id: Uuid::new_v4().to_string(),
                case_id: case_id.to_string(),
                selected_agent_id: competency.agent_id.clone(),
                competency_key: competency_key.to_string(),
                score_snapshot_json: serde_json::json!({
                    "selected": {
                        "agent_id": competency.agent_id.clone(),
                        "score": competency.score,
                        "confidence": competency.confidence,
                        "evidence_count": competency.evidence_count
                    },
                    "eligible_candidates": candidates.iter().map(|candidate| serde_json::json!({
                        "agent_id": candidate.agent_id,
                        "score": candidate.score,
                        "confidence": candidate.confidence,
                        "evidence_count": candidate.evidence_count
                    })).collect::<Vec<_>>()
                })
                .to_string(),
                rationale: "Highest persisted competency score at routing time.".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            })?;
            return Ok(Some(competency.agent_id));
        }
        Ok(None)
    }

    pub fn context_for_run(&self, run_id: &str) -> Result<Option<(String, String)>> {
        let Some(case) = self.db.get_collaboration_case_for_run(run_id)? else {
            return Ok(None);
        };
        let readback_status = self
            .db
            .list_case_readbacks(&case.id)?
            .into_iter()
            .last()
            .map(|readback| readback.status)
            .unwrap_or_else(|| "missing".to_string());
        let accepted_readback = self
            .db
            .list_case_readbacks(&case.id)?
            .into_iter()
            .rev()
            .find(|readback| readback.status == "accepted")
            .map(|readback| {
                format!(
                    "Understanding: {}\nAssumptions: {}\nQuestions: {}",
                    readback.understanding, readback.assumptions_json, readback.questions_json
                )
            })
            .unwrap_or_else(|| "No accepted readback content.".to_string());
        let decisions = self
            .db
            .list_case_decisions(&case.id)?
            .into_iter()
            .map(|decision| format!("- {}: {}", decision.decision, decision.rationale))
            .collect::<Vec<_>>()
            .join("\n");
        let deliverables = self
            .db
            .list_case_deliverables(&case.id)?
            .into_iter()
            .map(|deliverable| format!("- {} [{}]", deliverable.title, deliverable.status))
            .collect::<Vec<_>>()
            .join("\n");
        let text = format!(
            "Case: {}\nState: {}\nObjective: {}\nAcceptance: {}\nConstraints: {}\nReadback: {}\nAccepted Readback Evidence:\n{}\nDecisions:\n{}\nDeliverables:\n{}",
            case.id,
            case.state,
            case.objective,
            case.acceptance_json,
            case.constraints_json,
            readback_status,
            accepted_readback,
            if decisions.is_empty() {
                "None"
            } else {
                &decisions
            },
            if deliverables.is_empty() {
                "None"
            } else {
                &deliverables
            }
        );
        Ok(Some((case.id, text)))
    }

    fn required_case(&self, case_id: &str) -> Result<CollaborationCaseRecord> {
        self.db
            .get_collaboration_case(case_id)?
            .ok_or_else(|| anyhow!("Collaboration case '{}' was not found.", case_id))
    }

    fn require_work_started(&self, case_id: &str) -> Result<()> {
        let case = self.required_case(case_id)?;
        if !matches!(case.state.as_str(), "in_progress" | "review_pending") {
            return Err(anyhow!(
                "Case work requires accepted readback; current state is '{}'.",
                case.state
            ));
        }
        if !self.has_accepted_readback(case_id)? {
            return Err(anyhow!("Case work requires accepted readback evidence."));
        }
        Ok(())
    }

    fn transition_case(&self, case_id: &str, next_state: &str) -> Result<()> {
        let current = self.required_case(case_id)?;
        if current.state == next_state {
            return Ok(());
        }
        if !case_transition_allowed(&current.state, next_state) {
            return Err(anyhow!(
                "Case transition '{}' -> '{}' is not allowed.",
                current.state,
                next_state
            ));
        }
        self.db.update_collaboration_case_state(case_id, next_state)
    }
}

#[cfg(test)]
mod tests {
    use super::case_transition_allowed;

    #[test]
    fn case_requires_review_before_completion() {
        assert!(!case_transition_allowed("in_progress", "completed"));
        assert!(case_transition_allowed("in_progress", "review_pending"));
        assert!(case_transition_allowed("consensus_pending", "completed"));
        assert!(!case_transition_allowed("completed", "in_progress"));
    }
}
