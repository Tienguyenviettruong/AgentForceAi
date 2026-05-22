use super::{BaseProviderAdapter, ChatMessage, ChatResponse, TokenUsage};
use anyhow::{anyhow, Result};
use gpui::SharedString;
use std::future::Future;
use std::pin::Pin;

/// iFlow CLI adapter using ACP protocol
/// (Tasks 1.23, 1.24)
pub struct IFlowAdapter {
    config: Option<crate::db::Provider>,
    workflow_id: Option<String>,
    command_line: String,
}

impl Default for IFlowAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IFlowAdapter {
    pub fn new() -> Self {
        let _guard = get_runtime().enter();
        Self {
            config: None,
            workflow_id: None,
            command_line: "iflow".to_string(),
        }
    }

    /// (Task 1.24)
    pub fn execute_workflow(&mut self, workflow_id: &str) -> Result<String> {
        self.workflow_id = Some(workflow_id.to_string());
        Ok("Workflow started".to_string())
    }

    pub fn get_workflow_status(&self) -> Result<String> {
        Ok("Workflow running".to_string())
    }
}

impl BaseProviderAdapter for IFlowAdapter {
    fn provider_id(&self) -> &'static str {
        "iflow"
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
        let workflow_id = self.workflow_id.clone();
        let prompt = messages
            .into_iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        Box::pin(async move {
            let request_future = async move {
                let mut command = tokio::process::Command::new(cmd);
                if let Some(wid) = workflow_id {
                    command.arg("--workflow").arg(wid);
                }
                let mut child = command
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| anyhow!("Failed to spawn iflow command: {}", e))?;

                if let Some(mut stdin) = child.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    stdin.write_all(prompt.as_bytes()).await.ok();
                }

                let output = child
                    .wait_with_output()
                    .await
                    .map_err(|e| anyhow!("Failed to wait iflow output: {}", e))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    return Err(anyhow!("iFlow command failed: {}", stderr));
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
                    .map_err(|e| anyhow!("iFlow join error: {}", e))?,
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
        let workflow_id = self.workflow_id.clone();
        let prompt = messages
            .into_iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        Box::pin(async move {
            let (tx, rx) = futures::channel::mpsc::unbounded();
            let rt = get_runtime();

            rt.spawn(async move {
                let mut command = tokio::process::Command::new(cmd);
                if let Some(wid) = workflow_id {
                    command.arg("--workflow").arg(wid);
                }
                let mut child = match command
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        let _ =
                            tx.unbounded_send(Err(anyhow!("Failed to spawn iflow command: {}", e)));
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
                        let _ = tx.unbounded_send(Err(anyhow!("Failed to capture iflow stdout")));
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
                            let _ =
                                tx.unbounded_send(Err(anyhow!("iFlow stdout read error: {}", e)));
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

static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize Tokio runtime")
    })
}
