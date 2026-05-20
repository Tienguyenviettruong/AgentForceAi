use super::{BaseProviderAdapter, ChatMessage, ChatResponse, TokenUsage};
use anyhow::{anyhow, Result};
use futures::stream::StreamExt;
use gpui::SharedString;
use std::future::Future;
use std::pin::Pin;

/// OpenCode adapter with configurable command line
/// (Task 1.25)
pub struct OpenCodeAdapter {
    config: Option<crate::db::Provider>,
    command_line: String,
}

impl Default for OpenCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize Tokio runtime")
    })
}

impl OpenCodeAdapter {
    pub fn new() -> Self {
        let _guard = get_runtime().enter();
        Self {
            config: None,
            command_line: "opencode".to_string(),
        }
    }

    /// Set configurable command line
    pub fn set_command_line(&mut self, cmd: &str) {
        self.command_line = cmd.to_string();
    }
}

impl BaseProviderAdapter for OpenCodeAdapter {
    fn provider_id(&self) -> &'static str {
        "opencode"
    }

    fn initialize(&mut self, config: &crate::db::Provider) -> Result<()> {
        self.config = Some(config.clone());
        if let Some(cmd) = &config.command {
            self.command_line = cmd.clone();
        }
        Ok(())
    }

    fn send_message(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatResponse>> + Send>> {
        let cmd = self.command_line.clone();
        let prompt = messages
            .into_iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        Box::pin(async move {
            let request_future = async move {
                let mut child = tokio::process::Command::new(cmd)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| anyhow!("Failed to spawn opencode command: {}", e))?;

                if let Some(mut stdin) = child.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    stdin.write_all(prompt.as_bytes()).await.ok();
                }

                let output = child
                    .wait_with_output()
                    .await
                    .map_err(|e| anyhow!("Failed to wait opencode output: {}", e))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    return Err(anyhow!("OpenCode command failed: {}", stderr));
                }

                Ok(ChatResponse {
                    content: SharedString::from(
                        String::from_utf8_lossy(&output.stdout).to_string(),
                    ),
                    token_usage: TokenUsage::default(),
                })
            };

            match tokio::runtime::Handle::try_current() {
                Ok(_) => request_future.await,
                Err(_) => get_runtime()
                    .spawn(request_future)
                    .await
                    .map_err(|e| anyhow!("OpenCode join error: {}", e))?,
            }
        })
    }

    fn send_message_stream(
        &self,
        messages: Vec<ChatMessage>,
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
        let cmd = self.command_line.clone();
        let prompt = messages
            .into_iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        Box::pin(async move {
            let (tx, rx) = futures::channel::mpsc::unbounded();
            let rt = get_runtime();

            rt.spawn(async move {
                let mut child = match tokio::process::Command::new(cmd)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.unbounded_send(Err(anyhow!(
                            "Failed to spawn opencode command: {}",
                            e
                        )));
                        return;
                    }
                };

                if let Some(mut stdin) = child.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    let _ = stdin.write_all(prompt.as_bytes()).await;
                }

                let stdout = match child.stdout.take() {
                    Some(s) => s,
                    None => {
                        let _ =
                            tx.unbounded_send(Err(anyhow!("Failed to capture opencode stdout")));
                        return;
                    }
                };

                let mut reader = tokio::io::BufReader::new(stdout);
                let mut buf = vec![0u8; 1024];
                loop {
                    match tokio::io::AsyncReadExt::read(&mut reader, &mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]).to_string();
                            let _ =
                                tx.unbounded_send(Ok(crate::providers::StreamChunk::Text(text)));
                        }
                        Err(e) => {
                            let _ = tx
                                .unbounded_send(Err(anyhow!("OpenCode stdout read error: {}", e)));
                            break;
                        }
                    }
                }

                let _ = tx.unbounded_send(Ok(crate::providers::StreamChunk::Done(
                    crate::providers::TokenUsage::default(),
                )));
            });

            Ok(Box::new(rx)
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
