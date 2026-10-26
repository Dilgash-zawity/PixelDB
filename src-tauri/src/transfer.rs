//! Reading CSV and spreadsheet files for import, and writing rows out as CSV
//! or Excel. Free of Tauri and SQLite types so it can be tested directly;
//! the commands pick the files with native dialogs.

use std::io::Cursor;

use calamine::{Data, Reader};
use rust_xlsxwriter::{Format, Workbook};
use serde_json::Value as JsonValue;

use crate::errors::{AppError, AppResult};
use crate::models::{ExportFormat, FieldType, ImportFile, ImportSheet};

pub const MAX_IMPORT_BYTES: usize = 100 * 1024 * 1024;
pub const MAX_IMPORT_ROWS: usize = 200_000;
pub const MAX_IMPORT_COLUMNS: usize = 500;
/// Rows written by one export.
pub const MAX_EXPORT_ROWS: usize = 1_000_000;

/// Reads `bytes` as the file `file_name`, by its extension.
pub fn read_file(file_name: &str, bytes: &[u8]) -> AppResult<ImportFile> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(AppError::Validation(format!(
            "the file is larger than {} MB",
            MAX_IMPORT_BYTES / 1024 / 1024
        )));
    }
    let ext = file_name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    let stem = file_name.rsplit_once('.').map_or(file_name, |(s, _)| s);
    let sheets = match ext.as_str() {
        "xlsx" | "xlsm" | "xlsb" | "xls" | "ods" => read_workbook(bytes)?,
        "csv" | "tsv" | "txt" | "" => vec![read_csv(stem, bytes, (ext == "tsv").then_some(b'\t'))?],
        other => return Err(AppError::Unsupported(format!(".{other} files can't be imported; use CSV or Excel"))),
    };
    if sheets.iter().all(|s| s.rows.is_empty()) {
        return Err(AppError::Validation("the file has no rows".into()));
    }
    Ok(ImportFile { file_name: file_name.to_string(), sheets })
}

/// Text in UTF-8 (a BOM is dropped), else Windows-1252-like Latin-1.
fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

/// The delimiter used most on the first line, outside quotes: comma, semicolon or tab.
fn sniff_delimiter(text: &str) -> u8 {
    let line = text.lines().next().unwrap_or("");
    let mut counts = [(b',', 0usize), (b';', 0), (b'\t', 0)];
    let mut quoted = false;
    for c in line.bytes() {
        if c == b'"' {
            quoted = !quoted;
        } else if !quoted
            && let Some(n) = counts.iter_mut().find(|(d, _)| *d == c)
        {
            n.1 += 1;
        }
    }
    counts.iter().max_by_key(|(_, n)| *n).filter(|(_, n)| *n > 0).map_or(b',', |(d, _)| *d)
}

fn read_csv(name: &str, bytes: &[u8], delimiter: Option<u8>) -> AppResult<ImportSheet> {
    let text = decode(bytes);
    let delimiter = delimiter.unwrap_or_else(|| sniff_delimiter(&text));
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    let mut truncated = false;
    for record in reader.records() {
        let record = record.map_err(|e| AppError::Validation(format!("the file isn't valid CSV: {e}")))?;
        if rows.len() == MAX_IMPORT_ROWS {
            truncated = true;
            break;
        }
        rows.push(record.iter().take(MAX_IMPORT_COLUMNS).map(String::from).collect());
    }
    Ok(ImportSheet { name: name.to_string(), rows: trim_rows(rows), truncated })
}

fn read_workbook(bytes: &[u8]) -> AppResult<Vec<ImportSheet>> {
    let mut book = calamine::open_workbook_auto_from_rs(Cursor::new(bytes.to_vec()))
        .map_err(|e| AppError::Validation(format!("the spreadsheet can't be read: {e}")))?;
    let mut sheets = Vec::new();
    for name in book.sheet_names() {
        let range = book
            .worksheet_range(&name)
            .map_err(|e| AppError::Validation(format!("sheet {name:?} can't be read: {e}")))?;
        let mut rows = Vec::new();
        let mut truncated = false;
        for row in range.rows() {
            if rows.len() == MAX_IMPORT_ROWS {
                truncated = true;
                break;
            }
            rows.push(row.iter().take(MAX_IMPORT_COLUMNS).map(cell_text).collect());
        }
        sheets.push(ImportSheet { name, rows: trim_rows(rows), truncated });
    }
    Ok(sheets)
}

/// Drops empty rows at the end and empty cells at the end of each row.
fn trim_rows(mut rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    for r in &mut rows {
        while r.last().is_some_and(|c| c.trim().is_empty()) {
            r.pop();
        }
    }
    while rows.last().is_some_and(Vec::is_empty) {
        rows.pop();
    }
    rows
}

