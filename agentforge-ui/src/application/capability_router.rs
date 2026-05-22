use crate::application::file_intelligence::{analyze_path, AnalyzeOptions};
use crate::core::models::{ContentPart, Modality, ModelCapability};
use std::path::Path;

/// Kết quả routing cho một file attachment.
#[derive(Debug)]
pub enum AttachmentHandling {
    /// Pass bytes trực tiếp tới model (model hỗ trợ modality natively).
    PassNative(Vec<ContentPart>),
    /// Extract text/metadata local, gửi dưới dạng text context.
    LocalExtract { text: String, notes: Vec<String> },
    /// Model không hỗ trợ và không có local extractor – báo rõ cho user.
    CapabilityMissing { message: String },
}

/// Loại file cho routing decision.
#[derive(Debug, PartialEq)]
enum FileKind {
    Text,    // txt, md, json, csv, code files
    Html,
    Xml,
    Pdf,
    Docx,
    Xlsx,
    Pptx,
    Odf,
    Zip,
    Image,
    Audio,
    Video,
    Binary,
}

fn classify_path(path: &Path) -> FileKind {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    match ext.as_deref() {
        Some("html") | Some("htm") => FileKind::Html,
        Some("xml") | Some("svg") | Some("rss") | Some("atom") => FileKind::Xml,
        Some("pdf") => FileKind::Pdf,
        Some("docx") => FileKind::Docx,
        Some("xlsx") | Some("xlsm") => FileKind::Xlsx,
        Some("pptx") | Some("pptm") => FileKind::Pptx,
        Some("odt") | Some("ods") | Some("odp") => FileKind::Odf,
        Some("zip") => FileKind::Zip,
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp")
        | Some("bmp") | Some("tif") | Some("tiff") => FileKind::Image,
        Some("mp4") | Some("mov") | Some("mkv") | Some("avi") | Some("webm") | Some("m4v") => {
            FileKind::Video
        }
        Some("mp3") | Some("wav") | Some("m4a") | Some("aac") | Some("ogg") | Some("flac") => {
            FileKind::Audio
        }
        Some("txt") | Some("md") | Some("markdown") | Some("json") | Some("jsonl")
        | Some("toml") | Some("yaml") | Some("yml") | Some("csv") | Some("tsv") | Some("log")
        | Some("rs") | Some("py") | Some("js") | Some("jsx") | Some("ts") | Some("tsx")
        | Some("css") | Some("scss") | Some("java") | Some("c") | Some("h") | Some("cpp")
        | Some("hpp") | Some("cs") | Some("go") | Some("php") | Some("rb") | Some("sql")
        | Some("sh") | Some("ps1") | Some("bat") => FileKind::Text,
        _ => FileKind::Binary,
    }
}

fn image_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("tif") | Some("tiff") => "image/tiff",
        _ => "image/jpeg",
    }
}

fn audio_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("ogg") => "audio/ogg",
        Some("flac") => "audio/flac",
        _ => "audio/mpeg",
    }
}

fn video_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("mkv") => "video/x-matroska",
        Some("avi") => "video/x-msvideo",
        Some("webm") => "video/webm",
        _ => "video/mp4",
    }
}

/// Capability-aware router cho file attachments.
///
/// Quyết định cách xử lý từng file dựa vào khả năng của model hiện tại:
/// - Text/document: luôn extract local (không cần vision)
/// - Image/audio/video: nếu model hỗ trợ → pass native; nếu không → báo rõ
/// - PDF: nếu model hỗ trợ PDF native → pass; nếu không → extract text local
pub struct CapabilityRouter;

