use gpui::SharedString;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: SharedString,
    pub content: SharedString,
    pub agent_name: Option<SharedString>,
    pub thought_duration_secs: Option<f64>,
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
