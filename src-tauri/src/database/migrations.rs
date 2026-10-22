//! Schema changes. Every change runs in a transaction; structural changes
//! that SQLite's ALTER TABLE cannot express use the documented table-rebuild
//! procedure (https://sqlite.org/lang_altertable.html#otheralter):
//! foreign keys off, create new table, copy, drop, rename, recreate indexes
//! and triggers, verify foreign keys and views, commit.

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension};

use crate::errors::{AppError, AppResult};
use crate::models::{FieldDef, FieldType, SchemaChangeResult, TableDef, TableDesign};
use crate::validation::expressions::rename_identifier;
use crate::validation::identifiers::{quote_ident, validate_identifier};
use crate::validation::table::validate_table_def;

use super::metadata;
use super::schema::{
    self, create_table_sql, dependent_objects, find_object, foreign_key_violations, load_table,
    require_table, rowid_alias, rowid_name, working_views,
};
use super::transactions::{ForeignKeysOff, with_transaction};

fn current_sql(conn: &Connection, name: &str) -> AppResult<String> {
    find_object(conn, name)?
        .and_then(|o| o.sql)
        .ok_or_else(|| AppError::Internal(format!("schema entry for {name:?} disappeared")))
}

fn ensure_name_free(conn: &Connection, name: &str, except: Option<&str>) -> AppResult<()> {
    if let Some(existing) = find_object(conn, name)? {
        if except.is_some_and(|e| e.eq_ignore_ascii_case(&existing.name)) {
            return Ok(());
        }
        return Err(AppError::AlreadyExists(format!(
            "a {} named {:?} already exists",
            existing.kind, existing.name
        )));
    }
    Ok(())
}

/// Definitions of all tables whose metadata matches the live schema.
fn managed_tables(conn: &Connection) -> AppResult<Vec<TableDef>> {
    let mut out = Vec::new();
    for name in metadata::stored_names(conn)? {
        if find_object(conn, &name)?.is_some_and(|o| o.kind == "table") {
            let loaded = load_table(conn, &name)?;
            if loaded.managed {
                out.push(loaded.definition);
            }
        }
    }
    Ok(out)
}

/// Resolves and checks foreign key targets: the referenced table must exist
/// and the referenced columns must be its primary key or a unique key, or
/// SQLite reports "foreign key mismatch" on every later write.
fn resolve_foreign_keys(conn: &Connection, mut def: TableDef) -> AppResult<TableDef> {
    let this = def.clone();
    for fk in &mut def.foreign_keys {
        let (target, unique_sets) = if fk.ref_table.eq_ignore_ascii_case(&this.name) {
            (this.clone(), Vec::new())
        } else {
            let loaded = load_table(conn, &fk.ref_table).map_err(|_| {
                AppError::Validation(format!("referenced table {:?} does not exist", fk.ref_table))
            })?;
            let sets: Vec<HashSet<String>> = schema::indexes(conn, &loaded.definition.name)?
                .into_iter()
                .filter(|i| i.unique && !i.partial)
                .map(|i| i.columns.iter().map(|c| c.to_lowercase()).collect())
                .collect();
            (loaded.definition, sets)
        };
        fk.ref_table = target.name.clone();
        if fk.ref_columns.is_empty() {
            fk.ref_columns = target.primary_key().iter().map(|f| f.name.clone()).collect();
            if fk.ref_columns.is_empty() {
                return Err(AppError::Validation(format!(
                    "table {:?} has no primary key; name the referenced columns",
                    target.name
                )));
            }
        }
        for c in &mut fk.ref_columns {
            let f = target.field(c).ok_or_else(|| {
                AppError::Validation(format!("referenced column {:?}.{c:?} does not exist", target.name))
            })?;
            *c = f.name.clone();
        }
        if fk.ref_columns.len() != fk.columns.len() {
            return Err(AppError::Validation(format!(
                "foreign key to {:?}: column count does not match",
                target.name
            )));
        }
        let wanted: HashSet<String> = fk.ref_columns.iter().map(|c| c.to_lowercase()).collect();
        let pk: HashSet<String> =
            target.primary_key().iter().map(|f| f.name.to_lowercase()).collect();
        let single_unique = fk.ref_columns.len() == 1
            && target.field(&fk.ref_columns[0]).is_some_and(|f| f.unique);
        let constraint_unique = target
            .unique_constraints
            .iter()
            .any(|u| u.iter().map(|c| c.to_lowercase()).collect::<HashSet<_>>() == wanted);
        if wanted != pk && !single_unique && !constraint_unique && !unique_sets.contains(&wanted) {
            return Err(AppError::Validation(format!(
                "referenced columns {:?} of {:?} must be its primary key or unique",
                fk.ref_columns, target.name
            )));
        }
    }
    Ok(def)
}