impl CapabilityRouter {
    /// Xử lý một file attachment và trả về cách gửi tới model.
    ///
    /// # Arguments
    /// * `path` - Đường dẫn tới file
    /// * `capability` - Khả năng của model hiện tại  
    /// * `options` - Tuỳ chọn extraction (text limit, ...)
    pub async fn route_attachment(
        path: &Path,
        capability: &ModelCapability,
        options: AnalyzeOptions,
    ) -> AttachmentHandling {
        let kind = classify_path(path);
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        match kind {
            // Text-based formats: luôn extract local, không cần model capability đặc biệt
            FileKind::Text
            | FileKind::Html
            | FileKind::Xml
            | FileKind::Docx
            | FileKind::Xlsx
            | FileKind::Pptx
            | FileKind::Odf
            | FileKind::Zip => match analyze_path(path, options).await {
                Ok(analysis) => AttachmentHandling::LocalExtract {
                    text: analysis.render_markdown(options.max_text_chars),
                    notes: analysis.notes,
                },
                Err(e) => AttachmentHandling::LocalExtract {
                    text: format!("### {}\n- Error: {}\n", file_name, e),
                    notes: vec![],
                },
            },

            // PDF: nếu model hỗ trợ PDF native → pass; nếu không → extract text
            FileKind::Pdf => {
                if capability.supports(&Modality::Pdf) {
                    match std::fs::read(path) {
                        Ok(data) => AttachmentHandling::PassNative(vec![ContentPart::PdfBytes {
                            data,
                        }]),
                        Err(e) => AttachmentHandling::LocalExtract {
                            text: format!("### {}\n- Read error: {}\n", file_name, e),
                            notes: vec![],
                        },
                    }
                } else {
                    // Fallback: extract text local
                    match analyze_path(path, options).await {
                        Ok(analysis) if !analysis.text.trim().is_empty() => {
                            let text = analysis.render_markdown(options.max_text_chars);
                            let mut notes = analysis.notes;
                            notes.push(
                                "PDF rendered as text (model does not support native PDF input)."
                                    .to_string(),
                            );
                            AttachmentHandling::LocalExtract { text, notes }
                        }
                        Ok(analysis) => {
                            let mut notes = analysis.notes;
                            notes.push(
                                "PDF has no selectable text and the current model does not support PDF natively. To process this file, switch to a model with PDF capability (e.g. Gemini 1.5+, Claude 3+) or use a PDF OCR tool.".to_string()
                            );
                            AttachmentHandling::LocalExtract {
                                text: format!("### {}\n- PDF with no extractable text.\n", file_name),
                                notes,
                            }
                        }
                        Err(e) => AttachmentHandling::LocalExtract {
                            text: format!("### {}\n- Error: {}\n", file_name, e),
                            notes: vec![],
                        },
                    }
                }
            }

            // Image: nếu model hỗ trợ vision → pass native; nếu không → metadata only
            FileKind::Image => {
                if capability.supports(&Modality::Image) {
                    match std::fs::read(path) {
                        Ok(data) => {
                            let mime = image_mime(path).to_string();
                            AttachmentHandling::PassNative(vec![ContentPart::ImageBytes {
                                mime_type: mime,
                                data,
                            }])
                        }
                        Err(e) => AttachmentHandling::LocalExtract {
                            text: format!("### {}\n- Read error: {}\n", file_name, e),
                            notes: vec![],
                        },
                    }
                } else {
                    // Fallback: extract image metadata (dimensions, size) only
                    match analyze_path(path, options).await {
                        Ok(analysis) => {
                            let text = analysis.render_markdown(options.max_text_chars);
                            let mut notes = analysis.notes;
                            notes.push(
                                format!(
                                    "Image '{}' cannot be visually analyzed: current model does not support image input. \
                                    Switch to a vision-capable model (e.g. Gemini, Claude 3, GPT-4V) to see image content.",
                                    file_name
                                )
                            );
                            AttachmentHandling::LocalExtract { text, notes }
                        }
                        Err(e) => AttachmentHandling::LocalExtract {
                            text: format!("### {}\n- Error: {}\n", file_name, e),
                            notes: vec![],
                        },
                    }
                }
            }

            // Audio: nếu model hỗ trợ audio → pass native; nếu không → capability missing
            FileKind::Audio => {
                if capability.supports(&Modality::Audio) {
                    match std::fs::read(path) {
                        Ok(data) => {
                            let mime = audio_mime(path).to_string();
                            AttachmentHandling::PassNative(vec![ContentPart::AudioBytes {
                                mime_type: mime,
                                data,
                            }])
                        }
                        Err(e) => AttachmentHandling::LocalExtract {
                            text: format!("### {}\n- Read error: {}\n", file_name, e),
                            notes: vec![],
                        },
                    }
                } else {
                    AttachmentHandling::CapabilityMissing {
                        message: format!(
                            "Audio file '{}' cannot be processed: current model does not support audio input. \
                            Switch to an audio-capable model or transcribe the audio first.",
                            file_name
                        ),
                    }
                }
            }

            // Video: nếu model hỗ trợ video → pass native; nếu không → capability missing
            FileKind::Video => {
                if capability.supports(&Modality::Video) {
                    match std::fs::read(path) {
                        Ok(data) => {
                            let mime = video_mime(path).to_string();
                            AttachmentHandling::PassNative(vec![ContentPart::VideoBytes {
                                mime_type: mime,
                                data,
                            }])
                        }
                        Err(e) => AttachmentHandling::LocalExtract {
                            text: format!("### {}\n- Read error: {}\n", file_name, e),
                            notes: vec![],
                        },
                    }
                } else {
                    AttachmentHandling::CapabilityMissing {
                        message: format!(
                            "Video file '{}' cannot be processed: current model does not support video input. \
                            Switch to a video-capable model (e.g. Gemini 1.5 Pro+) or extract frames/transcript first.",
                            file_name
                        ),
                    }
                }
            }

            // Binary unknown: thử extract text nếu looks textual
            FileKind::Binary => match analyze_path(path, options).await {
                Ok(analysis) if !analysis.text.trim().is_empty() => {
                    AttachmentHandling::LocalExtract {
                        text: analysis.render_markdown(options.max_text_chars),
                        notes: analysis.notes,
                    }
                }
                Ok(_) => AttachmentHandling::CapabilityMissing {
                    message: format!(
                        "Binary file '{}' has no text content and no local extractor is available.",
                        file_name
                    ),
                },
                Err(e) => AttachmentHandling::LocalExtract {
                    text: format!("### {}\n- Error: {}\n", file_name, e),
                    notes: vec![],
                },
            },
        }
    }

