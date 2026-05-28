#[derive(Clone, Debug)]
pub struct WorkflowRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub origin_kind: String,
    pub activation_status: String,
    pub name: String,
    pub definition: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct WorkflowVersionRecord {
    pub id: String,
    pub workflow_id: String,
    pub run_id: Option<String>,
    pub instance_id: String,
    pub version: i64,
    pub definition_json: String,
    pub validation_status: String,
    pub created_at: String,
}

#[derive(Clone, Debug)]
pub struct WorkflowExecutionRecord {
    pub id: String,
    pub workflow_version_id: String,
    pub run_id: String,
    pub status: String,
    pub state_json: String,
    pub updated_at: String,
}
