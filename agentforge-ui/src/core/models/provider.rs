#[derive(Clone, Debug)]
pub struct Provider {
    pub id: String,
    pub provider_name: String,
    pub model: String,
    pub adapter_type: String,
    pub command: Option<String>,
    pub api_key_ref: Option<String>,
    pub status: String,
    /// Khả năng của model: vision, audio, pdf... Lưu dạng JSON trong DB.
    /// None = text-only (mặc định an toàn cho local models).
    pub capabilities: Option<crate::core::models::ModelCapability>,
}

#[derive(Clone, Debug)]
pub struct ProviderTemplate {
    pub id: String,
    pub label: String,
    pub protocol: String,
    pub adapter: String,
    pub models: Vec<String>,
    pub default_base_url: String,
}
