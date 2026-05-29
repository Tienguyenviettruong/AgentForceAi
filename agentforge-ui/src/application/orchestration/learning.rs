use crate::core::models::{
    BenchmarkRunRecord, BenchmarkRunnerJobRecord, CanaryDeploymentRecord, CanaryObservationRecord,
    FeedbackRecord, LearningCandidateRecord, LessonRecord, PromotionDecisionRecord, RollbackRecord,
    RunEvaluationRecord, SkillVersionRecord, WorkflowRecord, WorkflowVersionRecord,
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
            | ("draft", "benchmark_running")
            | ("benchmark_failed", "benchmark_running")
            | ("benchmark_running", "benchmark_passed")
            | ("benchmark_running", "benchmark_failed")
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
        let has_evidence = evidence.as_array().is_some_and(|values| !values.is_empty())
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
        let evaluation = RunEvaluationRecord {
            id: id.clone(),
            run_id: run_id.to_string(),
            rubric_id: None,
            evaluator_id: evaluator_id.to_string(),
            score,
            verdict: verdict.to_string(),
            evidence_json: evidence_json.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.db.insert_run_evaluation(&evaluation)?;
        if let Err(error) =
            self.collect_canary_observation_from_evaluation(&run, &evaluation, &evidence)
        {
            let _ = self
                .db
                .insert_run_event(&crate::core::models::RunEventRecord {
                    id: Uuid::new_v4().to_string(),
                    run_id: run.id.clone(),
                    event_type: "canary_observation_collection_failed".to_string(),
                    actor_type: "system".to_string(),
                    actor_id: Some(evaluator_id.to_string()),
                    task_id: None,
                    payload: Some(error.to_string()),
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
        }
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
        if subject_kind.trim().is_empty()
            || subject_id.trim().is_empty()
            || content.trim().is_empty()
        {
            return Err(anyhow!(
                "Feedback requires a subject and non-empty content."
            ));
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
            return Err(anyhow!(
                "A validated lesson cannot contain an empty instruction."
            ));
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
            if !matches!(
                feedback.validation_status.as_str(),
                "quarantined" | "validated"
            ) {
                return Err(anyhow!(
                    "Lesson references feedback that cannot be admitted."
                ));
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
                created_at: now.clone(),
            })?;
        }
        let auto_benchmark = self
            .db
            .get_setting("learning_auto_benchmark_on_candidate_create")
            .ok()
            .flatten()
            .map(|value| value != "false")
            .unwrap_or(true);
        if auto_benchmark {
            let _ = self
                .db
                .insert_benchmark_runner_job(&BenchmarkRunnerJobRecord {
                    id: Uuid::new_v4().to_string(),
                    candidate_id: id.clone(),
                    suite_id: String::new(),
                    status: "queued".to_string(),
                    requested_by: created_by.to_string(),
                    benchmark_run_id: None,
                    error: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    started_at: None,
                    completed_at: None,
                });
        }
        Ok(id)
    }

    pub(crate) fn begin_benchmark(
        &self,
        candidate_id: &str,
        requested_by: &str,
    ) -> Result<LearningCandidateRecord> {
        self.require_learning_governor(requested_by)?;
        let candidate = self.required_candidate(candidate_id)?;
        if !matches!(candidate.status.as_str(), "draft" | "benchmark_failed") {
            return Err(anyhow!(
                "Automatic benchmark cannot start from candidate state '{}'.",
                candidate.status
            ));
        }
        self.transition_candidate(candidate_id, "benchmark_running")?;
        self.required_candidate(candidate_id)
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
        if !matches!(
            candidate.status.as_str(),
            "draft" | "benchmark_failed" | "benchmark_running"
        ) {
            return Err(anyhow!(
                "Benchmark cannot be recorded from candidate state '{}'.",
                candidate.status
            ));
        }
        if suite_id.trim().is_empty() || !(0.0..=1.0).contains(&score) || regression_count < 0 {
            return Err(anyhow!(
                "Benchmark requires a suite, normalized score, and valid regression count."
            ));
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
        if !passed {
            let failed_runs = self
                .db
                .list_benchmark_runs_for_candidate(candidate_id)?
                .into_iter()
                .filter(|run| run.status == "failed")
                .count();
            if has_safety_violation || failed_runs >= 3 {
                self.db.insert_promotion_decision(&PromotionDecisionRecord {
                    id: Uuid::new_v4().to_string(),
                    candidate_id: candidate_id.to_string(),
                    decided_by: recorded_by.to_string(),
                    decision: "blocked_by_benchmark_circuit".to_string(),
                    rationale: if has_safety_violation {
                        "Automatic benchmark circuit breaker: safety violation or unauthorized side effect was detected."
                            .to_string()
                    } else {
                        format!(
                            "Automatic benchmark circuit breaker: {} failed benchmark runs.",
                            failed_runs
                        )
                    },
                    created_at: chrono::Utc::now().to_rfc3339(),
                })?;
            }
        }
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
        self.apply_canary_outcome(candidate_id, deployment_id, passed, recorded_by)
    }

    fn apply_canary_outcome(
        &self,
        candidate_id: &str,
        deployment_id: &str,
        passed: bool,
        recorded_by: &str,
    ) -> Result<()> {
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
        if passed {
            self.transition_candidate(candidate_id, "canary_passed")?;
            return Ok(());
        }

        self.rollback_candidate_activation(&candidate)?;
        self.db.insert_rollback_record(&RollbackRecord {
            id: Uuid::new_v4().to_string(),
            candidate_id: candidate_id.to_string(),
            deployment_id: Some(deployment_id.to_string()),
            initiated_by: recorded_by.to_string(),
            reason: "Automatic canary circuit breaker: failed canary outcome triggered pullback."
                .to_string(),
            restored_version_id: candidate.baseline_version_id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;
        self.transition_candidate(candidate_id, "rolled_back")?;
        self.db
            .update_lesson_status(&candidate.source_lesson_id, "validated")?;
        Ok(())
    }

    pub fn record_canary_observation(
        &self,
        deployment_id: &str,
        run_id: Option<&str>,
        metric_json: &str,
        recorded_by: &str,
    ) -> Result<String> {
        self.require_learning_governor(recorded_by)?;
        let deployment = self
            .find_canary_deployment(deployment_id)?
            .ok_or_else(|| anyhow!("Canary deployment was not found."))?;
        self.record_canary_observation_for_deployment(&deployment, run_id, metric_json, recorded_by)
    }

    fn record_canary_observation_for_deployment(
        &self,
        deployment: &CanaryDeploymentRecord,
        run_id: Option<&str>,
        metric_json: &str,
        recorded_by: &str,
    ) -> Result<String> {
        if deployment.status != "running" {
            return Err(anyhow!("Canary observation requires a running deployment."));
        }
        if let Some(run_id) = run_id {
            self.db
                .get_orchestration_run(run_id)?
                .ok_or_else(|| anyhow!("Canary observation references an unknown run."))?;
        }
        let metric: serde_json::Value = serde_json::from_str(metric_json)
            .map_err(|_| anyhow!("Canary metric_json must be valid JSON."))?;
        let quality_score = metric
            .get("quality_score")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        let error_rate = metric
            .get("error_rate")
            .and_then(|value| value.as_f64())
            .unwrap_or(1.0);
        let latency_ms = metric
            .get("latency_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        let max_latency_ms = self
            .db
            .get_setting("learning_canary_max_latency_ms")
            .ok()
            .flatten()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(30_000.0);
        let has_safety_violation = metric
            .get("safety_violations")
            .and_then(|value| value.as_array())
            .is_some_and(|violations| !violations.is_empty())
            || metric
                .get("unauthorized_side_effects")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0;
        let passed = quality_score >= 0.8
            && error_rate <= 0.05
            && latency_ms <= max_latency_ms
            && !has_safety_violation;
        let id = Uuid::new_v4().to_string();
        self.db
            .insert_canary_observation(&CanaryObservationRecord {
                id: id.clone(),
                deployment_id: deployment.id.clone(),
                run_id: run_id.map(str::to_string),
                metric_json: metric_json.to_string(),
                verdict: if passed { "pass" } else { "fail" }.to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            })?;

        if !passed {
            self.apply_canary_outcome(
                &deployment.candidate_id,
                &deployment.id,
                false,
                recorded_by,
            )?;
            return Ok(id);
        }

        let observations = self
            .db
            .list_canary_observations_for_deployment(&deployment.id)?;
        let required_passes = self
            .db
            .get_setting("learning_canary_required_passes")
            .ok()
            .flatten()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(3)
            .clamp(1, 100);
        let pass_count = observations
            .iter()
            .filter(|observation| observation.verdict == "pass")
            .count();
        let fail_count = observations
            .iter()
            .filter(|observation| observation.verdict == "fail")
            .count();
        if pass_count >= required_passes && fail_count == 0 {
            self.apply_canary_outcome(&deployment.candidate_id, &deployment.id, true, recorded_by)?;
        }

        Ok(id)
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

    fn find_canary_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<Option<CanaryDeploymentRecord>> {
        for candidate in self.db.list_recent_learning_candidates(500)? {
            if let Some(deployment) = self
                .db
                .list_canary_deployments_for_candidate(&candidate.id)?
                .into_iter()
                .find(|deployment| deployment.id == deployment_id)
            {
                return Ok(Some(deployment));
            }
        }
        Ok(None)
    }

    fn collect_canary_observation_from_evaluation(
        &self,
        run: &crate::core::models::OrchestrationRunRecord,
        evaluation: &RunEvaluationRecord,
        evidence: &serde_json::Value,
    ) -> Result<()> {
        for deployment in self.matching_running_canary_deployments(run)? {
            let metric = Self::canary_metric_from_evaluation(run, evaluation, evidence);
            self.record_canary_observation_for_deployment(
                &deployment,
                Some(run.id.as_str()),
                &metric.to_string(),
                &evaluation.evaluator_id,
            )?;
        }
        Ok(())
    }

    fn matching_running_canary_deployments(
        &self,
        run: &crate::core::models::OrchestrationRunRecord,
    ) -> Result<Vec<CanaryDeploymentRecord>> {
        let mut deployments = Vec::new();
        for candidate in self
            .db
            .list_recent_learning_candidates(500)?
            .into_iter()
            .filter(|candidate| candidate.status == "canary_running")
        {
            for deployment in self
                .db
                .list_canary_deployments_for_candidate(&candidate.id)?
                .into_iter()
                .filter(|deployment| deployment.status == "running")
            {
                if Self::canary_scope_matches_run(&deployment.scope_json, run) {
                    deployments.push(deployment);
                }
            }
        }
        Ok(deployments)
    }

    fn canary_scope_matches_run(
        scope_json: &str,
        run: &crate::core::models::OrchestrationRunRecord,
    ) -> bool {
        let Ok(scope) = serde_json::from_str::<serde_json::Value>(scope_json) else {
            return false;
        };
        let Some(scope) = scope.as_object() else {
            return false;
        };
        let mut has_runtime_constraint = false;

        if let Some(value) = scope.get("run_id").and_then(|value| value.as_str()) {
            has_runtime_constraint = true;
            if value != run.id {
                return false;
            }
        }
        if let Some(values) = scope.get("run_ids").and_then(|value| value.as_array()) {
            has_runtime_constraint = true;
            if !values
                .iter()
                .any(|value| value.as_str() == Some(run.id.as_str()))
            {
                return false;
            }
        }
        if let Some(value) = scope.get("instance_id").and_then(|value| value.as_str()) {
            has_runtime_constraint = true;
            if value != run.instance_id {
                return false;
            }
        }
        if let Some(value) = scope.get("mode").and_then(|value| value.as_str()) {
            has_runtime_constraint = true;
            if value != run.mode {
                return false;
            }
        }
        if let Some(value) = scope.get("workflow_id").and_then(|value| value.as_str()) {
            has_runtime_constraint = true;
            if run.workflow_id.as_deref() != Some(value) {
                return false;
            }
        }
        if let Some(values) = scope.get("workflow_ids").and_then(|value| value.as_array()) {
            has_runtime_constraint = true;
            let workflow_id = run.workflow_id.as_deref().unwrap_or_default();
            if !values
                .iter()
                .any(|value| value.as_str() == Some(workflow_id))
            {
                return false;
            }
        }

        has_runtime_constraint
    }

    fn canary_metric_from_evaluation(
        run: &crate::core::models::OrchestrationRunRecord,
        evaluation: &RunEvaluationRecord,
        evidence: &serde_json::Value,
    ) -> serde_json::Value {
        let verdict = evaluation.verdict.to_ascii_lowercase();
        let default_error_rate =
            if verdict.contains("pass") || verdict.contains("success") || evaluation.score >= 0.8 {
                0.0
            } else {
                (1.0 - evaluation.score).clamp(0.05, 1.0)
            };
        serde_json::json!({
            "source": "run_evaluation",
            "evaluation_id": evaluation.id.clone(),
            "quality_score": evaluation.score,
            "verdict": evaluation.verdict.clone(),
            "error_rate": Self::evidence_number(evidence, "error_rate").unwrap_or(default_error_rate),
            "latency_ms": Self::evidence_number(evidence, "latency_ms")
                .or_else(|| Self::run_latency_ms(run))
                .unwrap_or(0.0),
            "safety_violations": evidence
                .get("safety_violations")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
            "unauthorized_side_effects": evidence
                .get("unauthorized_side_effects")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(0)),
        })
    }

    fn evidence_number(evidence: &serde_json::Value, key: &str) -> Option<f64> {
        evidence.get(key).and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        })
    }

    fn run_latency_ms(run: &crate::core::models::OrchestrationRunRecord) -> Option<f64> {
        let created = chrono::DateTime::parse_from_rfc3339(&run.created_at).ok()?;
        let updated = chrono::DateTime::parse_from_rfc3339(&run.updated_at).ok()?;
        Some((updated - created).num_milliseconds().max(0) as f64)
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
                self.db
                    .update_skill_version_activation(&version.id, "active")
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
                self.db
                    .update_skill_version_activation(&version.id, "rolled_back")?;
                if let Some(baseline) = candidate.baseline_version_id.as_deref() {
                    self.db
                        .update_skill_version_activation(baseline, "active")?;
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