pub fn create_table(conn: &Connection, def: &TableDef) -> AppResult<TableDef> {
    validate_table_def(def)?;
    with_transaction(conn, |tx| {
        ensure_name_free(tx, &def.name, None)?;
        let def = resolve_foreign_keys(tx, def.clone())?;
        tx.execute_batch(&create_table_sql(&def, &def.name)?)?;
        metadata::save(tx, &def, &current_sql(tx, &def.name)?)?;
        Ok(def)
    })
}

pub fn drop_table(conn: &Connection, name: &str, confirm: bool) -> AppResult<SchemaChangeResult> {
    let table = require_table(conn, name)?.name;
    if !confirm {
        return Err(AppError::ConfirmationRequired(format!(
            "deleting table {table:?} permanently deletes all of its records"
        )));
    }
    let referencing = schema::referencing_tables(conn, &table)?;
    if !referencing.is_empty() {
        return Err(AppError::Constraint(format!(
            "table {table:?} is referenced by {referencing:?}; remove those relationships first"
        )));
    }
    with_transaction(conn, |tx| {
        let views_before = working_views(tx)?;
        tx.execute_batch(&format!("DROP TABLE {}", quote_ident(&table)))?;
        metadata::delete(tx, &table)?;
        let views_after = working_views(tx)?;
        let warnings = views_before
            .into_iter()
            .filter(|v| !views_after.contains(v))
            .map(|v| format!("view {v:?} no longer works"))
            .collect();
        Ok(SchemaChangeResult { rebuilt: false, warnings })
    })
}

pub fn rename_table(conn: &Connection, old: &str, new: &str) -> AppResult<()> {
    validate_identifier(new)?;
    let old = require_table(conn, old)?.name;
    with_transaction(conn, |tx| {
        ensure_name_free(tx, new, Some(&old))?;
        let snapshot = managed_tables(tx)?;
        tx.execute_batch(&format!("ALTER TABLE {} RENAME TO {}", quote_ident(&old), quote_ident(new)))?;
        metadata::delete(tx, &old)?;
        for mut def in snapshot {
            if def.name.eq_ignore_ascii_case(&old) {
                def.name = new.to_string();
            }
            for fk in &mut def.foreign_keys {
                if fk.ref_table.eq_ignore_ascii_case(&old) {
                    fk.ref_table = new.to_string();
                }
            }
            metadata::save(tx, &def, &current_sql(tx, &def.name)?)?;
        }
        Ok(())
    })
}

