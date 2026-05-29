use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::time::Duration;
use zip::ZipArchive;

const DEFAULT_MAX_TEXT_CHARS: usize = 16_000;
const DEFAULT_MAX_TOTAL_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct AnalyzeOptions {
    pub max_text_chars: usize,
    pub max_total_bytes: usize,
    pub include_external_media_analysis: bool,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            max_text_chars: DEFAULT_MAX_TEXT_CHARS,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
            include_external_media_analysis: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub source: String,
    pub file_name: String,
    pub extension: Option<String>,
    pub mime_type: Option<String>,
    pub kind: String,
    pub size_bytes: usize,
    pub text: String,
    pub notes: Vec<String>,
}

impl FileAnalysis {
    pub fn render_markdown(&self, text_limit: usize) -> String {
        let mut out = String::new();
        out.push_str(&format!("### {}\n", self.file_name));
        out.push_str(&format!("- Source: {}\n", self.source));
        out.push_str(&format!("- Kind: {}\n", self.kind));
        out.push_str(&format!("- Size: {} bytes\n", self.size_bytes));
        if let Some(mime) = &self.mime_type {
            out.push_str(&format!("- MIME: {}\n", mime));
        }
        if let Some(ext) = &self.extension {
            out.push_str(&format!("- Extension: .{}\n", ext));
        }
        if !self.notes.is_empty() {
            out.push_str("- Notes:\n");
            for note in &self.notes {
                out.push_str(&format!("  - {}\n", note));
            }
        }
        if self.text.trim().is_empty() {
            out.push_str("\nNo textual content was extracted.\n");
        } else {
            out.push_str("\nExtracted content:\n```text\n");
            out.push_str(&truncate_chars(&self.text, text_limit));
            out.push_str("\n```\n");
        }
        out
    }
}

pub async fn analyze_path(path: impl AsRef<Path>, options: AnalyzeOptions) -> Result<FileAnalysis> {
    let path = path.as_ref().to_path_buf();
    let bytes =
        std::fs::read(&path).with_context(|| format!("Failed to read file {}", path.display()))?;
    let source = path.display().to_string();
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| source.clone());
    let extension = path
        .extension()
        .map(|s| s.to_string_lossy().to_ascii_lowercase());

    let mut analysis = analyze_bytes_inner(
        &source,
        &file_name,
        extension.as_deref(),
        None,
        &bytes,
        options,
    )
    .await?;

    if extension.as_deref() == Some("pdf") {
        match pdf_extract::extract_text(&path) {
            Ok(text) if !text.trim().is_empty() => {
                analysis.text =
                    truncate_chars(&normalize_space_loose(&text), options.max_text_chars)
            }
            Ok(_) => analysis
                .notes
                .push("PDF parser returned no selectable text.".to_string()),
            Err(e) => analysis
                .notes
                .push(format!("PDF text extraction failed: {}", e)),
        }
    }

    Ok(analysis)
}

pub async fn analyze_bytes(
    source_name: &str,
    content_type: Option<&str>,
    bytes: &[u8],
    options: AnalyzeOptions,
) -> Result<FileAnalysis> {
    let file_name = source_name
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(source_name);
    let extension = extension_from_name(file_name)
        .or_else(|| extension_from_content_type(content_type.unwrap_or("")));
    analyze_bytes_inner(
        source_name,
        file_name,
        extension.as_deref(),
        content_type,
        bytes,
        options,
    )
    .await
}

pub async fn analyze_url(url: &str, options: AnalyzeOptions) -> Result<FileAnalysis> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(8))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let response = client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "AgentForge/1.0 (+https://agentforge.local; research fetcher)",
        )
        .send()
        .await
        .with_context(|| format!("Failed to fetch {}", url))?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());
    let final_url = response.url().to_string();
    let bytes = response.bytes().await?;
    if !status.is_success() {
        return Err(anyhow!(
            "HTTP {} while fetching {}",
            status.as_u16(),
            final_url
        ));
    }

    let mut analysis = analyze_bytes(&final_url, content_type.as_deref(), &bytes, options).await?;
    analysis.source = final_url;
    analysis.notes.push(format!("Fetched with HTTP {}", status));
    Ok(analysis)
}

