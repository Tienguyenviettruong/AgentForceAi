#[derive(Clone, Debug)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub objectives: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub team_id: String,
    pub config: Option<String>,
    pub state: Option<String>,
    pub created_at: String,
}
