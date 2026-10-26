use serde::{Deserialize, Serialize};

/// A spreadsheet or CSV file read for import: every cell as text.
#[derive(Debug, Clone, Serialize)]
pub struct ImportFile {
    pub file_name: String,
    pub sheets: Vec<ImportSheet>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportSheet {
    pub name: String,
    /// Rows of cells, the header row (if any) included. Rows may differ in length.
    pub rows: Vec<Vec<String>>,
    /// True when the sheet had more rows than were read.
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Csv,
    Xlsx,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Xlsx => "xlsx",
        }
    }
}

/// What an export wrote.
#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    /// The file's name (not its folder), for the confirmation.
    pub file_name: String,
    pub rows: usize,
    /// True when the source had more rows than could be exported.
    pub truncated: bool,
}