pub async fn build_chat_context(
    user_text: &str,
    attached_files: &[String],
    workspace_dir: Option<&str>,
    options: AnalyzeOptions,
) -> String {
    let mut sections = Vec::new();

    for file in attached_files {
        let trimmed = file.trim();
        if trimmed.is_empty() {
            continue;
        }
        let result = if is_http_url(trimmed) {
            analyze_url(trimmed, options).await
        } else {
            analyze_path(resolve_path(trimmed, workspace_dir), options).await
        };

        match result {
            Ok(analysis) => sections.push(analysis.render_markdown(options.max_text_chars)),
            Err(e) => sections.push(format!(
                "### {}\n- Source: {}\n- Error: {}\n",
                display_name_from_source(trimmed),
                trimmed,
                e
            )),
        }
    }

    for url in extract_urls(user_text) {
        let result = analyze_url(&url, options).await;
        match result {
            Ok(analysis) => sections.push(analysis.render_markdown(options.max_text_chars)),
            Err(e) => sections.push(format!(
                "### {}\n- Source: {}\n- Error: {}\n",
                display_name_from_source(&url),
                url,
                e
            )),
        }
    }

    if sections.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("\n\n--- AUTO-EXTRACTED FILE AND URL CONTEXT ---\n");
    out.push_str("AgentForge extracted this context before sending the prompt to the provider. Use it as grounded source material; mention extraction limits when relevant.\n\n");
    out.push_str(&sections.join("\n"));
    out.push_str("\n--- END AUTO-EXTRACTED CONTEXT ---\n");
    out
}

pub fn extract_urls(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for token in text.split_whitespace() {
        let trimmed = token.trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';'
            )
        });
        let trimmed = trimmed.trim_end_matches(['.', ':']);
        if is_http_url(trimmed) && !urls.iter().any(|u| u == trimmed) {
            urls.push(trimmed.to_string());
        }
    }
    urls
}

pub fn resolve_path(path: &str, workspace_dir: Option<&str>) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else if let Some(workspace) = workspace_dir.filter(|v| !v.trim().is_empty()) {
        PathBuf::from(workspace).join(path)
    } else {
        path
    }
}

async fn analyze_bytes_inner(
    source: &str,
    file_name: &str,
    extension: Option<&str>,
    content_type: Option<&str>,
    bytes: &[u8],
    options: AnalyzeOptions,
) -> Result<FileAnalysis> {
    let limited = if bytes.len() > options.max_total_bytes {
        &bytes[..options.max_total_bytes]
    } else {
        bytes
    };
    let extension = extension.map(|s| s.trim_start_matches('.').to_ascii_lowercase());
    let mime_type = content_type
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .or_else(|| detect_mime(limited, extension.as_deref()));

    let mut notes = Vec::new();
    if bytes.len() > options.max_total_bytes {
        notes.push(format!(
            "Input was capped at {} bytes for extraction.",
            options.max_total_bytes
        ));
    }

    let kind = classify_kind(extension.as_deref(), mime_type.as_deref(), limited);
    let mut text = match kind.as_str() {
        "html" => html_to_text(&String::from_utf8_lossy(limited)),
        "xml" => xml_to_text(&String::from_utf8_lossy(limited)),
        "pdf" => match pdf_extract::extract_text_from_mem(limited) {
            Ok(text) => normalize_space_loose(&text),
            Err(e) => {
                notes.push(format!("PDF text extraction failed: {}", e));
                String::new()
            }
        },
        "docx" => extract_docx(limited, &mut notes).unwrap_or_else(|e| {
            notes.push(format!("DOCX extraction failed: {}", e));
            String::new()
        }),
        "xlsx" => extract_xlsx(limited, &mut notes).unwrap_or_else(|e| {
            notes.push(format!("XLSX extraction failed: {}", e));
            String::new()
        }),
        "pptx" => extract_pptx(limited, &mut notes).unwrap_or_else(|e| {
            notes.push(format!("PPTX extraction failed: {}", e));
            String::new()
        }),
        "odf" => extract_odf(limited, &mut notes).unwrap_or_else(|e| {
            notes.push(format!("OpenDocument extraction failed: {}", e));
            String::new()
        }),
        "zip" => extract_zip_listing(limited, &mut notes).unwrap_or_else(|e| {
            notes.push(format!("ZIP listing failed: {}", e));
            String::new()
        }),
        "image" | "video" | "audio" => media_metadata(file_name, limited, &kind, &mut notes),
        "binary" => {
            if looks_textual(limited) {
                notes.push("Unknown extension, but bytes look textual.".to_string());
                String::from_utf8_lossy(limited).into_owned()
            } else {
                notes.push(
                    "Binary format has no built-in text extractor. Configure AGENTFORGE_FILE_UNDERSTANDING_URL for semantic media analysis."
                        .to_string(),
                );
                String::new()
            }
        }
        _ => String::from_utf8_lossy(limited).into_owned(),
    };

    if matches!(kind.as_str(), "image" | "video" | "audio")
        && options.include_external_media_analysis
    {
        match try_external_file_understanding(source, mime_type.as_deref(), limited).await {
            Ok(Some(semantic)) if !semantic.trim().is_empty() => {
                if !text.trim().is_empty() {
                    text.push_str("\n\n");
                }
                text.push_str("External semantic analysis:\n");
                text.push_str(&semantic);
            }
            Ok(_) => {}
            Err(e) => notes.push(format!("External media analysis failed: {}", e)),
        }
    }

    text = normalize_space_loose(&text);
    text = truncate_chars(&text, options.max_text_chars);

    Ok(FileAnalysis {
        source: source.to_string(),
        file_name: file_name.to_string(),
        extension,
        mime_type,
        kind,
        size_bytes: bytes.len(),
        text,
        notes,
    })
}

