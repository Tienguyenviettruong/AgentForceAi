#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub team_id: String,
    pub instance_id: Option<String>,
    pub run_id: Option<String>,
    pub assignee_id: Option<String>,
    pub status: String,
    pub priority: String,
    pub payload: Option<String>,
    pub claimed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