/// Applies a column rename of `table.from` to a definition: the table's own
/// fields, constraints and expressions, and references from other tables.
fn apply_column_rename(mut def: TableDef, table: &str, from: &str, to: &str) -> AppResult<TableDef> {
    let rename = |s: &mut String| {
        if s.eq_ignore_ascii_case(from) {
            *s = to.to_string();
        }
    };
    if def.name.eq_ignore_ascii_case(table) {
        for f in &mut def.fields {
            rename(&mut f.name);
            if let Some(c) = &f.check {
                f.check = Some(rename_identifier(c, from, to)?);
            }
            if let Some(crate::models::DefaultValue::Expression(e)) = &f.default {
                f.default = Some(crate::models::DefaultValue::Expression(rename_identifier(e, from, to)?));
            }
            if let FieldType::Calculated { expression, .. } = &mut f.field_type {
                *expression = rename_identifier(expression, from, to)?;
            }
        }
        for c in &mut def.checks {
            *c = rename_identifier(c, from, to)?;
        }
        for fk in &mut def.foreign_keys {
            fk.columns.iter_mut().for_each(rename);
        }
        for u in &mut def.unique_constraints {
            u.iter_mut().for_each(rename);
        }
    }
    let lookups_into_table: Vec<String> = def
        .foreign_keys
        .iter()
        .filter(|fk| fk.ref_table.eq_ignore_ascii_case(table) && fk.columns.len() == 1)
        .map(|fk| fk.columns[0].clone())
        .collect();
    for fk in &mut def.foreign_keys {
        if fk.ref_table.eq_ignore_ascii_case(table) {
            fk.ref_columns.iter_mut().for_each(rename);
        }
    }
    for f in &mut def.fields {
        if let FieldType::Lookup { display_column: Some(d), .. } = &mut f.field_type
            && lookups_into_table.iter().any(|c| c.eq_ignore_ascii_case(&f.name)) {
                rename(d);
            }
    }
    Ok(def)
}

fn rename_field_in_tx(conn: &Connection, table: &str, from: &str, to: &str) -> AppResult<()> {
    validate_identifier(to)?;
    let loaded = load_table(conn, table)?;
    let field = loaded
        .definition
        .field(from)
        .ok_or_else(|| AppError::NotFound(format!("field {from:?} in table {table:?}")))?;
    let from = field.name.clone();
    if let Some(existing) = loaded.definition.field(to)
        && !existing.name.eq_ignore_ascii_case(&from) {
            return Err(AppError::AlreadyExists(format!("field {to:?} in table {table:?}")));
        }
    let table = loaded.definition.name.clone();
    let snapshot = managed_tables(conn)?;
    conn.execute_batch(&format!(
        "ALTER TABLE {} RENAME COLUMN {} TO {}",
        quote_ident(&table),
        quote_ident(&from),
        quote_ident(to)
    ))?;
    for def in snapshot {
        let def = apply_column_rename(def, &table, &from, to)?;
        metadata::save(conn, &def, &current_sql(conn, &def.name)?)?;
    }
    Ok(())
}

pub fn rename_field(conn: &Connection, table: &str, from: &str, to: &str) -> AppResult<()> {
    with_transaction(conn, |tx| rename_field_in_tx(tx, table, from, to))
}

/// SQL that converts a copied value between field types where the stored
/// representation changes. Values that cannot be converted are passed through
/// unchanged so the new column's CHECK constraints reject them.
fn convert_expr(src: &str, old: &FieldType, new: &FieldType) -> String {
    use FieldType::*;
    let guard = |e: String| format!("CASE WHEN {src} IS NULL THEN NULL ELSE {e} END");
    match (old, new) {
        (Currency { scale: a }, Currency { scale: b }) if a == b => src.to_string(),
        (Currency { scale: a }, Currency { scale: b }) if b > a => {
            format!("{src} * {}", 10i64.pow((b - a) as u32))
        }
        (Currency { scale: a }, Currency { scale: b }) => {
            format!("CAST(ROUND({src} / {}.0) AS INTEGER)", 10i64.pow((a - b) as u32))
        }
        // Numbers and numeric text ("12", "-3.5") are in major units; other
        // text is kept as is, so the column's type check rejects the change.
        (_, Currency { scale }) => {
            let factor = 10i64.pow(*scale as u32);
            format!(
                "CASE WHEN typeof({src}) IN ('integer', 'real') THEN CAST(ROUND({src} * {factor}) AS INTEGER) \
                 WHEN typeof({src}) = 'text' AND {} THEN CAST(ROUND(CAST(trim({src}) AS REAL) * {factor}) AS INTEGER) \
                 ELSE {src} END",
                numeric_text(&format!("trim({src})"))
            )
        }
        (Currency { scale }, Number { .. } | LargeNumber) => {
            format!("{src} / {}.0", 10i64.pow(*scale as u32))
        }
        (Currency { scale }, ShortText { .. } | LongText) => guard(format!(
            "printf('%.{scale}f', {src} / {}.0)",
            10i64.pow(*scale as u32)
        )),
        (YesNo, YesNo) => src.to_string(),
        (_, YesNo) => format!(
            "CASE WHEN {src} IS NULL THEN NULL \
             WHEN typeof({src}) IN ('integer', 'real') THEN ({src} <> 0) \
             WHEN lower({src}) IN ('1', '-1', 'true', 'yes', 'on') THEN 1 \
             WHEN lower({src}) IN ('0', 'false', 'no', 'off') THEN 0 \
             ELSE {src} END"
        ),
        _ => src.to_string(),
    }
}