fn classify_kind(extension: Option<&str>, mime: Option<&str>, bytes: &[u8]) -> String {
    if let Some(ext) = extension {
        match ext {
            "html" | "htm" => return "html".to_string(),
            "xml" | "svg" | "rss" | "atom" => return "xml".to_string(),
            "pdf" => return "pdf".to_string(),
            "docx" => return "docx".to_string(),
            "xlsx" | "xlsm" => return "xlsx".to_string(),
            "pptx" | "pptm" => return "pptx".to_string(),
            "odt" | "ods" | "odp" => return "odf".to_string(),
            "zip" => return "zip".to_string(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" => {
                return "image".to_string();
            }
            "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" => return "video".to_string(),
            "mp3" | "wav" | "m4a" | "aac" | "ogg" | "flac" => return "audio".to_string(),
            "txt" | "md" | "markdown" | "json" | "jsonl" | "toml" | "yaml" | "yml" | "csv"
            | "tsv" | "log" | "rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "css" | "scss"
            | "java" | "c" | "h" | "cpp" | "hpp" | "cs" | "go" | "php" | "rb" | "sql" | "sh"
            | "ps1" | "bat" => return "text".to_string(),
            "doc" | "xls" | "ppt" => return "binary".to_string(),
            _ => {}
        }
    }

    if let Some(mime) = mime {
        if mime.contains("html") {
            return "html".to_string();
        }
        if mime.contains("xml") || mime.contains("svg") {
            return "xml".to_string();
        }
        if mime.contains("pdf") {
            return "pdf".to_string();
        }
        if mime.starts_with("image/") {
            return "image".to_string();
        }
        if mime.starts_with("video/") {
            return "video".to_string();
        }
        if mime.starts_with("audio/") {
            return "audio".to_string();
        }
        if mime.starts_with("text/") || mime.contains("json") || mime.contains("csv") {
            return "text".to_string();
        }
    }

    if bytes.starts_with(b"%PDF-") {
        return "pdf".to_string();
    }
    if bytes.starts_with(b"PK\x03\x04") {
        return "zip".to_string();
    }
    if is_image_signature(bytes) {
        return "image".to_string();
    }
    if looks_textual(bytes) {
        return "text".to_string();
    }
    "binary".to_string()
}

fn detect_mime(bytes: &[u8], extension: Option<&str>) -> Option<String> {
    if bytes.starts_with(b"%PDF-") {
        Some("application/pdf".to_string())
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png".to_string())
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg".to_string())
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif".to_string())
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp".to_string())
    } else if bytes.starts_with(b"PK\x03\x04") {
        match extension {
            Some("docx") => Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                    .to_string(),
            ),
            Some("xlsx") | Some("xlsm") => Some(
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
            ),
            Some("pptx") | Some("pptm") => Some(
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                    .to_string(),
            ),
            _ => Some("application/zip".to_string()),
        }
    } else {
        extension.and_then(|ext| match ext {
            "html" | "htm" => Some("text/html".to_string()),
            "xml" | "svg" => Some("application/xml".to_string()),
            "txt" | "md" | "csv" | "tsv" | "log" => Some("text/plain".to_string()),
            _ => None,
        })
    }
}