    /// Xây dựng danh sách ContentParts từ user text + file attachments,
    /// dựa vào capability của model hiện tại.
    ///
    /// Returns: (parts_for_message, warning_messages_for_user)
    pub async fn build_message_parts(
        user_text: &str,
        attachments: &[String],
        workspace_dir: Option<&str>,
        capability: &ModelCapability,
        options: AnalyzeOptions,
    ) -> (Vec<ContentPart>, Vec<String>) {
        let mut parts: Vec<ContentPart> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let mut text_context = String::new();

        for att in attachments {
            let trimmed = att.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Resolve path
            let path = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                // URLs: luôn extract local (không pass URL native qua multimodal)
                let url_options = AnalyzeOptions {
                    include_external_media_analysis: false,
                    ..options
                };
                match crate::application::file_intelligence::analyze_url(trimmed, url_options).await
                {
                    Ok(analysis) => {
                        text_context.push_str(&analysis.render_markdown(options.max_text_chars));
                        text_context.push('\n');
                    }
                    Err(e) => {
                        warnings.push(format!("URL '{}' could not be fetched: {}", trimmed, e));
                    }
                }
                continue;
            } else {
                crate::application::file_intelligence::resolve_path(trimmed, workspace_dir)
            };

            match Self::route_attachment(&path, capability, options).await {
                AttachmentHandling::PassNative(mut native_parts) => {
                    parts.append(&mut native_parts);
                }
                AttachmentHandling::LocalExtract { text, notes } => {
                    text_context.push_str(&text);
                    text_context.push('\n');
                    warnings.extend(notes);
                }
                AttachmentHandling::CapabilityMissing { message } => {
                    warnings.push(message);
                }
            }
        }

        // Build final parts: text_context đến trước (nếu có), rồi user_text, rồi media
        let mut final_parts: Vec<ContentPart> = Vec::new();

        if !text_context.is_empty() {
            let context_header = format!(
                "\n--- AUTO-EXTRACTED FILE CONTEXT ---\n\
                AgentForge extracted this context before sending to the provider. \
                Use it as grounded source material.\n\n{}\n--- END CONTEXT ---\n\n",
                text_context.trim()
            );
            final_parts.push(ContentPart::Text(context_header));
        }

        final_parts.push(ContentPart::Text(user_text.to_string()));
        final_parts.extend(parts); // media parts sau text

        (final_parts, warnings)
    }
}