/// SQL condition: `text` is a plain decimal like "12", "-3.5" or ".25".
fn numeric_text(text: &str) -> String {
    format!(
        "({text} GLOB '*[0-9]*' AND {text} NOT GLOB '*[^0-9.+-]*' AND {text} NOT GLOB '*.*.*' \
         AND {text} NOT GLOB '?*[+-]*')"
    )
}

fn unused_temp_name(conn: &Connection) -> AppResult<String> {
    for n in 0.. {
        let name = format!("__app_rebuild_{n}");
        if find_object(conn, &name)?.is_none() {
            return Ok(name);
        }
    }
    unreachable!()
}

/// Rebuilds `current.name` with `new_def`, copying same-named columns.
/// Must run inside a transaction with foreign keys disabled.
fn rebuild_table(
    conn: &Connection,
    current: &TableDef,
    new_def: &TableDef,
    warnings: &mut Vec<String>,
) -> AppResult<()> {
    let table = current.name.as_str();
    let deps = dependent_objects(conn, table)?;
    let old_seq: Option<i64> = if find_object(conn, "sqlite_sequence")?.is_some() {
        conn.query_row("SELECT seq FROM sqlite_sequence WHERE name = ?1", [table], |r| r.get(0))
            .optional()?
    } else {
        None
    };

    let tmp = unused_temp_name(conn)?;
    conn.execute_batch(&create_table_sql(new_def, &tmp)?)?;

    let mut targets = Vec::new();
    let mut sources = Vec::new();
    if rowid_alias(new_def).is_none() {
        targets.push(rowid_name(new_def)?.to_string());
        sources.push(format!("{}.{}", quote_ident(table), rowid_name(current)?));
    }
    for f in &new_def.fields {
        if f.field_type.is_calculated() {
            continue;
        }
        if let Some(old) = current.field(&f.name) {
            if old.field_type.is_calculated() {
                continue;
            }
            targets.push(quote_ident(&f.name));
            sources.push(convert_expr(&quote_ident(&old.name), &old.field_type, &f.field_type));
        }
    }
    if !targets.is_empty() {
        conn.execute(
            &format!(
                "INSERT INTO {} ({}) SELECT {} FROM {}",
                quote_ident(&tmp),
                targets.join(", "),
                sources.join(", "),
                quote_ident(table)
            ),
            [],
        )
        .map_err(|e| AppError::from(e).context("existing data does not fit the new design"))?;
    }

    conn.execute_batch(&format!("DROP TABLE {}", quote_ident(table)))?;
    // Legacy mode stops SQLite from re-parsing views that reference the
    // (temporarily missing) table during the rename.
    conn.pragma_update(None, "legacy_alter_table", true)?;
    let renamed = conn.execute_batch(&format!(
        "ALTER TABLE {} RENAME TO {}",
        quote_ident(&tmp),
        quote_ident(table)
    ));
    conn.pragma_update(None, "legacy_alter_table", false)?;
    renamed?;

    if let Some(seq) = old_seq
        && new_def.fields.iter().any(|f| matches!(f.field_type, FieldType::AutoNumber)) {
            conn.execute(
                "UPDATE sqlite_sequence SET seq = max(seq, ?2) WHERE name = ?1",
                rusqlite::params![table, seq],
            )?;
            conn.execute(
                "INSERT INTO sqlite_sequence (name, seq)
                 SELECT ?1, ?2 WHERE NOT EXISTS (SELECT 1 FROM sqlite_sequence WHERE name = ?1)",
                rusqlite::params![table, seq],
            )?;
        }

    for dep in deps {
        if dep.kind == "index"
            && let Some(missing) = dep
                .columns
                .iter()
                .find(|c| *c != "<expression>" && new_def.field(c).is_none())
            {
                warnings.push(format!("index {:?} was removed because field {missing:?} was removed", dep.name));
                continue;
            }
        conn.execute_batch(&dep.sql).map_err(|e| {
            AppError::from(e).context(format!("could not recreate {} {:?}", dep.kind, dep.name))
        })?;
    }
    Ok(())
}

