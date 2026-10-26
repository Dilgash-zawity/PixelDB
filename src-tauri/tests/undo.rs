//! Undo and redo of record changes (database::journal) on real temp-file databases.

use app_lib::database::{OpenDatabase, journal, migrations, records};
use app_lib::errors::AppError;
use app_lib::models::*;
use serde_json::{Value, json};

fn temp_db() -> (tempfile::TempDir, OpenDatabase) {
    let dir = tempfile::tempdir().unwrap();
    let db = OpenDatabase::create(dir.path().join("t.db").to_str().unwrap()).unwrap();
    (dir, db)
}

fn id() -> FieldDef {
    let mut f = FieldDef::new("ID", FieldType::AutoNumber);
    f.primary_key = true;
    f
}

fn table(name: &str, fields: Vec<FieldDef>, foreign_keys: Vec<ForeignKeyDef>) -> TableDef {
    TableDef { name: name.into(), fields, foreign_keys, unique_constraints: vec![], checks: vec![], description: None }
}

fn values(v: Value) -> RecordValues {
    v.as_object().unwrap().clone()
}

fn names(db: &OpenDatabase, t: &str) -> Vec<Value> {
    records::list_records(&db.conn, t, &RecordQuery::default()).unwrap().rows.into_iter().map(|r| r.values[1].clone()).collect()
}

fn setup(db: &OpenDatabase) {
    migrations::create_table(&db.conn, &table("Customers", vec![id(), FieldDef::new("Name", FieldType::ShortText { max_length: None }), FieldDef::new("Photo", FieldType::Attachment)], vec![])).unwrap();
    let fk = ForeignKeyDef { columns: vec!["Customer".into()], ref_table: "Customers".into(), ref_columns: vec![], on_delete: FkAction::Cascade, on_update: FkAction::Cascade };
    let customer = FieldDef::new("Customer", FieldType::Lookup { value_type: LookupValueType::Integer, display_column: None });
    migrations::create_table(&db.conn, &table("Orders", vec![id(), FieldDef::new("Item", FieldType::ShortText { max_length: None }), customer], vec![fk])).unwrap();
}

#[test]
fn undo_and_redo_inserts_edits_and_deletes() {
    let (_d, mut db) = temp_db();
    setup(&db);
    let key = db.recorded("Add", |c| records::insert_record(c, "Customers", &values(json!({"Name": "Ada", "Photo": {"base64": "AAEC"}})))).unwrap();
    db.recorded("Edit", |c| records::update_record(c, "Customers", &key, &values(json!({"Name": "Ada L."})))).unwrap();
    assert_eq!(db.journal.state(), journal::UndoState { undo: Some("Edit".into()), redo: None });

    assert_eq!(journal::undo(&db.conn, &mut db.journal).unwrap().as_deref(), Some("Edit"));
    assert_eq!(names(&db, "Customers"), [json!("Ada")]);
    assert_eq!(journal::redo(&db.conn, &mut db.journal).unwrap().as_deref(), Some("Edit"));
    assert_eq!(names(&db, "Customers"), [json!("Ada L.")]);

    db.recorded("Delete", |c| records::delete_records(c, "Customers", std::slice::from_ref(&key), true)).unwrap();
    assert!(names(&db, "Customers").is_empty());
    journal::undo(&db.conn, &mut db.journal).unwrap();
    // The record comes back whole, file included, with its own ID.
    assert_eq!(names(&db, "Customers"), [json!("Ada L.")]);
    let photo = records::get_attachment(&db.conn, "Customers", &key, "Photo").unwrap();
    assert_eq!(photo["base64"], json!("AAEC"));

    // A new change ends what could be redone.
    journal::undo(&db.conn, &mut db.journal).unwrap();
    db.recorded("Add 2", |c| records::insert_record(c, "Customers", &values(json!({"Name": "Bo"})))).unwrap();
    assert_eq!(db.journal.state().redo, None);
}

#[test]
fn undo_brings_back_rows_removed_by_cascades() {
    let (_d, mut db) = temp_db();
    setup(&db);
    let ada = records::insert_record(&db.conn, "Customers", &values(json!({"Name": "Ada"}))).unwrap();
    let RecordKey::RowId(ada_id) = ada else { panic!() };
    records::insert_record(&db.conn, "Orders", &values(json!({"Item": "Pen", "Customer": ada_id}))).unwrap();
    db.recorded("Delete", |c| records::delete_records(c, "Customers", &[ada], true)).unwrap();
    assert!(names(&db, "Orders").is_empty());
    journal::undo(&db.conn, &mut db.journal).unwrap();
    assert_eq!(names(&db, "Orders"), [json!("Pen")]);
}

#[test]
fn undo_is_refused_when_the_record_changed_since() {
    let (_d, mut db) = temp_db();
    setup(&db);
    let key = db.recorded("Add", |c| records::insert_record(c, "Customers", &values(json!({"Name": "Ada"})))).unwrap();
    db.recorded("Edit", |c| records::update_record(c, "Customers", &key, &values(json!({"Name": "B"})))).unwrap();
    // Changed outside the journal (e.g. by another program).
    records::update_record(&db.conn, "Customers", &key, &values(json!({"Name": "C"}))).unwrap();
    let err = journal::undo(&db.conn, &mut db.journal).unwrap_err();
    assert!(matches!(err, AppError::Constraint(_)), "{err}");
    assert_eq!(names(&db, "Customers"), [json!("C")]);
    // The step is dropped; the older one is still there.
    assert_eq!(db.journal.state().undo.as_deref(), Some("Add"));
}

#[test]
fn structure_changes_clear_the_journal() {
    let (_d, mut db) = temp_db();
    setup(&db);
    db.recorded("Add", |c| records::insert_record(c, "Customers", &values(json!({"Name": "Ada"})))).unwrap();
    db.restructured(|c| migrations::add_field(c, "Customers", FieldDef::new("Email", FieldType::ShortText { max_length: None }), None)).unwrap();
    assert_eq!(db.journal.state(), journal::UndoState::default());
    // A failed structure change keeps it.
    db.recorded("Add", |c| records::insert_record(c, "Customers", &values(json!({"Name": "Bo"})))).unwrap();
    assert!(db.restructured(|c| migrations::drop_table(c, "Nope", true)).is_err());
    assert!(db.journal.state().undo.is_some());
}
