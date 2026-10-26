//! Import, export reads and indexes against real temp-file databases.

use app_lib::database::{OpenDatabase, indexes, migrations, queries, records, schema};
use app_lib::errors::AppError;
use app_lib::models::*;
use serde_json::{Value, json};

fn temp_db() -> (tempfile::TempDir, OpenDatabase) {
    let dir = tempfile::tempdir().unwrap();
    let db = OpenDatabase::create(dir.path().join("t.db").to_str().unwrap()).unwrap();
    (dir, db)
}

fn values(v: Value) -> RecordValues {
    v.as_object().unwrap().clone()
}

fn products(db: &OpenDatabase) {
    let mut id = FieldDef::new("id", FieldType::AutoNumber);
    id.primary_key = true;
    let mut name = FieldDef::new("name", FieldType::ShortText { max_length: None });
    name.required = true;
    migrations::create_table(
        &db.conn,
        &TableDef {
            name: "Products".into(),
            fields: vec![id, name, FieldDef::new("price", FieldType::Currency { scale: 2 }), FieldDef::new("sku", FieldType::ShortText { max_length: None })],
            foreign_keys: vec![],
            unique_constraints: vec![],
            checks: vec![],
            description: None,
        },
    )
    .unwrap();
}

#[test]
fn import_is_all_or_nothing_and_names_the_row() {
    let (_d, db) = temp_db();
    products(&db);
    let ok = vec![values(json!({"name": "Pen", "price": "1.50"})), values(json!({"name": "Ink", "price": "12"}))];
    assert_eq!(records::import(&db.conn, "Products", &ok).unwrap(), 2);
    let bad = vec![values(json!({"name": "Cup"})), values(json!({"price": "x"}))];
    let err = records::import(&db.conn, "Products", &bad).unwrap_err();
    assert!(err.to_string().contains("row 2"), "{err}");
    assert_eq!(schema::row_count(&db.conn, "Products").unwrap(), 2);
}

#[test]
fn list_all_ignores_paging_but_keeps_filters_and_sort() {
    let (_d, db) = temp_db();
    products(&db);
    let rows: Vec<_> = (0..250).map(|i| values(json!({"name": format!("P{i:03}"), "price": i}))).collect();
    records::import(&db.conn, "Products", &rows).unwrap();
    let q = RecordQuery {
        limit: 5,
        offset: 40,
        search: Some("P1".into()),
        sort: vec![SortSpec { column: "name".into(), descending: true }],
        ..Default::default()
    };
    let all = records::list_all(&db.conn, "Products", &q, 1_000_000).unwrap();
    assert_eq!(all.rows.len(), 100);
    assert_eq!(all.total, 100);
    assert_eq!(all.rows[0].values[1], json!("P199"));
    let capped = records::list_all(&db.conn, "Products", &RecordQuery::default(), 10).unwrap();
    assert_eq!((capped.rows.len(), capped.total), (10, 250));
    let q = queries::run_query_capped(&db.conn, "SELECT * FROM Products", &Default::default(), 2000).unwrap();
    assert_eq!(q.rows.len(), 250);
    assert!(!q.truncated);
}

#[test]
fn indexes_are_added_kept_through_redesigns_and_dropped() {
    let (_d, db) = temp_db();
    products(&db);
    let i = indexes::create_index(&db.conn, "products", &["Name".into(), "price".into()], false).unwrap();
    assert_eq!(i.name, "idx_Products_name_price");
    assert_eq!(i.columns, ["name", "price"]);
    assert!(matches!(
        indexes::create_index(&db.conn, "Products", &["name".into(), "price".into()], false),
        Err(AppError::AlreadyExists(_))
    ));
    assert!(indexes::create_index(&db.conn, "Products", &["nope".into()], false).is_err());

    // A redesign that rebuilds the table keeps the index.
    let mut def = schema::table_info(&db.conn, "Products").unwrap().definition;
    def.fields.push(FieldDef::new("note", FieldType::LongText));
    migrations::save_table_design(&db.conn, "Products", &TableDesign { definition: def, renames: vec![] }, false).unwrap();
    let info = schema::table_info(&db.conn, "Products").unwrap();
    assert!(info.managed);
    assert!(info.indexes.iter().any(|x| x.name == "idx_Products_name_price"));

    // Unique fails on duplicates with a plain message.
    records::import(&db.conn, "Products", &[values(json!({"name": "A", "sku": "1"})), values(json!({"name": "B", "sku": "1"}))]).unwrap();
    let err = indexes::create_index(&db.conn, "Products", &["sku".into()], true).unwrap_err();
    assert!(matches!(&err, AppError::Constraint(m) if m.contains("sku")), "{err}");

    indexes::drop_index(&db.conn, "Products", "idx_Products_name_price").unwrap();
    assert!(schema::table_info(&db.conn, "Products").unwrap().indexes.iter().all(|x| x.origin != "c"));
    assert!(indexes::drop_index(&db.conn, "Products", "idx_Products_name_price").is_err());
}

#[test]
fn documents_are_kept_opaquely() {
    use app_lib::database::metadata;
    let (_d, db) = temp_db();
    assert_eq!(metadata::load_document(&db.conn, "saved_views").unwrap(), json!({}));
    let v = json!({"Products": [{"name": "Cheap", "sort": null, "filters": [], "search": "pen"}]});
    metadata::save_document(&db.conn, "saved_views", &v).unwrap();
    metadata::save_document(&db.conn, "reports", &json!({"Sales": {}})).unwrap();
    assert_eq!(metadata::load_document(&db.conn, "saved_views").unwrap(), v);
    assert_eq!(metadata::load_document(&db.conn, "reports").unwrap(), json!({"Sales": {}}));
    assert!(metadata::save_document(&db.conn, "saved_views", &json!([1])).is_err());
    assert!(metadata::save_document(&db.conn, "version", &json!({})).is_err());
    assert!(metadata::load_document(&db.conn, "relationship_layout").is_err());
    // Documents are hidden metadata, not tables.
    assert!(schema::list_objects(&db.conn).unwrap().is_empty());
}