/// Applies a complete table design. Field renames are applied first with
/// ALTER TABLE RENAME COLUMN; if the resulting schema still differs, the
/// table is rebuilt. Removing fields requires `confirm_data_loss`.
pub fn save_table_design(
    conn: &Connection,
    table: &str,
    design: &TableDesign,
    confirm_data_loss: bool,
) -> AppResult<SchemaChangeResult> {
    let loaded = load_table(conn, table)?;
    let table = loaded.definition.name.clone();
    if !loaded.unsupported_features.is_empty() {
        return Err(AppError::Unsupported(format!(
            "table {table:?} uses features this designer cannot preserve: {}",
            loaded.unsupported_features.join(", ")
        )));
    }
    let mut new_def = design.definition.clone();
    if !new_def.name.eq_ignore_ascii_case(&table) {
        return Err(AppError::Validation("use rename_table to rename a table".into()));
    }
    new_def.name = table.clone();
    validate_table_def(&new_def)?;

    let mut planned = loaded.definition.clone();
    let mut seen = HashSet::new();
    for r in &design.renames {
        validate_identifier(&r.to)?;
        if !seen.insert(r.from.to_lowercase()) {
            return Err(AppError::Validation(format!("field {:?} renamed twice", r.from)));
        }
        let from = planned
            .field(&r.from)
            .ok_or_else(|| AppError::NotFound(format!("field {:?} in table {table:?}", r.from)))?
            .name
            .clone();
        if new_def.field(&r.to).is_none() {
            return Err(AppError::Validation(format!("renamed field {:?} is not in the design", r.to)));
        }
        if planned.field(&r.to).is_some_and(|f| !f.name.eq_ignore_ascii_case(&from)) {
            return Err(AppError::AlreadyExists(format!("field {:?} in table {table:?}", r.to)));
        }
        planned = apply_column_rename(planned, &table, &from, &r.to)?;
    }

    let dropped: Vec<String> = planned
        .fields
        .iter()
        .filter(|f| new_def.field(&f.name).is_none())
        .map(|f| f.name.clone())
        .collect();
    if !dropped.is_empty() && !confirm_data_loss {
        return Err(AppError::ConfirmationRequired(format!(
            "removing fields {dropped:?} permanently deletes their data"
        )));
    }

    let rows = schema::row_count(conn, &table)?;
    if rows > 0 {
        for f in &new_def.fields {
            let added = planned.field(&f.name).is_none();
            let auto = matches!(f.field_type, FieldType::AutoNumber | FieldType::Calculated { .. });
            if added && (f.required || f.primary_key) && f.default.is_none() && !auto {
                return Err(AppError::Validation(format!(
                    "new required field {:?} needs a default value because the table has {rows} records",
                    f.name
                )));
            }
        }
    }

    let new_def = resolve_foreign_keys(conn, new_def)?;
    let needs_rebuild =
        create_table_sql(&planned, &table)? != create_table_sql(&new_def, &table)?;
    let _fk_guard = if needs_rebuild { Some(ForeignKeysOff::new(conn)?) } else { None };

    with_transaction(conn, |tx| {
        let fk_before = foreign_key_violations(tx)?;
        let views_before = working_views(tx)?;
        for r in &design.renames {
            if !r.from.eq(&r.to) {
                rename_field_in_tx(tx, &table, &r.from, &r.to)?;
            }
        }
        let mut warnings = Vec::new();
        if needs_rebuild {
            let current = load_table(tx, &table)?.definition;
            rebuild_table(tx, &current, &new_def, &mut warnings)?;
        }
        let new_violations: Vec<_> = foreign_key_violations(tx)?
            .into_iter()
            .filter(|v| !fk_before.contains(v))
            .collect();
        if let Some((child, rowid, parent)) = new_violations.first() {
            return Err(AppError::Constraint(format!(
                "{} record(s) would break relationships, e.g. {child:?} row {} has no matching {parent:?} record",
                new_violations.len(),
                rowid.map_or("?".into(), |r| r.to_string())
            )));
        }
        let views_after = working_views(tx)?;
        if let Some(broken) = views_before.iter().find(|v| !views_after.contains(v)) {
            return Err(AppError::Constraint(format!("the change would break view {broken:?}")));
        }
        metadata::save(tx, &new_def, &current_sql(tx, &table)?)?;
        Ok(SchemaChangeResult { rebuilt: needs_rebuild, warnings })
    })
}