fn cell_text(d: &Data) -> String {
    match d {
        Data::Empty | Data::Error(_) => String::new(),
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Bool(b) => if *b { "Yes" } else { "No" }.into(),
        Data::DateTime(dt) => excel_serial_to_iso(dt.as_f64()),
    }
}

/// An Excel date serial (days since 1899-12-30) as ISO 8601: a date, or a
/// date and time when it has a time part.
pub fn excel_serial_to_iso(serial: f64) -> String {
    let days = serial.floor() as i64;
    let mut secs = ((serial - serial.floor()) * 86_400.0).round() as i64;
    let (mut y, mut m, mut d) = civil_from_days(days - 25_569); // 1970-01-01 is serial 25569
    if secs >= 86_400 {
        secs -= 86_400;
        (y, m, d) = civil_from_days(days - 25_568);
    }
    if secs == 0 {
        format!("{y:04}-{m:02}-{d:02}")
    } else {
        format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
    }
}

/// Year, month and day of a count of days since 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + (m <= 2) as i64, m, d)
}

// ----- export -----

/// One value as it should appear in an exported file.
enum Cell {
    Empty,
    Text(String),
    Number(f64),
    /// A currency amount: its text and its value.
    Money(String, f64),
    Bool(bool),
}

fn export_cell(v: &JsonValue, t: Option<&FieldType>) -> Cell {
    match (v, t) {
        (JsonValue::Null, _) => Cell::Empty,
        (JsonValue::Number(n), Some(FieldType::Currency { scale })) if n.is_i64() => {
            let minor = n.as_i64().unwrap_or_default();
            let div = 10i64.pow(*scale as u32);
            let sign = if minor < 0 { "-" } else { "" };
            let abs = minor.unsigned_abs();
            let text = if *scale == 0 {
                format!("{sign}{abs}")
            } else {
                format!("{sign}{}.{:0w$}", abs / div as u64, abs % div as u64, w = *scale as usize)
            };
            Cell::Money(text, minor as f64 / div as f64)
        }
        (JsonValue::Number(n), Some(FieldType::YesNo)) => Cell::Bool(n.as_i64() == Some(1)),
        (JsonValue::Number(n), _) => n.as_f64().map_or_else(|| Cell::Text(n.to_string()), Cell::Number),
        (JsonValue::String(s), _) => Cell::Text(s.clone()),
        (JsonValue::Bool(b), _) => Cell::Bool(*b),
        (JsonValue::Object(o), _) if o.contains_key("blob") => {
            let size = o["blob"].get("size").and_then(JsonValue::as_u64).unwrap_or_default();
            Cell::Text(format!("(file, {size} bytes)"))
        }
        (other, _) => Cell::Text(other.to_string()),
    }
}

/// Rows as a file in `format`. `name` titles the sheet of an Excel file.
pub fn write_rows(
    format: ExportFormat,
    name: &str,
    columns: &[String],
    types: &[Option<FieldType>],
    rows: &[Vec<JsonValue>],
) -> AppResult<Vec<u8>> {
    match format {
        ExportFormat::Csv => write_csv(columns, types, rows),
        ExportFormat::Xlsx => write_xlsx(name, columns, types, rows),
    }
}

fn write_csv(columns: &[String], types: &[Option<FieldType>], rows: &[Vec<JsonValue>]) -> AppResult<Vec<u8>> {
    // A BOM so Excel reads the file as UTF-8.
    let mut writer = csv::Writer::from_writer(b"\xEF\xBB\xBF".to_vec());
    let io = |e: csv::Error| AppError::Internal(e.to_string());
    writer.write_record(columns).map_err(io)?;
    for row in rows {
        let cells = row.iter().enumerate().map(|(i, v)| match export_cell(v, types.get(i).and_then(Option::as_ref)) {
            Cell::Empty => String::new(),
            Cell::Text(s) | Cell::Money(s, _) => s,
            Cell::Number(n) => n.to_string(),
            Cell::Bool(b) => if b { "Yes" } else { "No" }.into(),
        });
        writer.write_record(cells).map_err(io)?;
    }
    writer.into_inner().map_err(|e| AppError::Internal(e.to_string()))
}

/// A worksheet name Excel accepts: at most 31 characters, none of `[]:*?/\`.
fn sheet_name(name: &str) -> String {
    let clean: String = name.chars().filter(|c| !"[]:*?/\\".contains(*c)).take(31).collect();
    let clean = clean.trim_matches('\'').trim().to_string();
    if clean.is_empty() { "Sheet1".into() } else { clean }
}

