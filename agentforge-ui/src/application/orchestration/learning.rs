use crate::core::models::{
    BenchmarkRunRecord, CanaryDeploymentRecord, FeedbackRecord, LearningCandidateRecord,
    LessonRecord, PromotionDecisionRecord, RollbackRecord, RunEvaluationRecord,
    SkillVersionRecord, WorkflowRecord, WorkflowVersionRecord,
};
use crate::core::traits::database::DatabasePort;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use uuid::Uuid;

pub struct LearningService {
    db: Arc<dyn DatabasePort>,
}

fn candidate_transition_allowed(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("draft", "benchmark_passed")
            | ("draft", "benchmark_failed")
            | ("benchmark_failed", "benchmark_passed")
            | ("benchmark_failed", "benchmark_failed")
            | ("benchmark_passed", "canary_running")
            | ("canary_running", "canary_passed")
            | ("canary_running", "canary_failed")
            | ("canary_passed", "promoted")
            | ("canary_running", "rolled_back")
            | ("canary_passed", "rolled_back")
            | ("promoted", "rolled_back")
    )
}

impl LearningService {
    pub fn new(db: Arc<dyn DatabasePort>) -> Self {
        Self { db }
    }

    pub fn record_evaluation(
        &self,
        run_id: &str,
        evaluator_id: &str,
        score: f64,
        verdict: &str,
        evidence_json: &str,
    ) -> Result<String> {
        if !(0.0..=1.0).contains(&score) {
            return Err(anyhow!("Evaluation score must be between 0 and 1."));
        }
        if verdict.trim().is_empty() {
            return Err(anyhow!("Evaluation verdict is required."));
        }
        let evidence: serde_json::Value = serde_json::from_str(evidence_json)
            .map_err(|_| anyhow!("Evaluation evidence must be valid JSON."))?;
        let has_evidence = evidence
            .as_array()
            .is_some_and(|values| !values.is_empty())
            || evidence
                .as_object()
                .is_some_and(|values| !values.is_empty());
        if !has_evidence {
            return Err(anyhow!("Evaluation requires persisted evidence."));
        }
        let run = self
            .db
            .get_orchestration_run(run_id)?
            .ok_or_else(|| anyhow!("Evaluation requires a persisted orchestration run."))?;
        if !matches!(run.status.as_str(), "completed" | "failed" | "cancelled") {
            return Err(anyhow!(
                "Evaluation can be recorded only after the target run is terminal."
            ));
        }
        let id = Uuid::new_v4().to_string();
        self.db.insert_run_evaluation(&RunEvaluationRecord {
            id: id.clone(),
            run_id: run_id.to_string(),
            rubric_id: None,
            evaluator_id: evaluator_id.to_string(),
            score,
            verdict: verdict.to_string(),
            evidence_json: evidence_json.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;
        Ok(id)
    }

    pub fn record_feedback(
        &self,
        run_id: Option<&str>,
        case_id: Option<&str>,
        source_kind: &str,
        source_id: &str,
        subject_kind: &str,
        subject_id: &str,
        content: &str,
    ) -> Result<String> {
        if subject_kind.trim().is_empty() || subject_id.trim().is_empty() || content.trim().is_empty()
        {
            return Err(anyhow!("Feedback requires a subject and non-empty content."));
        }
        if let Some(run_id) = run_id {
            self.db
                .get_orchestration_run(run_id)?
                .ok_or_else(|| anyhow!("Feedback references an unknown run."))?;
        }
        if let Some(case_id) = case_id {
            self.db
                .get_collaboration_case(case_id)?
                .ok_or_else(|| anyhow!("Feedback references an unknown collaboration case."))?;
        }
        let id = Uuid::new_v4().to_string();
        self.db.insert_feedback_record(&FeedbackRecord {
            id: id.clone(),
            run_id: run_id.map(str::to_string),
            case_id: case_id.map(str::to_string),
            source_kind: source_kind.to_string(),
            source_id: source_id.to_string(),
            subject_kind: subject_kind.to_string(),
            subject_id: subject_id.to_string(),
            content: content.to_string(),
            validation_status: "quarantined".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;
        Ok(id)
    }

    pub fn validate_lesson(
        &self,
        source_evaluation_id: Option<&str>,
        source_feedback_id: Option<&str>,
        scope_kind: &str,
        scope_id: &str,
        instruction: &str,
        validated_by: &str,
    ) -> Result<String> {
        self.require_learning_governor(validated_by)?;
        if source_evaluation_id.is_none() && source_feedback_id.is_none() {
            return Err(anyhow!(
                "A validated lesson must reference evaluation or feedback evidence."
            ));
        }
        if instruction.trim().is_empty() {
            return Err(anyhow!("A validated lesson cannot contain an empty instruction."));
        }
        if let Some(evaluation_id) = source_evaluation_id {
            self.db
                .get_run_evaluation(evaluation_id)?
                .ok_or_else(|| anyhow!("Lesson references an unknown evaluation."))?;
        }
        if let Some(feedback_id) = source_feedback_id {
            let feedback = self
                .db
                .get_feedback_record(feedback_id)?
                .ok_or_else(|| anyhow!("Lesson references unknown feedback."))?;
            if !matches!(feedback.validation_status.as_str(), "quarantined" | "validated") {
                return Err(anyhow!("Lesson references feedback that cannot be admitted."));
            }
        }
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.db.insert_lesson(&LessonRecord {
            id: id.clone(),
            source_evaluation_id: source_evaluation_id.map(str::to_string),
            source_feedback_id: source_feedback_id.map(str::to_string),
            scope_kind: scope_kind.to_string(),
            scope_id: scope_id.to_string(),
            instruction: instruction.to_string(),
            status: "validated".to_string(),
            validated_by: Some(validated_by.to_string()),
            created_at: now.clone(),
            updated_at: now,
        })?;
        if let Some(feedback_id) = source_feedback_id {
            self.db
                .update_feedback_validation_status(feedback_id, "validated")?;
        }
        Ok(id)
    }

    pub fn create_candidate(
        &self,
        candidate_kind: &str,
        lesson_id: &str,
        target_id: &str,
        proposed_definition_json: &str,
        risk_level: &str,
        created_by: &str,
    ) -> Result<String> {
        self.require_learning_governor(created_by)?;
        let lesson = self
            .db
            .get_lesson(lesson_id)?
            .ok_or_else(|| anyhow!("Candidate requires a persisted validated lesson."))?;
        if lesson.status != "validated" {
            return Err(anyhow!(
                "Candidate requires a validated lesson that is not yet active in production."
            ));
        }
        if !matches!(candidate_kind, "skill" | "workflow") {
            return Err(anyhow!("Candidate kind must be skill or workflow."));
        }
        let baseline_version_id = if candidate_kind == "skill" {
            let value: serde_json::Value = serde_json::from_str(proposed_definition_json)
                .map_err(|_| anyhow!("Skill candidate definition must be valid JSON."))?;
            if value
                .get("instructions")
                .and_then(|instructions| instructions.as_str())
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err(anyhow!(
                    "Skill candidate definition must contain non-empty instructions."
                ));
            }
            self.db
                .list_active_skill_versions()?
                .into_iter()
                .find(|version| version.skill_id == target_id)
                .map(|version| version.id)
        } else {
            crate::application::iflow_engine::engine::WorkflowEngine::parse_validated_draft(
                proposed_definition_json,
            )
            .map_err(|error| anyhow!("Workflow candidate rejected: {}", error))?;
            self.db
                .get_workflow(target_id)?
                .ok_or_else(|| anyhow!("Workflow candidate must target a reviewed draft."))?;
            self.db
                .get_latest_workflow_version_for_workflow(target_id)?
                .map(|version| version.id)
        };
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.db
            .insert_learning_candidate(&LearningCandidateRecord {
                id: id.clone(),
                candidate_kind: candidate_kind.to_string(),
                source_lesson_id: lesson_id.to_string(),
                target_id: target_id.to_string(),
                baseline_version_id,
                proposed_definition_json: proposed_definition_json.to_string(),
                risk_level: risk_level.to_string(),
                status: "draft".to_string(),
                created_by: created_by.to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            })?;
        if candidate_kind == "skill" {
            let definition: serde_json::Value = serde_json::from_str(proposed_definition_json)?;
            self.db.insert_skill_version(&SkillVersionRecord {
                id: Uuid::new_v4().to_string(),
                skill_id: target_id.to_string(),
                candidate_id: Some(id.clone()),
                version: self.db.next_skill_version_number(target_id)?,
                instructions: definition["instructions"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                activation_status: "candidate".to_string(),
                created_at: now,
            })?;
        }
        Ok(id)
    }

    pub fn record_benchmark(
        &self,
        candidate_id: &str,
        suite_id: &str,
        score: f64,
        regression_count: i64,
        result_json: &str,
        recorded_by: &str,
    ) -> Result<String> {
        self.require_learning_governor(recorded_by)?;
        let candidate = self.required_candidate(candidate_id)?;
        if !matches!(candidate.status.as_str(), "draft" | "benchmark_failed") {
            return Err(anyhow!(
                "Benchmark cannot be recorded from candidate state '{}'.",
                candidate.status
            ));
        }
        if suite_id.trim().is_empty() || !(0.0..=1.0).contains(&score) || regression_count < 0 {
            return Err(anyhow!("Benchmark requires a suite, normalized score, and valid regression count."));
        }
        let evidence: serde_json::Value = serde_json::from_str(result_json)
            .map_err(|_| anyhow!("Benchmark result must be a JSON evidence object."))?;
        let has_case_results = evidence
            .get("case_results")
            .and_then(|value| value.as_array())
            .is_some_and(|results| !results.is_empty());
        let has_safety_violation = evidence
            .get("safety_violations")
            .and_then(|value| value.as_array())
            .is_some_and(|violations| !violations.is_empty())
            || evidence
                .get("unauthorized_side_effects")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0;
        if !has_case_results {
            return Err(anyhow!("Benchmark evidence must contain case_results."));
        }
        let passed = score >= 0.8 && regression_count == 0 && !has_safety_violation;
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.db.insert_benchmark_run(&BenchmarkRunRecord {
            id: id.clone(),
            candidate_id: candidate_id.to_string(),
            suite_id: suite_id.to_string(),
            status: if passed { "passed" } else { "failed" }.to_string(),
            aggregate_score: Some(score),
            regression_count,
            result_json: result_json.to_string(),
            created_at: now.clone(),
            completed_at: Some(now),
        })?;
        self.transition_candidate(
            candidate_id,
            if passed {
                "benchmark_passed"
            } else {
                "benchmark_failed"
            },
        )?;
        Ok(id)
    }

    pub fn start_canary(
        &self,
        candidate_id: &str,
        scope_json: &str,
        traffic_percent: f64,
        started_by: &str,
    ) -> Result<String> {
        self.require_learning_governor(started_by)?;
        let candidate = self.required_candidate(candidate_id)?;
        if candidate.status != "benchmark_passed" {
            return Err(anyhow!("Canary requires a benchmark-passed candidate."));
        }
        let scope: serde_json::Value = serde_json::from_str(scope_json)
            .map_err(|_| anyhow!("Canary scope must be a JSON object."))?;
        if !scope.is_object() || scope.as_object().is_some_and(|scope| scope.is_empty()) {
            return Err(anyhow!("Canary requires an explicit bounded scope."));
        }
        if candidate.risk_level == "critical" {
            return Err(anyhow!("Critical-risk candidates cannot enter canary."));
        }
        if candidate.risk_level == "high" && traffic_percent > 5.0 {
            return Err(anyhow!("High-risk canary traffic cannot exceed 5 percent."));
        }
        if !(0.1..=25.0).contains(&traffic_percent) {
            return Err(anyhow!(
                "Canary traffic must be between 0.1 and 25 percent."
            ));
        }
        let id = Uuid::new_v4().to_string();
        self.db.insert_canary_deployment(&CanaryDeploymentRecord {
            id: id.clone(),
            candidate_id: candidate_id.to_string(),
            scope_json: scope_json.to_string(),
            traffic_percent,
            status: "running".to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
        })?;
        self.transition_candidate(candidate_id, "canary_running")?;
        Ok(id)
    }

    pub fn record_canary_outcome(
        &self,
        candidate_id: &str,
        deployment_id: &str,
        passed: bool,
        recorded_by: &str,
    ) -> Result<()> {
        self.require_learning_governor(recorded_by)?;
        let candidate = self.required_candidate(candidate_id)?;
        if candidate.status != "canary_running" {
            return Err(anyhow!(
                "Canary outcome requires an active canary deployment."
            ));
        }
        if !self
            .db
            .list_canary_deployments_for_candidate(candidate_id)?
            .iter()
            .any(|deployment| deployment.id == deployment_id && deployment.status == "running")
        {
            return Err(anyhow!(
                "Active deployment does not belong to the candidate."
            ));
        }
        self.db.update_canary_deployment_status(
            deployment_id,
            if passed { "passed" } else { "failed" },
        )?;
        self.transition_candidate(
            candidate_id,
            if passed {
                "canary_passed"
            } else {
                "canary_failed"
            },
        )?;
        Ok(())
    }

    pub fn promote(&self, candidate_id: &str, decided_by: &str, rationale: &str) -> Result<String> {
        self.require_learning_governor(decided_by)?;
        let candidate = self.required_candidate(candidate_id)?;
        if candidate.status != "canary_passed" {
            return Err(anyhow!("Promotion requires a successful canary outcome."));
        }
        if rationale.trim().is_empty() {
            return Err(anyhow!("Promotion requires a rationale."));
        }
        self.activate_candidate(&candidate)?;
        let id = Uuid::new_v4().to_string();
        self.db
            .insert_promotion_decision(&PromotionDecisionRecord {
                id: id.clone(),
                candidate_id: candidate_id.to_string(),
                decided_by: decided_by.to_string(),
                decision: "promoted".to_string(),
                rationale: rationale.to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            })?;
        self.transition_candidate(candidate_id, "promoted")?;
        self.db
            .update_lesson_status(&candidate.source_lesson_id, "active")?;
        Ok(id)
    }

    pub fn rollback(
        &self,
        candidate_id: &str,
        deployment_id: Option<&str>,
        initiated_by: &str,
        reason: &str,
    ) -> Result<String> {
        self.require_learning_governor(initiated_by)?;
        let candidate = self.required_candidate(candidate_id)?;
        if !matches!(
            candidate.status.as_str(),
            "promoted" | "canary_running" | "canary_passed"
        ) {
            return Err(anyhow!("Rollback applies only to deployed candidates."));
        }
        self.rollback_candidate_activation(&candidate)?;
        let id = Uuid::new_v4().to_string();
        self.db.insert_rollback_record(&RollbackRecord {
            id: id.clone(),
            candidate_id: candidate_id.to_string(),
            deployment_id: deployment_id.map(str::to_string),
            initiated_by: initiated_by.to_string(),
            reason: reason.to_string(),
            restored_version_id: candidate.baseline_version_id,
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;
        self.transition_candidate(candidate_id, "rolled_back")?;
        self.db
            .update_lesson_status(&candidate.source_lesson_id, "validated")?;
        Ok(id)
    }

    pub fn active_lessons_context(&self, instance_id: &str) -> Result<Vec<LessonRecord>> {
        let mut lessons = self
            .db
            .list_active_lessons(Some("instance"), Some(instance_id), 8)?;
        lessons.extend(self.db.list_active_lessons(Some("global"), Some(""), 8)?);
        lessons.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        lessons.dedup_by(|left, right| left.id == right.id);
        lessons.truncate(8);
        Ok(lessons)
    }

    fn required_candidate(&self, candidate_id: &str) -> Result<LearningCandidateRecord> {
        self.db
            .get_learning_candidate(candidate_id)?
            .ok_or_else(|| anyhow!("Learning candidate '{}' was not found.", candidate_id))
    }

    fn require_learning_governor(&self, actor_id: &str) -> Result<()> {
        if self
            .db
            .check_actor_permission(actor_id, "governance:learning:promote")
            .unwrap_or(false)
        {
            Ok(())
        } else {
            Err(anyhow!(
                "Governed learning decisions require an authorized human actor."
            ))
        }
    }

    fn activate_candidate(&self, candidate: &LearningCandidateRecord) -> Result<()> {
        match candidate.candidate_kind.as_str() {
            "skill" => {
                let version = self
                    .db
                    .get_skill_version_for_candidate(&candidate.id)?
                    .ok_or_else(|| anyhow!("Skill candidate has no immutable proposed version."))?;
                self.db.update_skill_version_activation(&version.id, "active")
            }
            "workflow" => {
                let mut parsed =
                    crate::application::iflow_engine::engine::WorkflowEngine::parse_validated_draft(
                        &candidate.proposed_definition_json,
                    )
                    .map_err(|error| anyhow!("Workflow candidate rejected: {}", error))?;
                parsed.id = candidate.target_id.clone();
                let definition = serde_json::to_string(&parsed)?;
                let existing = self
                    .db
                    .get_workflow(&candidate.target_id)?
                    .ok_or_else(|| anyhow!("Workflow candidate target does not exist."))?;
                self.db.upsert_workflow(&WorkflowRecord {
                    id: candidate.target_id.clone(),
                    run_id: existing.run_id,
                    origin_kind: "learned".to_string(),
                    activation_status: "active".to_string(),
                    name: parsed.name,
                    definition: definition.clone(),
                    version: parsed.version,
                    created_at: existing.created_at,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                })?;
                let instance_id = self
                    .db
                    .get_latest_workflow_version_for_workflow(&candidate.target_id)?
                    .map(|version| version.instance_id)
                    .ok_or_else(|| anyhow!("Workflow candidate requires a versioned draft."))?;
                self.db.save_workflow_version(&WorkflowVersionRecord {
                    id: Uuid::new_v4().to_string(),
                    workflow_id: candidate.target_id.clone(),
                    run_id: None,
                    instance_id,
                    version: self.db.next_workflow_version_number(&candidate.target_id)?,
                    definition_json: definition,
                    validation_status: "promoted".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                })
            }
            _ => Err(anyhow!("Unsupported learning candidate kind.")),
        }
    }

    fn rollback_candidate_activation(&self, candidate: &LearningCandidateRecord) -> Result<()> {
        match candidate.candidate_kind.as_str() {
            "skill" => {
                let version = self
                    .db
                    .get_skill_version_for_candidate(&candidate.id)?
                    .ok_or_else(|| anyhow!("Skill candidate version is missing."))?;
                self.db.update_skill_version_activation(&version.id, "rolled_back")?;
                if let Some(baseline) = candidate.baseline_version_id.as_deref() {
                    self.db.update_skill_version_activation(baseline, "active")?;
                }
                Ok(())
            }
            "workflow" => {
                let mut workflow = self
                    .db
                    .get_workflow(&candidate.target_id)?
                    .ok_or_else(|| anyhow!("Workflow candidate target is missing."))?;
                if let Some(baseline) = candidate.baseline_version_id.as_deref() {
                    let version = self
                        .db
                        .get_workflow_version(baseline)?
                        .ok_or_else(|| anyhow!("Workflow rollback baseline is missing."))?;
                    workflow.definition = version.definition_json;
                    workflow.activation_status = "active".to_string();
                } else {
                    workflow.activation_status = "review_required".to_string();
                }
                self.db.upsert_workflow(&workflow)
            }
            _ => Err(anyhow!("Unsupported learning candidate kind.")),
        }
    }

    fn transition_candidate(&self, candidate_id: &str, next_state: &str) -> Result<()> {
        let current = self.required_candidate(candidate_id)?;
        if !candidate_transition_allowed(&current.status, next_state) {
            return Err(anyhow!(
                "Learning candidate transition '{}' -> '{}' is not allowed.",
                current.status,
                next_state
            ));
        }
        self.db
            .update_learning_candidate_status(candidate_id, next_state)
    }
}

#[cfg(test)]
mod tests {
    use super::candidate_transition_allowed;

    #[test]
    fn candidate_pipeline_requires_canary_before_promotion() {
        assert!(!candidate_transition_allowed("draft", "promoted"));
        assert!(!candidate_transition_allowed(
            "benchmark_passed",
            "promoted"
        ));
        assert!(candidate_transition_allowed(
            "benchmark_passed",
            "canary_running"
        ));
        assert!(candidate_transition_allowed("canary_passed", "promoted"));
        assert!(candidate_transition_allowed("promoted", "rolled_back"));
    }
}