fn is_image_signature(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(&[0xff, 0xd8, 0xff])
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
}

fn extension_from_name(name: &str) -> Option<String> {
    let clean = name
        .split(['?', '#'])
        .next()
        .unwrap_or(name)
        .trim_end_matches('/');
    Path::new(clean)
        .extension()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
}

fn extension_from_content_type(content_type: &str) -> Option<String> {
    let content_type = content_type.split(';').next().unwrap_or("").trim();
    match content_type {
        "text/html" => Some("html".to_string()),
        "application/pdf" => Some("pdf".to_string()),
        "application/json" => Some("json".to_string()),
        "text/plain" => Some("txt".to_string()),
        "text/csv" => Some("csv".to_string()),
        "image/png" => Some("png".to_string()),
        "image/jpeg" => Some("jpg".to_string()),
        "image/gif" => Some("gif".to_string()),
        "image/webp" => Some("webp".to_string()),
        _ => None,
    }
}

fn html_to_text(source: &str) -> String {
    let without_script = remove_tag_blocks(source, "script");
    let without_style = remove_tag_blocks(&without_script, "style");
    let spaced = without_style
        .replace("<br", "\n<br")
        .replace("</p>", "\n")
        .replace("</div>", "\n")
        .replace("</li>", "\n")
        .replace("</tr>", "\n")
        .replace("</h1>", "\n")
        .replace("</h2>", "\n")
        .replace("</h3>", "\n");
    normalize_space_loose(&decode_entities(&strip_tags(&spaced)))
}

fn xml_to_text(source: &str) -> String {
    let source = source
        .replace("</w:p>", "\n")
        .replace("</a:p>", "\n")
        .replace("</w:tc>", "\t")
        .replace("</c>", "\t")
        .replace("</row>", "\n")
        .replace("</text:p>", "\n")
        .replace("</text:h>", "\n");
    normalize_space_loose(&decode_entities(&strip_tags(&source)))
}

fn strip_tags(input: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn remove_tag_blocks(source: &str, tag: &str) -> String {
    let lower = source.to_ascii_lowercase();
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut out = String::new();
    let mut pos = 0usize;

    while let Some(start_rel) = lower[pos..].find(&open) {
        let start = pos + start_rel;
        out.push_str(&source[pos..start]);
        if let Some(end_rel) = lower[start..].find(&close) {
            pos = start + end_rel + close.len();
        } else {
            pos = source.len();
            break;
        }
    }
    out.push_str(&source[pos..]);
    out
}

fn extract_docx(bytes: &[u8], notes: &mut Vec<String>) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let mut parts = Vec::new();
    for name in [
        "word/document.xml",
        "word/footnotes.xml",
        "word/endnotes.xml",
        "word/comments.xml",
    ] {
        if let Some(xml) = read_zip_entry(&mut archive, name)? {
            parts.push(xml_to_text(&xml));
        }
    }

    let names = zip_names(&mut archive)?;
    for name in names {
        if (name.starts_with("word/header") || name.starts_with("word/footer"))
            && name.ends_with(".xml")
        {
            if let Some(xml) = read_zip_entry(&mut archive, &name)? {
                parts.push(xml_to_text(&xml));
            }
        }
    }

    if parts.is_empty() {
        notes.push("DOCX archive did not contain readable Word XML parts.".to_string());
    }
    Ok(parts.join("\n\n"))
}

fn extract_pptx(bytes: &[u8], notes: &mut Vec<String>) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let mut names = zip_names(&mut archive)?;
    names.sort();
    let mut parts = Vec::new();
    for name in names {
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            if let Some(xml) = read_zip_entry(&mut archive, &name)? {
                parts.push(format!("Slide {}:\n{}", parts.len() + 1, xml_to_text(&xml)));
            }
        }
    }
    if parts.is_empty() {
        notes.push("PPTX archive did not contain readable slide XML parts.".to_string());
    }
    Ok(parts.join("\n\n"))
}

fn extract_odf(bytes: &[u8], notes: &mut Vec<String>) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    if let Some(xml) = read_zip_entry(&mut archive, "content.xml")? {
        return Ok(xml_to_text(&xml));
    }
    notes.push("OpenDocument archive did not contain content.xml.".to_string());
    Ok(String::new())
}

