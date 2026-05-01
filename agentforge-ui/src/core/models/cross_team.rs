#[derive(Debug, Clone)]
pub struct CrossTeamCaseRecord {
    pub correlation_id: String,
    pub owner_instance_id: String,
    pub target_instance_id: String,
    pub latest_event_type: String,
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CrossTeamCaseEventRecord {
    pub id: String,
    pub correlation_id: String,
    pub from_instance_id: String,
    pub reply_to_instance_id: String,
    pub source_message_id: Option<String>,
    pub event_type: String,
    pub summary: String,
    pub payload: Option<String>,
    pub created_at: String,
}
