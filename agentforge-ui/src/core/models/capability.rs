use serde::{Deserialize, Serialize};

/// Các loại input modality mà một model có thể hỗ trợ.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Image,
    Audio,
    Video,
    Pdf,
}

/// Khả năng của một model/provider cụ thể.
/// Được lưu dạng JSON trong DB column `capabilities`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    /// Danh sách modality mà model hỗ trợ (luôn bao gồm Text).
    pub supported_modalities: Vec<Modality>,
    /// Model có hỗ trợ tool/function calling không?
    pub supports_tool_calling: bool,
    /// Model có hỗ trợ JSON schema output không?
    pub supports_json_schema: bool,
    /// Model có hỗ trợ streaming response không?
    pub supports_streaming: bool,
    /// Context window tối đa (tokens). None = unknown.
    pub max_context_tokens: Option<usize>,
}

impl Default for ModelCapability {
    fn default() -> Self {
        Self::text_only()
    }
}

impl ModelCapability {
    /// Model chỉ hiểu text (Ollama text-only, local LLMs không có vision).
    pub fn text_only() -> Self {
        Self {
            supported_modalities: vec![Modality::Text],
            supports_tool_calling: false,
            supports_json_schema: false,
            supports_streaming: true,
            max_context_tokens: None,
        }
    }

    /// Model hiểu text + image (vision models như llava, GPT-4V...).
    pub fn vision() -> Self {
        Self {
            supported_modalities: vec![Modality::Text, Modality::Image],
            supports_tool_calling: true,
            supports_json_schema: true,
            supports_streaming: true,
            max_context_tokens: None,
        }
    }

    /// Model Gemini 1.5/2.x – full multimodal.
    pub fn gemini_multimodal() -> Self {
        Self {
            supported_modalities: vec![
                Modality::Text,
                Modality::Image,
                Modality::Audio,
                Modality::Video,
                Modality::Pdf,
            ],
            supports_tool_calling: true,
            supports_json_schema: true,
            supports_streaming: true,
            max_context_tokens: Some(1_000_000),
        }
    }

    /// Model Claude 3+ – text + image + pdf.
    pub fn claude_multimodal() -> Self {
        Self {
            supported_modalities: vec![Modality::Text, Modality::Image, Modality::Pdf],
            supports_tool_calling: true,
            supports_json_schema: true,
            supports_streaming: true,
            max_context_tokens: Some(200_000),
        }
    }

    /// Kiểm tra xem model có hỗ trợ modality cụ thể không.
    pub fn supports(&self, modality: &Modality) -> bool {
        self.supported_modalities.contains(modality)
    }

    /// Deserialize từ JSON string (lưu trong DB).
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    /// Serialize sang JSON string để lưu vào DB.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}
