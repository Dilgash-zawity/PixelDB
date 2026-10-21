//! Conversion between JSON values from the frontend and SQLite values,
//! applying the rules of each Access-style field type.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rusqlite::types::{Value, ValueRef};
use serde_json::{Number, Value as JsonValue, json};

use crate::errors::{AppError, AppResult};
use crate::models::{FieldDef, FieldType, LookupValueType};

use super::identifiers::quote_literal;

/// Largest attachment accepted through IPC.
pub const MAX_ATTACHMENT_BYTES: usize = 32 * 1024 * 1024;

fn invalid(field: &FieldDef, why: impl std::fmt::Display) -> AppError {
    AppError::Validation(format!("field {:?}: {why}", field.name))
}

/// Converts a JSON value into the SQLite value stored for `field`.
///
/// Strings are accepted for every type (HTML inputs produce strings); an empty
/// string means NULL for non-text types. Currency accepts an integer number of
/// minor units or a decimal string in major units ("12.34").
pub fn coerce_value(field: &FieldDef, input: &JsonValue) -> AppResult<Value> {
    if input.is_null() {
        return Ok(Value::Null);
    }
    let text_input = input.as_str();
    let is_text_type = matches!(
        field.field_type,
        FieldType::ShortText { .. } | FieldType::LongText | FieldType::Hyperlink
    ) || matches!(
        field.field_type,
        FieldType::Lookup { value_type: LookupValueType::Text, .. }
    );
    if !is_text_type && text_input.is_some_and(|s| s.trim().is_empty()) {
        return Ok(Value::Null);
    }

    match &field.field_type {
        FieldType::ShortText { max_length } => {
            let s = as_text(field, input)?;
            if let Some(max) = max_length
                && s.chars().count() > *max as usize {
                    return Err(invalid(field, format!("text longer than {max} characters")));
                }
            Ok(Value::Text(s))
        }
        FieldType::LongText | FieldType::Hyperlink => Ok(Value::Text(as_text(field, input)?)),
        FieldType::Number { real: true } => Ok(Value::Real(as_real(field, input)?)),
        FieldType::Number { real: false }
        | FieldType::LargeNumber
        | FieldType::AutoNumber
        | FieldType::Lookup { value_type: LookupValueType::Integer, .. } => {
            Ok(Value::Integer(as_integer(field, input)?))
        }
        FieldType::Lookup { value_type: LookupValueType::Text, .. } => {
            Ok(Value::Text(as_text(field, input)?))
        }
        FieldType::Currency { scale } => match input {
            JsonValue::String(s) => Ok(Value::Integer(parse_decimal_minor(field, s, *scale)?)),
            _ => Ok(Value::Integer(as_integer(field, input)?)),
        },
        FieldType::DateTime => {
            let s = input
                .as_str()
                .ok_or_else(|| invalid(field, "expected an ISO 8601 date/time string"))?
                .trim();
            if !is_iso8601(s) {
                return Err(invalid(field, format!("{s:?} is not an ISO 8601 date/time")));
            }
            Ok(Value::Text(s.to_string()))
        }
        FieldType::YesNo => Ok(Value::Integer(as_bool(field, input)? as i64)),
        FieldType::Attachment => Ok(Value::Blob(as_blob(field, input)?)),
        FieldType::Calculated { .. } => Err(invalid(field, "calculated fields are read-only")),
        FieldType::Unknown { .. } => untyped_value(input),
    }
}

/// Converts JSON to SQLite without type information.
pub fn untyped_value(input: &JsonValue) -> AppResult<Value> {
    Ok(match input {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(b) => Value::Integer(*b as i64),
        JsonValue::Number(n) => match n.as_i64() {
            Some(i) => Value::Integer(i),
            None => Value::Real(n.as_f64().unwrap_or(f64::NAN)),
        },
        JsonValue::String(s) => Value::Text(s.clone()),
        JsonValue::Object(o) if o.contains_key("base64") => {
            let data = o["base64"]
                .as_str()
                .ok_or_else(|| AppError::Validation("base64 must be a string".into()))?;
            Value::Blob(decode_base64(data)?)
        }
        _ => {
            return Err(AppError::Validation(format!("unsupported value: {input}")));
        }
    })
}

/// Converts a SQLite value to JSON. Blobs are summarized, not transferred.
pub fn sql_to_json(value: ValueRef<'_>) -> JsonValue {
    match value {
        ValueRef::Null => JsonValue::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => Number::from_f64(f).map(JsonValue::Number).unwrap_or_else(|| json!(f.to_string())),
        ValueRef::Text(t) => JsonValue::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => json!({ "blob": { "size": b.len() } }),
    }
}

