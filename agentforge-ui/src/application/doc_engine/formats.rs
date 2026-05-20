use anyhow::Result;
use std::collections::HashMap;

pub trait Formatter: Send + Sync {
    fn name(&self) -> &str;
    fn format(&self, content: &str) -> Result<Vec<u8>>;
}

pub struct MarkdownFormatter;

impl Formatter for MarkdownFormatter {
    fn name(&self) -> &str {
        "markdown"
    }

    fn format(&self, content: &str) -> Result<Vec<u8>> {
        // Markdown is just the raw content from the template
        Ok(content.as_bytes().to_vec())
    }
}

pub struct WordFormatter;

impl Formatter for WordFormatter {
    fn name(&self) -> &str {
        "word"
    }

    fn format(&self, content: &str) -> Result<Vec<u8>> {
        use docx_rs::*;
        use std::io::Cursor;

        let mut doc = Docx::new();
        for line in content.lines() {
            doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(line)));
        }

        let mut buf = Cursor::new(Vec::new());
        doc.build().pack(&mut buf)?;
        Ok(buf.into_inner())
    }
}

pub struct PdfFormatter;

impl Formatter for PdfFormatter {
    fn name(&self) -> &str {
        "pdf"
    }

    fn format(&self, content: &str) -> Result<Vec<u8>> {
        use printpdf::*;

        let (doc, page1, layer1) = PdfDocument::new("Document", Mm(210.0), Mm(297.0), "Layer 1");
        let current_layer = doc.get_page(page1).get_layer(layer1);

        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| anyhow::anyhow!("Font error: {:?}", e))?;

        current_layer.use_text(content, 12.0, Mm(10.0), Mm(280.0), &font);

        let mut buf = Vec::new();
        {
            let mut writer = std::io::BufWriter::new(&mut buf);
            doc.save(&mut writer)
                .map_err(|e| anyhow::anyhow!("PDF save error: {:?}", e))?;
        }

        Ok(buf)
    }
}

pub struct FormatManager {
    formatters: HashMap<String, Box<dyn Formatter>>,
}

impl Default for FormatManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatManager {
    pub fn new() -> Self {
        let mut manager = Self {
            formatters: HashMap::new(),
        };

        manager.register_formatter(Box::new(MarkdownFormatter));
        manager.register_formatter(Box::new(WordFormatter));
        manager.register_formatter(Box::new(PdfFormatter));

        manager
    }

    pub fn register_formatter(&mut self, formatter: Box<dyn Formatter>) {
        self.formatters
            .insert(formatter.name().to_string(), formatter);
    }

    pub fn get_formatter(&self, name: &str) -> Option<&dyn Formatter> {
        self.formatters.get(name).map(|f| f.as_ref())
    }

    pub fn list_formats(&self) -> Vec<String> {
        self.formatters.keys().cloned().collect()
    }
}
