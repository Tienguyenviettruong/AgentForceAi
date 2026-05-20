use crate::core::traits::database::DatabasePort;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::Arc;

pub struct OutputTools {
    db: Arc<dyn DatabasePort>,
    team_instance_id: String,
    client: reqwest::Client,
}

impl OutputTools {
    pub fn new(db: Arc<dyn DatabasePort>, team_instance_id: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            db,
            team_instance_id,
            client,
        }
    }

    pub async fn generate_image(&self, args: &Value) -> Result<String> {
        let prompt = required_arg(args, "prompt")?;
        let endpoint = self
            .setting_or_env("output_image_endpoint", "AGENTFORGE_IMAGE_OUTPUT_URL")
            .unwrap_or_else(|| "https://api.openai.com/v1/images/generations".to_string());
        let api_key = self
            .setting_or_env("output_image_api_key", "AGENTFORGE_IMAGE_OUTPUT_API_KEY")
            .or_else(|| {
                std::env::var("OPENAI_API_KEY")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
            });

        if endpoint.contains("api.openai.com") && api_key.is_none() {
            return Err(anyhow!(
                "Image output service requires an API key. Set output_image_api_key or OPENAI_API_KEY."
            ));
        }

        let model = arg_string(args, "model")
            .or_else(|| self.setting_or_env("output_image_model", "AGENTFORGE_IMAGE_OUTPUT_MODEL"))
            .unwrap_or_else(|| "gpt-image-1".to_string());
        let size = arg_string(args, "size").unwrap_or_else(|| "1024x1024".to_string());
        let format = arg_string(args, "format").unwrap_or_else(|| "png".to_string());

        let mut payload = Map::new();
        payload.insert("model".to_string(), Value::String(model));
        payload.insert("prompt".to_string(), Value::String(prompt));
        payload.insert("size".to_string(), Value::String(size));
        payload.insert("n".to_string(), Value::from(1));
        merge_options(args, &mut payload);

        let bytes = self
            .post_for_artifact(&endpoint, api_key.as_deref(), &Value::Object(payload))
            .await?;
        let path = self.save_artifact(args, "images", &format, &bytes)?;

        Ok(format!("Image generated: {}", path.display()))
    }

    pub async fn render_pdf(&self, args: &Value) -> Result<String> {
        let endpoint = self
            .setting_or_env("output_pdf_endpoint", "AGENTFORGE_PDF_OUTPUT_URL")
            .ok_or_else(|| {
                anyhow!(
                    "PDF output service is not configured. Set output_pdf_endpoint or AGENTFORGE_PDF_OUTPUT_URL."
                )
            })?;
        let api_key = self.setting_or_env("output_pdf_api_key", "AGENTFORGE_PDF_OUTPUT_API_KEY");

        let (content, source_format) = if let Some(html) = arg_string(args, "html") {
            (html, "html".to_string())
        } else if let Some(markdown) = arg_string(args, "markdown") {
            (markdown, "markdown".to_string())
        } else {
            (
                required_arg(args, "content")?,
                arg_string(args, "source_format").unwrap_or_else(|| "markdown".to_string()),
            )
        };

        let mut payload = Map::new();
        payload.insert("content".to_string(), Value::String(content));
        payload.insert("source_format".to_string(), Value::String(source_format));
        payload.insert(
            "output_format".to_string(),
            Value::String("pdf".to_string()),
        );
        merge_options(args, &mut payload);

        let bytes = self
            .post_for_artifact(&endpoint, api_key.as_deref(), &Value::Object(payload))
            .await?;
        let path = self.save_artifact(args, "docs", "pdf", &bytes)?;

        Ok(format!("PDF generated: {}", path.display()))
    }

    pub async fn generate_video(&self, args: &Value) -> Result<String> {
        let prompt = required_arg(args, "prompt")?;
        let endpoint = self
            .setting_or_env("output_video_endpoint", "AGENTFORGE_VIDEO_OUTPUT_URL")
            .ok_or_else(|| {
                anyhow!(
                    "Video output service is not configured. Set output_video_endpoint or AGENTFORGE_VIDEO_OUTPUT_URL."
                )
            })?;
        let api_key =
            self.setting_or_env("output_video_api_key", "AGENTFORGE_VIDEO_OUTPUT_API_KEY");

        let mut payload = Map::new();
        payload.insert("prompt".to_string(), Value::String(prompt));
        if let Some(model) = arg_string(args, "model") {
            payload.insert("model".to_string(), Value::String(model));
        }
        if let Some(size) = arg_string(args, "size") {
            payload.insert("size".to_string(), Value::String(size));
        }
        if let Some(duration) = args.get("duration_seconds").cloned() {
            payload.insert("duration_seconds".to_string(), duration);
        }
        if let Some(input_image) = arg_string(args, "input_image") {
            payload.insert("input_image".to_string(), Value::String(input_image));
        }
        merge_options(args, &mut payload);

        let bytes = self
            .post_for_artifact(&endpoint, api_key.as_deref(), &Value::Object(payload))
            .await?;
        let path = self.save_artifact(args, "videos", "mp4", &bytes)?;

        Ok(format!("Video generated: {}", path.display()))
    }

    fn setting_or_env(&self, setting_key: &str, env_key: &str) -> Option<String> {
        self.db
            .get_setting(setting_key)
            .ok()
            .flatten()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| std::env::var(env_key).ok().filter(|v| !v.trim().is_empty()))
    }

    async fn post_for_artifact(
        &self,
        endpoint: &str,
        api_key: Option<&str>,
        payload: &Value,
    ) -> Result<Vec<u8>> {
        let mut request = self.client.post(endpoint).json(payload);
        if let Some(api_key) = api_key.filter(|v| !v.trim().is_empty()) {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        let bytes = response.bytes().await?;

        if !status.is_success() {
            let body = String::from_utf8_lossy(&bytes);
            return Err(anyhow!(
                "Output service returned HTTP {}: {}",
                status,
                truncate(&body, 800)
            ));
        }

        let trimmed = bytes
            .iter()
            .copied()
            .skip_while(|b| b.is_ascii_whitespace())
            .next();
        let looks_json =
            content_type.contains("json") || matches!(trimmed, Some(b'{') | Some(b'['));
        if !looks_json {
            return Ok(bytes.to_vec());
        }

        let json: Value = serde_json::from_slice(&bytes)?;
        self.artifact_from_json(&json)
            .await
            .with_context_json(&json)
    }

    async fn artifact_from_json(&self, json: &Value) -> Result<Vec<u8>> {
        if let Some(encoded) = find_string_key(
            json,
            &[
                "b64_json",
                "base64",
                "base64_data",
                "file_base64",
                "content_base64",
                "artifact_base64",
            ],
        ) {
            return decode_base64_artifact(&encoded);
        }

        if let Some(data_url) = find_data_url(json) {
            return decode_base64_artifact(&data_url);
        }

        if let Some(url) = find_string_key(
            json,
            &[
                "download_url",
                "file_url",
                "artifact_url",
                "video_url",
                "image_url",
                "url",
            ],
        ) {
            let response = self.client.get(url).send().await?;
            let status = response.status();
            let bytes = response.bytes().await?;
            if !status.is_success() {
                return Err(anyhow!("Artifact download returned HTTP {}", status));
            }
            return Ok(bytes.to_vec());
        }

        Err(anyhow!(
            "Output service response did not include binary data, base64 data, or a download URL."
        ))
    }

    fn save_artifact(
        &self,
        args: &Value,
        category: &str,
        extension: &str,
        bytes: &[u8],
    ) -> Result<PathBuf> {
        let requested_path = arg_string(args, "path");
        let path = self.resolve_output_path(requested_path.as_deref(), category, extension)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
        Ok(path)
    }

    fn resolve_output_path(
        &self,
        requested_path: Option<&str>,
        category: &str,
        extension: &str,
    ) -> Result<PathBuf> {
        let workspace = self.workspace_dir();

        if let Some(requested_path) = requested_path.filter(|v| !v.trim().is_empty()) {
            let path = PathBuf::from(requested_path);
            if path.is_absolute() {
                return Ok(path);
            }
            if let Some(workspace) = workspace {
                return Ok(workspace.join(path));
            }
            return Ok(std::env::current_dir()?.join(path));
        }

        let output_dir = self
            .setting_or_env("output_tools_dir", "AGENTFORGE_OUTPUT_TOOLS_DIR")
            .unwrap_or_else(|| "outputs".to_string());
        let output_dir = PathBuf::from(output_dir);
        let base = if output_dir.is_absolute() {
            output_dir
        } else if let Some(workspace) = workspace {
            workspace.join(output_dir)
        } else {
            std::env::current_dir()?.join(output_dir)
        };

        Ok(base.join(category).join(format!(
            "{}.{}",
            uuid::Uuid::new_v4(),
            extension.trim_start_matches('.')
        )))
    }

    fn workspace_dir(&self) -> Option<PathBuf> {
        self.db
            .get_setting(&format!("workspace_{}", self.team_instance_id))
            .ok()
            .flatten()
            .filter(|v| !v.trim().is_empty())
            .map(PathBuf::from)
    }
}

