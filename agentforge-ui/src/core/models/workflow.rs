#[derive(Clone, Debug)]
pub struct WorkflowRecord {
    pub id: String,
    pub name: String,
    pub definition: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}