fn write_xlsx(name: &str, columns: &[String], types: &[Option<FieldType>], rows: &[Vec<JsonValue>]) -> AppResult<Vec<u8>> {
    let xl = |e: rust_xlsxwriter::XlsxError| AppError::Internal(e.to_string());
    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    sheet.set_name(sheet_name(name)).map_err(xl)?;
    let bold = Format::new().set_bold();
    let money: Vec<Format> = types
        .iter()
        .map(|t| match t {
            Some(FieldType::Currency { scale }) if *scale > 0 => {
                Format::new().set_num_format(format!("#,##0.{}", "0".repeat(*scale as usize)))
            }
            _ => Format::new().set_num_format("#,##0"),
        })
        .collect();
    for (c, col) in columns.iter().enumerate() {
        sheet.write_string_with_format(0, c as u16, col, &bold).map_err(xl)?;
    }
    for (r, row) in rows.iter().enumerate() {
        let r = r as u32 + 1;
        for (c, v) in row.iter().enumerate() {
            let col = c as u16;
            match export_cell(v, types.get(c).and_then(Option::as_ref)) {
                Cell::Empty => {}
                Cell::Text(s) => {
                    sheet.write_string(r, col, s).map_err(xl)?;
                }
                Cell::Number(n) => {
                    sheet.write_number(r, col, n).map_err(xl)?;
                }
                Cell::Money(_, n) => {
                    sheet.write_number_with_format(r, col, n, &money[c]).map_err(xl)?;
                }
                Cell::Bool(b) => {
                    sheet.write_boolean(r, col, b).map_err(xl)?;
                }
            }
        }
    }
    sheet.set_freeze_panes(1, 0).map_err(xl)?;
    if !columns.is_empty() {
        sheet.autofit();
    }
    book.save_to_buffer().map_err(xl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_csv_with_sniffed_delimiter_and_quotes() {
        let f = read_file("people.csv", "\u{feff}Name;Note\r\nAda;\"a; b\"\r\nBo;\r\n\r\n".as_bytes()).unwrap();
        assert_eq!(f.sheets.len(), 1);
        assert_eq!(f.sheets[0].name, "people");
        assert_eq!(f.sheets[0].rows, vec![vec!["Name", "Note"], vec!["Ada", "a; b"], vec!["Bo"]]);
    }

    #[test]
    fn reads_latin1_and_tabs() {
        let f = read_file("x.tsv", b"A\tB\ncaf\xE9\t2\n").unwrap();
        assert_eq!(f.sheets[0].rows[1], vec!["café", "2"]);
    }

    #[test]
    fn refuses_unknown_and_empty_files() {
        assert!(read_file("x.pdf", b"%PDF").is_err());
        assert!(read_file("x.csv", b"\n\n").is_err());
    }

    #[test]
    fn excel_serials() {
        assert_eq!(excel_serial_to_iso(45_292.0), "2024-01-01");
        assert_eq!(excel_serial_to_iso(45_292.5), "2024-01-01T12:00:00");
        assert_eq!(excel_serial_to_iso(1.0), "1899-12-31");
    }

    #[test]
    fn exports_csv_with_display_values() {
        let cols = vec!["Price".to_string(), "Paid".into(), "Name".into()];
        let types = vec![Some(FieldType::Currency { scale: 2 }), Some(FieldType::YesNo), None];
        let rows = vec![vec![json!(-1250), json!(1), json!("a,b")], vec![json!(5), json!(0), json!(null)]];
        let out = write_rows(ExportFormat::Csv, "t", &cols, &types, &rows).unwrap();
        let text = String::from_utf8(out[3..].to_vec()).unwrap();
        assert_eq!(text, "Price,Paid,Name\n-12.50,Yes,\"a,b\"\n0.05,No,\n");
    }

    #[test]
    fn xlsx_round_trips_through_the_reader() {
        let cols = vec!["Name".to_string(), "Price".into(), "Paid".into()];
        let types = vec![None, Some(FieldType::Currency { scale: 2 }), Some(FieldType::YesNo)];
        let rows = vec![vec![json!("Ada"), json!(1250), json!(1)]];
        let out = write_rows(ExportFormat::Xlsx, "Orders: 2024", &cols, &types, &rows).unwrap();
        let f = read_file("o.xlsx", &out).unwrap();
        assert_eq!(f.sheets[0].name, "Orders 2024");
        assert_eq!(f.sheets[0].rows, vec![vec!["Name", "Price", "Paid"], vec!["Ada", "12.5", "Yes"]]);
    }
}