trait ArtifactJsonContext<T> {
    fn with_context_json(self, json: &Value) -> Result<T>;
}

impl<T> ArtifactJsonContext<T> for Result<T> {
    fn with_context_json(self, json: &Value) -> Result<T> {
        self.map_err(|e| anyhow!("{} Response: {}", e, truncate(&json.to_string(), 800)))
    }
}

fn required_arg(args: &Value, key: &str) -> Result<String> {
    arg_string(args, key)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| anyhow!("Missing required argument: {}", key))
}

fn arg_string(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|value| match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    })
}

fn merge_options(args: &Value, payload: &mut Map<String, Value>) {
    if let Some(options) = args.get("options").and_then(|v| v.as_object()) {
        for (key, value) in options {
            payload.insert(key.clone(), value.clone());
        }
    }
}

fn decode_base64_artifact(encoded: &str) -> Result<Vec<u8>> {
    let encoded = encoded.trim();
    let encoded = encoded
        .strip_prefix("data:")
        .and_then(|rest| rest.split_once(',').map(|(_, data)| data))
        .unwrap_or(encoded);

    STANDARD
        .decode(encoded)
        .map_err(|e| anyhow!("Failed to decode base64 artifact: {}", e))
}

fn find_string_key(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(Value::String(value)) = map.get(*key) {
                    if !value.trim().is_empty() {
                        return Some(value.clone());
                    }
                }
            }
            for child in map.values() {
                if let Some(value) = find_string_key(child, keys) {
                    return Some(value);
                }
            }
            None
        }
        Value::Array(values) => {
            for child in values {
                if let Some(value) = find_string_key(child, keys) {
                    return Some(value);
                }
            }
            None
        }
        _ => None,
    }
}

fn find_data_url(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if value.trim_start().starts_with("data:") => Some(value.clone()),
        Value::Object(map) => {
            for child in map.values() {
                if let Some(value) = find_data_url(child) {
                    return Some(value);
                }
            }
            None
        }
        Value::Array(values) => {
            for child in values {
                if let Some(value) = find_data_url(child) {
                    return Some(value);
                }
            }
            None
        }
        _ => None,
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}
