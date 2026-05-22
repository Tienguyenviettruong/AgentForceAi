use gpui::SharedString;

/// Một phần nội dung trong tin nhắn multimodal.
#[derive(Clone, Debug)]
pub enum ContentPart {
    /// Nội dung text thông thường.
    Text(String),
    /// Dữ liệu hình ảnh nhúng trực tiếp, pass native sang model vision.
    ImageBytes { mime_type: String, data: Vec<u8> },
    /// URL hình ảnh công khai (model fetch từ URL).
    ImageUrl(String),
    /// Dữ liệu audio (MP3, WAV, ...).
    AudioBytes { mime_type: String, data: Vec<u8> },
    /// Dữ liệu video.
    VideoBytes { mime_type: String, data: Vec<u8> },
    /// Dữ liệu PDF nhúng trực tiếp.
    PdfBytes { data: Vec<u8> },
}

impl ContentPart {
    /// Trả về text content nếu là Text part.
    pub fn as_text(&self) -> Option<&str> {
        if let ContentPart::Text(t) = self {
            Some(t.as_str())
        } else {
            None
        }
    }

    /// Kiểm tra part này có phải binary media không.
    pub fn is_media(&self) -> bool {
        !matches!(self, ContentPart::Text(_) | ContentPart::ImageUrl(_))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ChatMessage {
    pub role: SharedString,
    /// Legacy field – text-only content (backward compatible).
    /// Với multimodal messages, dùng `parts` thay thế.
    pub content: SharedString,
    /// Danh sách content parts (multimodal). Nếu empty, dùng `content`.
    pub parts: Vec<ContentPart>,
    pub agent_name: Option<SharedString>,
    pub thought_duration_secs: Option<f64>,
}

impl ChatMessage {
    /// Tạo message text-only (backward compatible).
    pub fn new_text(role: impl Into<SharedString>, content: impl Into<SharedString>) -> Self {
        let content = content.into();
        Self {
            role: role.into(),
            content: content.clone(),
            parts: vec![ContentPart::Text(content.to_string())],
            agent_name: None,
            thought_duration_secs: None,
        }
    }

    /// Tạo message multimodal với danh sách parts.
    pub fn new_multimodal(role: impl Into<SharedString>, parts: Vec<ContentPart>) -> Self {
        // Gom text để populate legacy content field
        let text: String = parts
            .iter()
            .filter_map(|p| p.as_text())
            .collect::<Vec<_>>()
            .join(" ");
        Self {
            role: role.into(),
            content: SharedString::from(text),
            parts,
            agent_name: None,
            thought_duration_secs: None,
        }
    }

    /// Lấy text content (gom từ tất cả Text parts, hoặc legacy content).
    pub fn content_text(&self) -> String {
        if !self.parts.is_empty() {
            let collected: String = self
                .parts
                .iter()
                .filter_map(|p| p.as_text())
                .collect::<Vec<_>>()
                .join("\n");
            if !collected.is_empty() {
                return collected;
            }
        }
        self.content.to_string()
    }

    /// Kiểm tra message có chứa media (image/audio/video/pdf) không.
    pub fn has_media(&self) -> bool {
        self.parts.iter().any(|p| p.is_media())
    }

    /// Lấy tất cả image parts.
    pub fn image_parts(&self) -> Vec<&ContentPart> {
        self.parts
            .iter()
            .filter(|p| matches!(p, ContentPart::ImageBytes { .. } | ContentPart::ImageUrl(_)))
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Clone, Debug)]
pub enum StreamChunk {
    Text(String),
    Done(TokenUsage),
}

#[derive(Clone, Debug)]
pub struct ChatResponse {
    pub content: SharedString,
    pub token_usage: TokenUsage,
}