fn extract_xlsx(bytes: &[u8], notes: &mut Vec<String>) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let shared = read_zip_entry(&mut archive, "xl/sharedStrings.xml")?
        .map(|xml| extract_tag_texts(&xml, "t"))
        .unwrap_or_default();

    let mut names = zip_names(&mut archive)?;
    names.sort();
    let mut parts = Vec::new();
    for name in names {
        if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
            if let Some(xml) = read_zip_entry(&mut archive, &name)? {
                let sheet = extract_xlsx_sheet(&xml, &shared);
                if !sheet.trim().is_empty() {
                    parts.push(format!("{}:\n{}", name, sheet));
                }
            }
        }
    }

    if parts.is_empty() {
        notes.push("XLSX archive did not contain readable worksheet XML parts.".to_string());
    }
    Ok(parts.join("\n\n"))
}

fn extract_xlsx_sheet(xml: &str, shared_strings: &[String]) -> String {
    let mut out = String::new();
    for row in xml.split("<row").skip(1) {
        let row_body = row.split("</row>").next().unwrap_or(row);
        let mut values = Vec::new();
        for cell in row_body.split("<c").skip(1) {
            let cell_body = cell.split("</c>").next().unwrap_or(cell);
            let is_shared = cell_body.contains(" t=\"s\"") || cell_body.contains(" t='s'");
            let value = if let Some(v) = extract_first_tag_text(cell_body, "v") {
                if is_shared {
                    v.parse::<usize>()
                        .ok()
                        .and_then(|ix| shared_strings.get(ix).cloned())
                        .unwrap_or(v)
                } else {
                    decode_entities(&v)
                }
            } else if let Some(t) = extract_first_tag_text(cell_body, "t") {
                decode_entities(&t)
            } else {
                String::new()
            };
            values.push(value);
        }
        if values.iter().any(|v| !v.trim().is_empty()) {
            out.push_str(&values.join("\t"));
            out.push('\n');
        }
    }
    out
}

fn extract_zip_listing(bytes: &[u8], notes: &mut Vec<String>) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let names = zip_names(&mut archive)?;
    notes.push(
        "Archive listing only; nested file extraction is not expanded automatically.".to_string(),
    );
    Ok(names.join("\n"))
}

fn read_zip_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Option<String>> {
    match archive.by_name(name) {
        Ok(mut file) => {
            let mut text = String::new();
            file.read_to_string(&mut text)?;
            Ok(Some(text))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn zip_names<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        names.push(file.name().to_string());
    }
    Ok(names)
}

fn extract_tag_texts(xml: &str, tag: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut pos = 0usize;
    while let Some(start_rel) = xml[pos..].find(&format!("<{}", tag)) {
        let start = pos + start_rel;
        let Some(gt_rel) = xml[start..].find('>') else {
            break;
        };
        let value_start = start + gt_rel + 1;
        let close = format!("</{}>", tag);
        let Some(end_rel) = xml[value_start..].find(&close) else {
            break;
        };
        let value_end = value_start + end_rel;
        values.push(decode_entities(&xml[value_start..value_end]));
        pos = value_end + close.len();
    }
    values
}

fn extract_first_tag_text(xml: &str, tag: &str) -> Option<String> {
    extract_tag_texts(xml, tag).into_iter().next()
}

fn media_metadata(file_name: &str, bytes: &[u8], kind: &str, notes: &mut Vec<String>) -> String {
    let mut out = format!("{} file: {}\nSize: {} bytes", kind, file_name, bytes.len());
    if kind == "image" {
        if let Some((width, height)) = image_dimensions(bytes) {
            out.push_str(&format!("\nDimensions: {}x{}", width, height));
        } else {
            notes.push("Image dimensions could not be detected from header.".to_string());
        }
        notes
            .push("For visual semantics, configure AGENTFORGE_FILE_UNDERSTANDING_URL.".to_string());
    } else {
        notes.push(format!(
            "Built-in {} support extracts metadata only. Configure AGENTFORGE_FILE_UNDERSTANDING_URL for semantic analysis.",
            kind
        ));
    }
    out
}

fn image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() >= 24 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
        let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
        return Some((width, height));
    }
    if bytes.len() >= 10 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        let width = u16::from_le_bytes(bytes[6..8].try_into().ok()?) as u32;
        let height = u16::from_le_bytes(bytes[8..10].try_into().ok()?) as u32;
        return Some((width, height));
    }
    jpeg_dimensions(bytes)
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut pos = 2usize;
    while pos + 9 < bytes.len() {
        if bytes[pos] != 0xff {
            pos += 1;
            continue;
        }
        let marker = bytes[pos + 1];
        pos += 2;
        if marker == 0xd8 || marker == 0xd9 {
            continue;
        }
        if pos + 2 > bytes.len() {
            return None;
        }
        let len = u16::from_be_bytes(bytes[pos..pos + 2].try_into().ok()?) as usize;
        if len < 2 || pos + len > bytes.len() {
            return None;
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            let height = u16::from_be_bytes(bytes[pos + 3..pos + 5].try_into().ok()?) as u32;
            let width = u16::from_be_bytes(bytes[pos + 5..pos + 7].try_into().ok()?) as u32;
            return Some((width, height));
        }
        pos += len;
    }
    None
}

