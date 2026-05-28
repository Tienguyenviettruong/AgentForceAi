use crate::application::orchestration::tool_gateway::ToolExecutionGateway;
use crate::core::traits::database::DatabasePort;
use crate::infrastructure::security::keychain::resolve_credential_reference;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;

pub struct OutputTools {
    db: Arc<dyn DatabasePort>,
    team_instance_id: String,
    run_id: Option<String>,
    session_id: Option<String>,
    agent_id: Option<String>,
    invocation_id: Option<String>,
    client: reqwest::Client,
}

impl OutputTools {
    pub fn new(
        db: Arc<dyn DatabasePort>,
        team_instance_id: String,
        run_id: Option<String>,
        session_id: Option<String>,
        agent_id: Option<String>,
        invocation_id: Option<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            db,
            team_instance_id,
            run_id,
            session_id,
            agent_id,
            invocation_id,
            client,
        }
    }

    pub async fn generate_image(&self, args: &Value) -> Result<String> {
        let prompt = required_arg(args, "prompt")?;
        let endpoint = self
            .setting_or_env("output_image_endpoint", "AGENTFORGE_IMAGE_OUTPUT_URL")
            .unwrap_or_else(|| "https://api.openai.com/v1/images/generations".to_string());
        let api_key = self
            .output_credential(
                "output_image_api_key_ref",
                "output_image_api_key",
                &["AGENTFORGE_IMAGE_OUTPUT_API_KEY", "OPENAI_API_KEY"],
            )
            .await?;

        if endpoint.contains("api.openai.com") && api_key.is_none() {
            return Err(anyhow!(
                "Image output service requires an API key. Set output_image_api_key_ref to secret:// or env:, or set OPENAI_API_KEY."
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
        let api_key = self
            .output_credential(
                "output_pdf_api_key_ref",
                "output_pdf_api_key",
                &["AGENTFORGE_PDF_OUTPUT_API_KEY"],
            )
            .await?;

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
        let api_key = self
            .output_credential(
                "output_video_api_key_ref",
                "output_video_api_key",
                &["AGENTFORGE_VIDEO_OUTPUT_API_KEY"],
            )
            .await?;

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

    pub async fn render_document(&self, args: &Value) -> Result<String> {
        let format = arg_string(args, "format")
            .or_else(|| arg_string(args, "output_format"))
            .unwrap_or_else(|| "markdown".to_string())
            .to_ascii_lowercase();
        let content = arg_string(args, "html")
            .or_else(|| arg_string(args, "markdown"))
            .or_else(|| arg_string(args, "content"))
            .unwrap_or_default();

        let (normalized_format, extension, bytes) = match format.as_str() {
            "html" | "htm" => ("html", "html", normalize_html_document(&content).into_bytes()),
            "txt" | "text" => ("text", "txt", content.into_bytes()),
            "md" | "markdown" => ("markdown", "md", content.into_bytes()),
            "csv" => ("csv", "csv", content.into_bytes()),
            "json" => ("json", "json", content.into_bytes()),
            "xml" => ("xml", "xml", content.into_bytes()),
            "docx" | "word" => {
                let manager = crate::application::doc_engine::formats::FormatManager::new();
                let formatter = manager
                    .get_formatter("word")
                    .ok_or_else(|| anyhow!("Word formatter is not registered."))?;
                ("docx", "docx", formatter.format(&content)?)
            }
            "pdf" => {
                if self
                    .setting_or_env("output_pdf_endpoint", "AGENTFORGE_PDF_OUTPUT_URL")
                    .is_some()
                {
                    return self.render_pdf(args).await;
                }
                let manager = crate::application::doc_engine::formats::FormatManager::new();
                let formatter = manager
                    .get_formatter("pdf")
                    .ok_or_else(|| anyhow!("PDF formatter is not registered."))?;
                ("pdf", "pdf", formatter.format(&content)?)
            }
            "xlsx" | "excel" => ("xlsx", "xlsx", build_xlsx(args, &content)?),
            other => {
                return Err(anyhow!(
                    "Unsupported document format '{}'. Supported: html, txt, md, csv, json, xml, docx, pdf, xlsx.",
                    other
                ))
            }
        };

        let path = self.save_artifact(args, "docs", extension, &bytes)?;
        Ok(format!(
            "Document generated ({}): {}",
            normalized_format,
            path.display()
        ))
    }

    fn setting_or_env(&self, setting_key: &str, env_key: &str) -> Option<String> {
        self.db
            .get_setting(setting_key)
            .ok()
            .flatten()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| std::env::var(env_key).ok().filter(|v| !v.trim().is_empty()))
    }

    async fn output_credential(
        &self,
        reference_setting_key: &str,
        legacy_raw_setting_key: &str,
        fallback_env_keys: &[&str],
    ) -> Result<Option<String>> {
        let reference = self
            .db
            .get_setting(reference_setting_key)?
            .filter(|value| !value.trim().is_empty());
        if reference.is_some() {
            return resolve_credential_reference(reference.as_deref(), fallback_env_keys).await;
        }

        if self
            .db
            .get_setting(legacy_raw_setting_key)?
            .filter(|value| !value.trim().is_empty())
            .is_some()
        {
            return Err(anyhow!(
                "Legacy raw credential setting '{}' is blocked. Replace it with '{}' using a secret:// or env: reference.",
                legacy_raw_setting_key,
                reference_setting_key
            ));
        }

        resolve_credential_reference(None, fallback_env_keys).await
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
        let content_hash = format!("{:x}", Sha256::digest(bytes));
        self.db
            .insert_artifact(&crate::core::models::ArtifactRecord {
                id: uuid::Uuid::new_v4().to_string(),
                run_id: self.run_id.clone(),
                session_id: self.session_id.clone(),
                instance_id: self.team_instance_id.clone(),
                agent_id: self.agent_id.clone(),
                invocation_id: self.invocation_id.clone(),
                artifact_kind: format!("{}.{}", category, extension.trim_start_matches('.')),
                path: path.to_string_lossy().to_string(),
                content_hash,
                created_at: chrono::Utc::now().to_rfc3339(),
            })?;
        Ok(path)
    }

    fn resolve_output_path(
        &self,
        requested_path: Option<&str>,
        category: &str,
        extension: &str,
    ) -> Result<PathBuf> {
        let requested = if let Some(path) = requested_path.filter(|v| !v.trim().is_empty()) {
            PathBuf::from(path)
        } else {
            PathBuf::from(
                self.setting_or_env("output_tools_dir", "AGENTFORGE_OUTPUT_TOOLS_DIR")
                    .unwrap_or_else(|| "outputs".to_string()),
            )
            .join(category)
            .join(format!(
                "{}.{}",
                uuid::Uuid::new_v4(),
                extension.trim_start_matches('.')
            ))
        };
        ToolExecutionGateway::new(self.db.clone())
            .resolve_workspace_path(&self.team_instance_id, &requested.to_string_lossy())
            .map_err(anyhow::Error::msg)
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

fn normalize_html_document(content: &str) -> String {
    let trimmed = content.trim_start();
    if trimmed.starts_with("<!doctype")
        || trimmed.starts_with("<!DOCTYPE")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<HTML")
    {
        content.to_string()
    } else {
        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"></head><body>{}</body></html>",
            escape_xml(content).replace('\n', "<br>\n")
        )
    }
}

fn build_xlsx(args: &Value, content: &str) -> Result<Vec<u8>> {
    use std::io::Write;

    let rows = rows_from_args(args).unwrap_or_else(|| parse_csv_like(content));
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#)?;

    zip.add_directory("_rels/", options)?;
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#)?;

    zip.add_directory("xl/_rels/", options)?;
    zip.start_file("xl/_rels/workbook.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#)?;

    zip.start_file("xl/workbook.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#)?;

    zip.add_directory("xl/worksheets/", options)?;
    zip.start_file("xl/worksheets/sheet1.xml", options)?;
    let sheet = worksheet_xml(&rows);
    zip.write_all(sheet.as_bytes())?;

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

fn rows_from_args(args: &Value) -> Option<Vec<Vec<String>>> {
    let rows = args.get("rows")?.as_array()?;
    let mut out = Vec::new();
    for row in rows {
        if let Some(values) = row.as_array() {
            out.push(values.iter().map(value_to_cell).collect());
        } else {
            out.push(vec![value_to_cell(row)]);
        }
    }
    Some(out)
}

fn value_to_cell(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn parse_csv_like(content: &str) -> Vec<Vec<String>> {
    content
        .lines()
        .map(|line| {
            let delimiter = if line.contains('\t') { '\t' } else { ',' };
            line.split(delimiter)
                .map(|cell| cell.trim().trim_matches('"').to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn worksheet_xml(rows: &[Vec<String>]) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    for (row_ix, row) in rows.iter().enumerate() {
        let row_number = row_ix + 1;
        out.push_str(&format!(r#"<row r="{}">"#, row_number));
        for (col_ix, value) in row.iter().enumerate() {
            let cell_ref = format!("{}{}", column_name(col_ix + 1), row_number);
            out.push_str(&format!(
                r#"<c r="{}" t="inlineStr"><is><t>{}</t></is></c>"#,
                cell_ref,
                escape_xml(value)
            ));
        }
        out.push_str("</row>");
    }
    out.push_str("</sheetData></worksheet>");
    out
}

fn column_name(mut index: usize) -> String {
    let mut name = String::new();
    while index > 0 {
        let rem = (index - 1) % 26;
        name.insert(0, (b'A' + rem as u8) as char);
        index = (index - 1) / 26;
    }
    name
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
