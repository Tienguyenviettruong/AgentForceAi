#[derive(Debug, Clone)]
pub struct EvaluationRubricRecord {
    pub id: String,
    pub name: String,
    pub version: i64,
    pub criteria_json: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct RunEvaluationRecord {
    pub id: String,
    pub run_id: String,
    pub rubric_id: Option<String>,
    pub evaluator_id: String,
    pub score: f64,
    pub verdict: String,
    pub evidence_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct FeedbackRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub case_id: Option<String>,
    pub source_kind: String,
    pub source_id: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub content: String,
    pub validation_status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct LessonRecord {
    pub id: String,
    pub source_evaluation_id: Option<String>,
    pub source_feedback_id: Option<String>,
    pub scope_kind: String,
    pub scope_id: String,
    pub instruction: String,
    pub status: String,
    pub validated_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct LearningCandidateRecord {
    pub id: String,
    pub candidate_kind: String,
    pub source_lesson_id: String,
    pub target_id: String,
    pub baseline_version_id: Option<String>,
    pub proposed_definition_json: String,
    pub risk_level: String,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SkillVersionRecord {
    pub id: String,
    pub skill_id: String,
    pub candidate_id: Option<String>,
    pub version: i64,
    pub instructions: String,
    pub activation_status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct BenchmarkRunRecord {
    pub id: String,
    pub candidate_id: String,
    pub suite_id: String,
    pub status: String,
    pub aggregate_score: Option<f64>,
    pub regression_count: i64,
    pub result_json: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkSuiteRecord {
    pub id: String,
    pub name: String,
    pub version: i64,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct BenchmarkCaseRecord {
    pub id: String,
    pub suite_id: String,
    pub input_json: String,
    pub expectation_json: String,
    pub risk_level: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct BenchmarkResultRecord {
    pub id: String,
    pub benchmark_run_id: String,
    pub benchmark_case_id: String,
    pub score: f64,
    pub verdict: String,
    pub evidence_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct BenchmarkRunnerJobRecord {
    pub id: String,
    pub candidate_id: String,
    pub suite_id: String,
    pub status: String,
    pub requested_by: String,
    pub benchmark_run_id: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CanaryDeploymentRecord {
    pub id: String,
    pub candidate_id: String,
    pub scope_json: String,
    pub traffic_percent: f64,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CanaryObservationRecord {
    pub id: String,
    pub deployment_id: String,
    pub run_id: Option<String>,
    pub metric_json: String,
    pub verdict: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct PromotionDecisionRecord {
    pub id: String,
    pub candidate_id: String,
    pub decided_by: String,
    pub decision: String,
    pub rationale: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct RollbackRecord {
    pub id: String,
    pub candidate_id: String,
    pub deployment_id: Option<String>,
    pub initiated_by: String,
    pub reason: String,
    pub restored_version_id: Option<String>,
    pub created_at: String,
}