pub fn add_field(
    conn: &Connection,
    table: &str,
    field: FieldDef,
    position: Option<usize>,
) -> AppResult<SchemaChangeResult> {
    let mut def = load_table(conn, table)?.definition;
    let at = position.unwrap_or(def.fields.len()).min(def.fields.len());
    def.fields.insert(at, field);
    save_table_design(conn, table, &TableDesign { definition: def, renames: vec![] }, false)
}

/// Replaces field `name` with `field`; a different `field.name` renames it.
pub fn modify_field(
    conn: &Connection,
    table: &str,
    name: &str,
    field: FieldDef,
) -> AppResult<SchemaChangeResult> {
    let loaded = load_table(conn, table)?;
    let old = loaded
        .definition
        .field(name)
        .ok_or_else(|| AppError::NotFound(format!("field {name:?} in table {table:?}")))?
        .name
        .clone();
    let table = loaded.definition.name.clone();
    let mut def = apply_column_rename(loaded.definition, &table, &old, &field.name)?;
    let idx = def
        .fields
        .iter()
        .position(|f| f.name.eq_ignore_ascii_case(&field.name))
        .expect("renamed field present");
    let renames = if old == field.name {
        vec![]
    } else {
        vec![crate::models::ColumnRename { from: old, to: field.name.clone() }]
    };
    def.fields[idx] = field;
    save_table_design(conn, &table, &TableDesign { definition: def, renames }, false)
}

/// Removes a field together with relationships and unique constraints that
/// include it.
pub fn drop_field(
    conn: &Connection,
    table: &str,
    name: &str,
    confirm: bool,
) -> AppResult<SchemaChangeResult> {
    let mut def = load_table(conn, table)?.definition;
    let before = def.fields.len();
    def.fields.retain(|f| !f.name.eq_ignore_ascii_case(name));
    if def.fields.len() == before {
        return Err(AppError::NotFound(format!("field {name:?} in table {table:?}")));
    }
    let mut warnings = Vec::new();
    def.foreign_keys.retain(|fk| {
        let keep = !fk.columns.iter().any(|c| c.eq_ignore_ascii_case(name));
        if !keep {
            warnings.push(format!("relationship to {:?} was removed", fk.ref_table));
        }
        keep
    });
    def.unique_constraints.retain(|u| {
        let keep = !u.iter().any(|c| c.eq_ignore_ascii_case(name));
        if !keep {
            warnings.push(format!("unique constraint on {u:?} was removed"));
        }
        keep
    });
    let mut result =
        save_table_design(conn, table, &TableDesign { definition: def, renames: vec![] }, confirm)?;
    warnings.append(&mut result.warnings);
    result.warnings = warnings;
    Ok(result)
}
