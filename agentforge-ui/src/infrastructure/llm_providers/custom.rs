use super::{BaseProviderAdapter, ChatMessage, ChatResponse, TokenUsage};
use anyhow::Result;
use gpui::SharedString;
use std::future::Future;
use std::pin::Pin;

/// Custom provider adapter framework and SDK
/// (Task 1.26)
pub struct CustomAdapterSDK {
    config: Option<crate::db::Provider>,
    _plugin_name: String,
}

impl CustomAdapterSDK {
    pub fn new(plugin_name: &str) -> Self {
        Self {
            config: None,
            _plugin_name: plugin_name.to_string(),
        }
    }
}

impl BaseProviderAdapter for CustomAdapterSDK {
    fn provider_id(&self) -> &'static str {
        // Normally this would be dynamic based on the plugin, but returning a static str
        // to satisfy the trait for this mock SDK. In a real scenario we'd use a leak or
        // Box::leak(self.plugin_name.clone().into_boxed_str())
        "custom"
    }

    fn initialize(&mut self, config: &crate::db::Provider) -> Result<()> {
        self.config = Some(config.clone());
        Ok(())
    }

    fn send_message(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse>> + Send>> {
        Box::pin(async move {
            Ok(ChatResponse {
                content: SharedString::from("Custom adapter SDK response"),
                token_usage: TokenUsage::default(),
            })
        })
    }

    fn send_message_stream(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Box<
                            dyn futures::Stream<
                                    Item = Result<crate::providers::StreamChunk, anyhow::Error>,
                                > + Send
                                + Unpin,
                        >,
                    >,
                > + Send,
        >,
    > {
        Box::pin(async move {
            let stream = futures::stream::iter(vec![
                Ok(crate::providers::StreamChunk::Text(
                    "Custom adapter SDK stream".to_string(),
                )),
                Ok(crate::providers::StreamChunk::Done(
                    crate::providers::TokenUsage::default(),
                )),
            ]);
            Ok(Box::new(stream)
                as Box<
                    dyn futures::Stream<Item = Result<crate::providers::StreamChunk, anyhow::Error>>
                        + Send
                        + Unpin,
                >)
        })
    }

    fn check_health(&self) -> Pin<Box<dyn Future<Output = Result<bool>> + Send>> {
        Box::pin(async move { Ok(true) })
    }
}
