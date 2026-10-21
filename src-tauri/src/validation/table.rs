//! Structural validation of table definitions before any DDL is generated.

use std::collections::HashSet;

use crate::errors::{AppError, AppResult};
use crate::models::{DefaultValue, FieldType, TableDef};

use super::expressions::validate_expression;
use super::identifiers::validate_identifier;
use super::values::coerce_value;

pub const MAX_FIELDS: usize = 1000;
pub const MAX_DESCRIPTION_LEN: usize = 4000;

fn err(msg: String) -> AppError {
    AppError::Validation(msg)
}

fn check_description(owner: &str, d: &Option<String>) -> AppResult<()> {
    match d {
        Some(text) if text.chars().count() > MAX_DESCRIPTION_LEN => {
            Err(err(format!("description of {owner:?} is too long")))
        }
        _ => Ok(()),
    }
}

fn check_columns_exist(def: &TableDef, cols: &[String], what: &str) -> AppResult<()> {
    if cols.is_empty() {
        return Err(err(format!("{what} must list at least one column")));
    }
    let mut seen = HashSet::new();
    for c in cols {
        if def.field(c).is_none() {
            return Err(err(format!("{what} references unknown column {c:?}")));
        }
        if !seen.insert(c.to_lowercase()) {
            return Err(err(format!("{what} lists column {c:?} twice")));
        }
    }
    Ok(())
}

/// A declared type for `Unknown` fields must look like `NAME` or `NAME(1,2)`.
fn check_declared_type(declared: &str) -> AppResult<()> {
    let ok = declared
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '(' | ')' | ',' | '.' | '+' | '-'))
        && declared.matches('(').count() == declared.matches(')').count()
        && declared.matches('(').count() <= 1;
    if ok { Ok(()) } else { Err(err(format!("invalid declared type {declared:?}"))) }
}

pub fn validate_table_def(def: &TableDef) -> AppResult<()> {
    validate_identifier(&def.name)?;
    check_description(&def.name, &def.description)?;
    if def.fields.is_empty() {
        return Err(err(format!("table {:?} must have at least one field", def.name)));
    }
    if def.fields.len() > MAX_FIELDS {
        return Err(err(format!("table {:?} has too many fields", def.name)));
    }

    let mut names = HashSet::new();
    let pk_count = def.fields.iter().filter(|f| f.primary_key).count();
    let mut autonumbers = 0;
    for field in &def.fields {
        validate_identifier(&field.name)?;
        if !names.insert(field.name.to_lowercase()) {
            return Err(err(format!("duplicate field name {:?}", field.name)));
        }
        check_description(&field.name, &field.description)?;
        let fname = &field.name;
        match &field.field_type {
            FieldType::ShortText { max_length: Some(n) } if *n == 0 || *n > 1_000_000 => {
                return Err(err(format!("field {fname:?}: max length must be 1..1000000")));
            }
            FieldType::Currency { scale } if *scale > 6 => {
                return Err(err(format!("field {fname:?}: currency scale must be 0..6")));
            }
            FieldType::AutoNumber => {
                autonumbers += 1;
                if !field.primary_key || pk_count != 1 {
                    return Err(err(format!(
                        "field {fname:?}: an AutoNumber field must be the table's only primary key field"
                    )));
                }
                if field.default.is_some() {
                    return Err(err(format!("field {fname:?}: AutoNumber fields cannot have a default")));
                }
            }
            FieldType::Calculated { expression, .. } => {
                validate_expression(expression)?;
                if field.primary_key || field.default.is_some() {
                    return Err(err(format!(
                        "field {fname:?}: calculated fields cannot be primary keys or have defaults"
                    )));
                }
            }
            FieldType::Lookup { display_column, .. } => {
                if let Some(c) = display_column {
                    validate_identifier(c)?;
                }
                let covered = def.foreign_keys.iter().any(|fk| {
                    fk.columns.len() == 1 && fk.columns[0].eq_ignore_ascii_case(fname)
                });
                if !covered {
                    return Err(err(format!(
                        "lookup field {fname:?} needs a single-column foreign key"
                    )));
                }
            }
            FieldType::Unknown { declared } => check_declared_type(declared)?,
            _ => {}
        }
        if let Some(check) = &field.check {
            validate_expression(check)?;
        }
        match &field.default {
            Some(DefaultValue::Literal(v)) => {
                coerce_value(field, v)
                    .map_err(|e| err(format!("default of field {fname:?}: {e}")))?;
            }
            Some(DefaultValue::Expression(e)) => validate_expression(e)?,
            _ => {}
        }
    }
    if autonumbers > 1 {
        return Err(err("a table can have only one AutoNumber field".into()));
    }

    for fk in &def.foreign_keys {
        check_columns_exist(def, &fk.columns, "foreign key")?;
        validate_identifier(&fk.ref_table)?;
        for c in &fk.ref_columns {
            validate_identifier(c)?;
        }
        if !fk.ref_columns.is_empty() && fk.ref_columns.len() != fk.columns.len() {
            return Err(err(format!(
                "foreign key to {:?}: column count does not match referenced columns",
                fk.ref_table
            )));
        }
    }
    for cols in &def.unique_constraints {
        check_columns_exist(def, cols, "unique constraint")?;
    }
    for check in &def.checks {
        validate_expression(check)?;
    }
    Ok(())
}
