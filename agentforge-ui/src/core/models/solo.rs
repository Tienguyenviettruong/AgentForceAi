#[derive(Clone, Debug)]
pub struct SoloProjectRecord {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct SoloConversationRecord {
    pub id: i64,
    pub project_id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct SoloMessageRecord {
    pub role: String,
    pub content: String,
    pub metadata: Option<String>,
}