/// Renders a value as a SQL literal for DDL (DEFAULT clauses).
pub fn render_literal(value: &Value) -> AppResult<String> {
    Ok(match value {
        Value::Null => "NULL".into(),
        Value::Integer(i) => i.to_string(),
        Value::Real(f) => {
            if !f.is_finite() {
                return Err(AppError::Validation("default value must be finite".into()));
            }
            format!("{f:?}")
        }
        Value::Text(s) => quote_literal(s),
        Value::Blob(b) => {
            let hex: String = b.iter().map(|byte| format!("{byte:02X}")).collect();
            format!("X'{hex}'")
        }
    })
}

fn as_text(field: &FieldDef, input: &JsonValue) -> AppResult<String> {
    match input {
        JsonValue::String(s) => Ok(s.clone()),
        JsonValue::Number(n) => Ok(n.to_string()),
        JsonValue::Bool(b) => Ok(b.to_string()),
        _ => Err(invalid(field, "expected text")),
    }
}

fn as_integer(field: &FieldDef, input: &JsonValue) -> AppResult<i64> {
    match input {
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Ok(i);
            }
            let f = n.as_f64().unwrap_or(f64::NAN);
            if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
                Ok(f as i64)
            } else {
                Err(invalid(field, format!("{n} is not a whole number")))
            }
        }
        JsonValue::String(s) => s
            .trim()
            .parse::<i64>()
            .map_err(|_| invalid(field, format!("{s:?} is not a whole number"))),
        JsonValue::Bool(b) => Ok(*b as i64),
        _ => Err(invalid(field, "expected a whole number")),
    }
}

fn as_real(field: &FieldDef, input: &JsonValue) -> AppResult<f64> {
    let f = match input {
        JsonValue::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        JsonValue::String(s) => s
            .trim()
            .parse::<f64>()
            .map_err(|_| invalid(field, format!("{s:?} is not a number")))?,
        _ => return Err(invalid(field, "expected a number")),
    };
    if !f.is_finite() {
        return Err(invalid(field, "number must be finite"));
    }
    Ok(f)
}

fn as_bool(field: &FieldDef, input: &JsonValue) -> AppResult<bool> {
    match input {
        JsonValue::Bool(b) => Ok(*b),
        JsonValue::Number(n) => match n.as_i64() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(invalid(field, "expected 0 or 1")),
        },
        JsonValue::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(invalid(field, format!("{s:?} is not yes/no"))),
        },
        _ => Err(invalid(field, "expected yes/no")),
    }
}

fn as_blob(field: &FieldDef, input: &JsonValue) -> AppResult<Vec<u8>> {
    let data = input
        .get("base64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| invalid(field, "attachments must be sent as {\"base64\": \"...\"}"))?;
    decode_base64(data).map_err(|e| invalid(field, e))
}

fn decode_base64(data: &str) -> AppResult<Vec<u8>> {
    if data.len() / 4 * 3 > MAX_ATTACHMENT_BYTES {
        return Err(AppError::Validation(format!(
            "attachment exceeds {MAX_ATTACHMENT_BYTES} bytes"
        )));
    }
    BASE64
        .decode(data)
        .map_err(|e| AppError::Validation(format!("invalid base64: {e}")))
}

/// Parses "-1234.5" into minor units with `scale` fractional digits, exactly.
fn parse_decimal_minor(field: &FieldDef, raw: &str, scale: u8) -> AppResult<i64> {
    let err = || invalid(field, format!("{raw:?} is not a valid amount with at most {scale} decimals"));
    let s = raw.trim();
    let (negative, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int_part, frac_part) = match digits.split_once('.') {
        Some((i, f)) => (i, f),
        None => (digits, ""),
    };
    if (int_part.is_empty() && frac_part.is_empty())
        || !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
        || frac_part.len() > scale as usize
    {
        return Err(err());
    }
    let mut minor: i64 = 0;
    for c in int_part.chars().chain(frac_part.chars()) {
        minor = minor
            .checked_mul(10)
            .and_then(|m| m.checked_add(c.to_digit(10).unwrap() as i64))
            .ok_or_else(err)?;
    }
    for _ in frac_part.len()..scale as usize {
        minor = minor.checked_mul(10).ok_or_else(err)?;
    }
    Ok(if negative { -minor } else { minor })
}

