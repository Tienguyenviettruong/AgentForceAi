use crate::core::errors::CoreError;
use crate::core::models::{ChatMessage, ChatResponse, StreamChunk};
use futures::Stream;
use std::future::Future;
use std::pin::Pin;

pub trait LlmProviderPort: Send + Sync {
    fn provider_id(&self) -> &'static str;

    fn initialize(&mut self, config: &crate::core::models::Provider) -> Result<(), CoreError>;

    fn send_message(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, CoreError>> + Send>>;

    fn send_message_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Box<dyn Stream<Item = Result<StreamChunk, CoreError>> + Send + Unpin>,
                        CoreError,
                    >,
                > + Send,
        >,
    >;

    fn check_health(&self) -> Pin<Box<dyn Future<Output = Result<bool, CoreError>> + Send>>;
}