async fn try_external_file_understanding(
    source: &str,
    mime_type: Option<&str>,
    bytes: &[u8],
) -> Result<Option<String>> {
    let endpoint = match std::env::var("AGENTFORGE_FILE_UNDERSTANDING_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        Some(endpoint) => endpoint,
        None => return Ok(None),
    };

    let mut payload = serde_json::Map::new();
    payload.insert("source".to_string(), Value::String(source.to_string()));
    if let Some(mime) = mime_type {
        payload.insert("mime_type".to_string(), Value::String(mime.to_string()));
    }
    payload.insert(
        "data_base64".to_string(),
        Value::String(STANDARD.encode(bytes)),
    );
    payload.insert(
        "prompt".to_string(),
        Value::String(
            "Extract semantic content, visible text, objects, timeline, and useful facts for an AI agent."
                .to_string(),
        ),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let mut request = client.post(endpoint).json(&Value::Object(payload));
    if let Ok(api_key) = std::env::var("AGENTFORGE_FILE_UNDERSTANDING_API_KEY") {
        if !api_key.trim().is_empty() {
            request = request.bearer_auth(api_key);
        }
    }
    let response = request.send().await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!("service returned HTTP {}: {}", status, body));
    }
    if let Ok(json) = serde_json::from_str::<Value>(&body) {
        return Ok(find_string_key(
            &json,
            &["analysis", "text", "content", "description", "summary"],
        ));
    }
    Ok(Some(body))
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

fn looks_textual(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return true;
    }
    if bytes.iter().any(|b| *b == 0) {
        return false;
    }
    let sample = &bytes[..bytes.len().min(4096)];
    let printable = sample
        .iter()
        .filter(|b| b.is_ascii_graphic() || b.is_ascii_whitespace())
        .count();
    printable as f32 / sample.len() as f32 > 0.85
}

fn decode_entities(input: &str) -> String {
    let mut out = input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    while let Some(start) = out.find("&#x") {
        let Some(end_rel) = out[start..].find(';') else {
            break;
        };
        let end = start + end_rel;
        let code = &out[start + 3..end];
        if let Ok(value) = u32::from_str_radix(code, 16) {
            if let Some(ch) = char::from_u32(value) {
                out.replace_range(start..=end, &ch.to_string());
                continue;
            }
        }
        break;
    }
    while let Some(start) = out.find("&#") {
        let Some(end_rel) = out[start..].find(';') else {
            break;
        };
        let end = start + end_rel;
        let code = &out[start + 2..end];
        if let Ok(value) = code.parse::<u32>() {
            if let Some(ch) = char::from_u32(value) {
                out.replace_range(start..=end, &ch.to_string());
                continue;
            }
        }
        break;
    }
    out
}

fn normalize_space_loose(input: &str) -> String {
    let mut out = String::new();
    let mut last_was_space = false;
    let mut blank_lines = 0usize;
    for line in input.lines() {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.trim().is_empty() {
            blank_lines += 1;
            if blank_lines <= 1 && !out.ends_with('\n') {
                out.push('\n');
            }
            last_was_space = false;
            continue;
        }
        blank_lines = 0;
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        for ch in line.chars() {
            if ch.is_whitespace() {
                if !last_was_space {
                    out.push(' ');
                    last_was_space = true;
                }
            } else {
                out.push(ch);
                last_was_space = false;
            }
        }
        out.push('\n');
        last_was_space = false;
    }
    out.trim().to_string()
}

pub fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars).collect();
    out.push_str("\n[truncated]");
    out
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn display_name_from_source(source: &str) -> String {
    source
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(source)
        .to_string()
}