/// Accepts `YYYY-MM-DD`, optionally followed by `T` or a space and
/// `HH:MM[:SS[.fff]]`, optionally followed by `Z` or `±HH:MM`.
pub fn is_iso8601(s: &str) -> bool {
    fn digits(s: &str, n: usize) -> Option<(u32, &str)> {
        if s.len() < n || !s.is_char_boundary(n) || !s[..n].chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some((s[..n].parse().ok()?, &s[n..]))
    }
    fn hh_mm(s: &str) -> Option<&str> {
        let (h, rest) = digits(s, 2)?;
        let rest = rest.strip_prefix(':')?;
        let (m, rest) = digits(rest, 2)?;
        (h < 24 && m < 60).then_some(rest)
    }
    let parse = || -> Option<()> {
        let (year, rest) = digits(s, 4)?;
        let (month, rest) = digits(rest.strip_prefix('-')?, 2)?;
        let (day, rest) = digits(rest.strip_prefix('-')?, 2)?;
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => return None,
        };
        if day == 0 || day > days_in_month {
            return None;
        }
        if rest.is_empty() {
            return Some(());
        }
        let rest = rest.strip_prefix('T').or_else(|| rest.strip_prefix(' '))?;
        let mut rest = hh_mm(rest)?;
        if let Some(r) = rest.strip_prefix(':') {
            let (sec, r) = digits(r, 2)?;
            if sec > 59 {
                return None;
            }
            rest = r;
            if let Some(r) = rest.strip_prefix('.') {
                let n = r.chars().take_while(|c| c.is_ascii_digit()).count();
                if n == 0 {
                    return None;
                }
                rest = &r[n..];
            }
        }
        if rest.is_empty() || rest == "Z" {
            return Some(());
        }
        let r = rest.strip_prefix('+').or_else(|| rest.strip_prefix('-'))?;
        hh_mm(r)?.is_empty().then_some(())
    };
    parse().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FieldType;

    fn field(t: FieldType) -> FieldDef {
        FieldDef::new("f", t)
    }

    #[test]
    fn currency_parsing() {
        let f = field(FieldType::Currency { scale: 2 });
        assert_eq!(coerce_value(&f, &json!("12.34")).unwrap(), Value::Integer(1234));
        assert_eq!(coerce_value(&f, &json!("-0.5")).unwrap(), Value::Integer(-50));
        assert_eq!(coerce_value(&f, &json!("7")).unwrap(), Value::Integer(700));
        assert_eq!(coerce_value(&f, &json!(1234)).unwrap(), Value::Integer(1234));
        assert!(coerce_value(&f, &json!("1.234")).is_err());
        assert!(coerce_value(&f, &json!("abc")).is_err());
        assert!(coerce_value(&f, &json!("99999999999999999999")).is_err());
    }

    #[test]
    fn yes_no_and_numbers() {
        let yn = field(FieldType::YesNo);
        assert_eq!(coerce_value(&yn, &json!(true)).unwrap(), Value::Integer(1));
        assert_eq!(coerce_value(&yn, &json!("no")).unwrap(), Value::Integer(0));
        assert!(coerce_value(&yn, &json!(2)).is_err());
        let n = field(FieldType::Number { real: false });
        assert_eq!(coerce_value(&n, &json!(" 42 ")).unwrap(), Value::Integer(42));
        assert_eq!(coerce_value(&n, &json!("")).unwrap(), Value::Null);
        assert!(coerce_value(&n, &json!(1.5)).is_err());
    }

    #[test]
    fn text_limits_and_dates() {
        let t = field(FieldType::ShortText { max_length: Some(3) });
        assert!(coerce_value(&t, &json!("abc")).is_ok());
        assert!(coerce_value(&t, &json!("abcd")).is_err());
        assert_eq!(coerce_value(&t, &json!("")).unwrap(), Value::Text(String::new()));
        let d = field(FieldType::DateTime);
        for ok in ["2024-02-29", "2024-01-01T10:00", "2024-01-01 10:00:59.123Z", "2024-01-01T00:00:00+05:30"] {
            assert!(is_iso8601(ok), "{ok}");
        }
        for bad in ["2023-02-29", "2024-13-01", "24-01-01", "2024-01-01T25:00", "yesterday", "2024-01-01T10:00+5"] {
            assert!(!is_iso8601(bad), "{bad}");
        }
        assert!(coerce_value(&d, &json!("nope")).is_err());
    }

    #[test]
    fn literals() {
        assert_eq!(render_literal(&Value::Text("a'b".into())).unwrap(), "'a''b'");
        assert_eq!(render_literal(&Value::Real(1.0)).unwrap(), "1.0");
        assert_eq!(render_literal(&Value::Blob(vec![0xab, 1])).unwrap(), "X'AB01'");
    }
}
