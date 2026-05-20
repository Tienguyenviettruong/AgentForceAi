use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub struct ExportManager;

impl ExportManager {
    pub fn export_to_json<T: serde::Serialize>(data: &T, path: PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(data)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn export_to_csv(headers: &[&str], rows: &[Vec<String>], path: PathBuf) -> Result<()> {
        let mut file = File::create(path)?;

        let header_line = headers.join(",") + "\n";
        file.write_all(header_line.as_bytes())?;

        for row in rows {
            let row_line = row.join(",") + "\n";
            file.write_all(row_line.as_bytes())?;
        }

        Ok(())
    }
}
