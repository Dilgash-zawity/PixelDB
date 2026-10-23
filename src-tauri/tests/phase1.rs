//! Phase 1 acceptance tests. Every test uses a real SQLite file in a
//! temporary directory.

use app_lib::database::{OpenDatabase, metadata, migrations, queries, records, schema};
use app_lib::errors::AppError;
use app_lib::models::*;
use serde_json::{Value, json};

fn temp_db() -> (tempfile::TempDir, String, OpenDatabase) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db").to_str().unwrap().to_string();
    let db = OpenDatabase::create(&path).unwrap();
    (dir, path, db)
}

fn field(name: &str, t: FieldType) -> FieldDef {
    FieldDef::new(name, t)
}

fn auto_id() -> FieldDef {
    let mut f = field("id", FieldType::AutoNumber);
    f.primary_key = true;
    f
}

fn values(v: Value) -> RecordValues {
    v.as_object().unwrap().clone()
}

fn customers_def() -> TableDef {
    let mut name = field("name", FieldType::ShortText { max_length: Some(50) });
    name.required = true;
    name.description = Some("Customer name".into());
    let mut active = field("active", FieldType::YesNo);
    active.default = Some(DefaultValue::Literal(json!(true)));
    TableDef {
        name: "Customers".into(),
        fields: vec![
            auto_id(),
            name,
            field("balance", FieldType::Currency { scale: 2 }),
            active,
            field("joined", FieldType::DateTime),
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        checks: vec![],
        description: Some("People who buy things".into()),
    }
}

fn all_rows(db: &OpenDatabase, table: &str) -> RecordPage {
    records::list_records(&db.conn, table, &RecordQuery::default()).unwrap()
}

#[test]
fn create_close_reopen_persists_everything() {
    let (_dir, path, db) = temp_db();
    migrations::create_table(&db.conn, &customers_def()).unwrap();
    let key = records::insert_record(
        &db.conn,
        "Customers",
        &values(json!({"name": "Ada", "balance": "12.34", "joined": "2024-05-01T10:00:00Z"})),
    )
    .unwrap();
    assert_eq!(key, RecordKey::RowId(1));
    db.close().unwrap();

    let db = OpenDatabase::open(&path).unwrap();
    let objects = schema::list_objects(&db.conn).unwrap();
    assert_eq!(objects.len(), 1, "metadata tables must be hidden: {objects:?}");
    assert!(objects[0].managed);
    let info = schema::table_info(&db.conn, "customers").unwrap();
    assert!(info.managed);
    assert_eq!(info.definition, customers_def());
    let page = all_rows(&db, "Customers");
    assert_eq!(page.columns, ["id", "name", "balance", "active", "joined"]);
    assert_eq!(page.rows[0].values, vec![json!(1), json!("Ada"), json!(1234), json!(1), json!("2024-05-01T10:00:00Z")]);
    let meta = db.info().unwrap();
    assert!(meta.foreign_keys);
    assert_eq!(meta.table_count, 1);
}

#[test]
fn never_overwrites_and_rejects_bad_files() {
    let (dir, path, db) = temp_db();
    drop(db);
    assert!(matches!(OpenDatabase::create(&path), Err(AppError::AlreadyExists(_))));

    let fake = dir.path().join("fake.db");
    std::fs::write(&fake, b"definitely not sqlite, just some text").unwrap();
    assert!(matches!(OpenDatabase::open(fake.to_str().unwrap()), Err(AppError::InvalidDatabase(_))));
    assert_eq!(std::fs::read(&fake).unwrap(), b"definitely not sqlite, just some text");

    let missing = dir.path().join("missing.db");
    assert!(matches!(OpenDatabase::open(missing.to_str().unwrap()), Err(AppError::InvalidPath(_))));
    assert!(matches!(OpenDatabase::create("relative.db"), Err(AppError::InvalidPath(_))));
}

#[test]
fn all_field_types_round_trip() {
    let (_dir, _path, db) = temp_db();
    migrations::create_table(&db.conn, &customers_def()).unwrap();
    let mut lookup = field("customer", FieldType::Lookup { value_type: LookupValueType::Integer, display_column: Some("name".into()) });
    lookup.required = true;
    let def = TableDef {
        name: "Order Items".into(),
        fields: vec![
            auto_id(),
            lookup,
            field("notes", FieldType::LongText),
            field("qty", FieldType::Number { real: false }),
            field("weight", FieldType::Number { real: true }),
            field("big", FieldType::LargeNumber),
            field("price", FieldType::Currency { scale: 2 }),
            field("total", FieldType::Calculated { expression: "qty * price".into(), stored: false, result: CalculatedResult::Integer }),
            field("shipped", FieldType::YesNo),
            field("photo", FieldType::Attachment),
            field("link", FieldType::Hyperlink),
            field("at", FieldType::DateTime),
        ],
        foreign_keys: vec![ForeignKeyDef {
            columns: vec!["customer".into()],
            ref_table: "customers".into(),
            ref_columns: vec![],
            on_delete: FkAction::Cascade,
            on_update: FkAction::NoAction,
        }],
        unique_constraints: vec![],
        checks: vec!["qty >= 0".into()],
        description: None,
    };
    let created = migrations::create_table(&db.conn, &def).unwrap();
    assert_eq!(created.foreign_keys[0].ref_table, "Customers");
    assert_eq!(created.foreign_keys[0].ref_columns, ["id"]);

    let cust = records::insert_record(&db.conn, "Customers", &values(json!({"name": "Bob"}))).unwrap();
    let RecordKey::RowId(cust_id) = cust else { panic!() };
    let key = records::insert_record(
        &db.conn,
        "Order Items",
        &values(json!({
            "customer": cust_id, "notes": "fragile", "qty": "3", "weight": 1.5, "big": 9007199254740993i64,
            "price": "2.50", "shipped": "yes", "photo": {"base64": "AAEC"}, "link": "https://example.com",
            "at": "2024-01-02"
        })),
    )
    .unwrap();
    let row = records::get_record(&db.conn, "Order Items", &key).unwrap();
    assert_eq!(
        row.values,
        vec![
            json!(1), json!(cust_id), json!("fragile"), json!(3), json!(1.5), json!(9007199254740993i64),
            json!(250), json!(750), json!(1), json!({"blob": {"size": 3}}), json!("https://example.com"), json!("2024-01-02")
        ]
    );
    assert_eq!(
        records::get_attachment(&db.conn, "Order Items", &key, "photo").unwrap(),
        json!({"base64": "AAEC"})
    );
    // Calculated fields are read-only, CHECK constraints are enforced.
    let err = records::update_record(&db.conn, "Order Items", &key, &values(json!({"total": 1}))).unwrap_err();
    assert!(matches!(err, AppError::Validation(_)), "{err}");
    let err = records::update_record(&db.conn, "Order Items", &key, &values(json!({"qty": -1}))).unwrap_err();
    assert!(matches!(err, AppError::Constraint(_)), "{err}");
    // Foreign keys are enforced.
    let err = records::insert_record(&db.conn, "Order Items", &values(json!({"customer": 999}))).unwrap_err();
    assert!(matches!(err, AppError::Constraint(_)), "{err}");
    // Cascade delete.
    records::delete_records(&db.conn, "Customers", &[cust], true).unwrap();
    assert_eq!(all_rows(&db, "Order Items").total, 0);
}

#[test]
fn crud_and_meaningful_errors() {
    let (_dir, _path, db) = temp_db();
    migrations::create_table(&db.conn, &customers_def()).unwrap();
    let c = &db.conn;

    let k = records::insert_record(c, "Customers", &values(json!({"name": "Ada"}))).unwrap();
    let row = records::get_record(c, "Customers", &k).unwrap();
    assert_eq!(row.values[3], json!(1), "default applied");

    records::update_record(c, "Customers", &k, &values(json!({"name": "Ada L.", "balance": "-3.5"}))).unwrap();
    let row = records::get_record(c, "Customers", &k).unwrap();
    assert_eq!(row.values[1], json!("Ada L."));
    assert_eq!(row.values[2], json!(-350));

    let cases: Vec<(RecordValues, &str)> = vec![
        (values(json!({})), "validation"),
        (values(json!({"name": "x", "nope": 1})), "validation"),
        (values(json!({"name": "x", "joined": "yesterday"})), "validation"),
        (values(json!({"name": "x", "balance": "1.234"})), "validation"),
        (values(json!({"name": "x".repeat(51)})), "validation"),
        (values(json!({"name": "x", "active": 5})), "validation"),
    ];
    for (v, kind) in cases {
        let err = records::insert_record(c, "Customers", &v).unwrap_err();
        assert_eq!(err.kind(), kind, "{v:?}: {err}");
    }
    let err = records::update_record(c, "Customers", &k, &values(json!({"name": null}))).unwrap_err();
    assert_eq!(err.kind(), "validation");
    let err = records::update_record(c, "Customers", &RecordKey::RowId(42), &values(json!({"name": "z"}))).unwrap_err();
    assert_eq!(err.kind(), "not_found");
    let err = records::list_records(c, "Nope", &RecordQuery::default()).unwrap_err();
    assert_eq!(err.kind(), "not_found");

    let err = records::delete_records(c, "Customers", std::slice::from_ref(&k), false).unwrap_err();
    assert_eq!(err.kind(), "confirmation_required");
    assert_eq!(records::delete_records(c, "Customers", std::slice::from_ref(&k), true).unwrap(), 1);
    assert_eq!(all_rows(&db, "Customers").total, 0);
    let err = records::delete_records(c, "Customers", &[k], true).unwrap_err();
    assert_eq!(err.kind(), "not_found");
}

#[test]
fn pagination_sorting_filtering_search() {
    let (_dir, _path, db) = temp_db();
    migrations::create_table(&db.conn, &customers_def()).unwrap();
    let ops: Vec<BatchOp> = (1..=25)
        .map(|i| BatchOp::Insert {
            values: values(json!({"name": format!("cust_{i:02}"), "balance": i * 100, "active": i % 2 == 0})),
        })
        .collect();
    let res = records::batch(&db.conn, "Customers", &ops, false).unwrap();
    assert_eq!(res.inserted.len(), 25);

    let q = RecordQuery {
        offset: 5,
        limit: 10,
        sort: vec![SortSpec { column: "balance".into(), descending: true }],
        ..Default::default()
    };
    let page = records::list_records(&db.conn, "Customers", &q).unwrap();
    assert_eq!(page.total, 25);
    assert_eq!(page.rows.len(), 10);
    assert_eq!(page.rows[0].values[1], json!("cust_20"));

    let q = RecordQuery {
        filters: vec![
            FilterSpec { column: "active".into(), op: FilterOp::Eq, value: json!("yes") },
            FilterSpec { column: "balance".into(), op: FilterOp::Ge, value: json!("10.00") },
        ],
        ..Default::default()
    };
    assert_eq!(records::list_records(&db.conn, "Customers", &q).unwrap().total, 8);

    // LIKE wildcards in the search term are literal.
    let q = RecordQuery { search: Some("_1".into()), ..Default::default() };
    assert_eq!(records::list_records(&db.conn, "Customers", &q).unwrap().total, 10);
    let q = RecordQuery { search: Some("%".into()), ..Default::default() };
    assert_eq!(records::list_records(&db.conn, "Customers", &q).unwrap().total, 0);

    let q = RecordQuery {
        filters: vec![FilterSpec { column: "name".into(), op: FilterOp::In, value: json!(["cust_01", "cust_02"]) }],
        ..Default::default()
    };
    assert_eq!(records::list_records(&db.conn, "Customers", &q).unwrap().total, 2);

    let q = RecordQuery { limit: 1_000_000, ..Default::default() };
    assert_eq!(records::list_records(&db.conn, "Customers", &q).unwrap().limit, records::MAX_PAGE_SIZE);
    let q = RecordQuery { sort: vec![SortSpec { column: "x; DROP TABLE Customers".into(), descending: false }], ..Default::default() };
    assert_eq!(records::list_records(&db.conn, "Customers", &q).unwrap_err().kind(), "validation");
}

#[test]
fn batch_is_atomic() {
    let (_dir, _path, db) = temp_db();
    migrations::create_table(&db.conn, &customers_def()).unwrap();
    let ops = vec![
        BatchOp::Insert { values: values(json!({"name": "ok"})) },
        BatchOp::Insert { values: values(json!({"name": null})) },
    ];
    let err = records::batch(&db.conn, "Customers", &ops, false).unwrap_err();
    assert!(err.to_string().contains("operation #2"), "{err}");
    assert_eq!(all_rows(&db, "Customers").total, 0);
}

#[test]
fn add_modify_rename_drop_fields() {
    let (_dir, path, db) = temp_db();
    migrations::create_table(&db.conn, &customers_def()).unwrap();
    let c = &db.conn;
    for (n, b) in [("A", 150), ("B", 275), ("C", 300)] {
        records::insert_record(c, "Customers", &values(json!({"name": n, "balance": b}))).unwrap();
    }
    records::delete_records(c, "Customers", &[RecordKey::RowId(3)], true).unwrap();
    c.execute_batch("CREATE INDEX idx_name ON Customers(name)").unwrap();

    // Adding a required field without default to a non-empty table is refused.
    let mut email = field("email", FieldType::ShortText { max_length: None });
    email.required = true;
    assert_eq!(migrations::add_field(c, "Customers", email.clone(), None).unwrap_err().kind(), "validation");
    email.default = Some(DefaultValue::Literal(json!("")));
    migrations::add_field(c, "Customers", email, Some(2)).unwrap();
    let def = schema::load_table(c, "Customers").unwrap().definition;
    assert_eq!(def.fields[2].name, "email");

    // Rename keeps data and indexes.
    migrations::rename_field(c, "Customers", "name", "full name").unwrap();
    let info = schema::table_info(c, "Customers").unwrap();
    assert!(info.managed);
    assert_eq!(info.definition.fields[1].name, "full name");
    assert!(info.indexes.iter().any(|i| i.name == "idx_name" && i.columns == ["full name"]));

    // Currency -> Number(real) converts minor units, and back.
    let mut bal = field("balance", FieldType::Number { real: true });
    let res = migrations::modify_field(c, "Customers", "balance", bal.clone()).unwrap();
    assert!(res.rebuilt);
    let page = all_rows(&db, "Customers");
    assert_eq!(page.rows[0].values[3], json!(1.5));
    bal.field_type = FieldType::Currency { scale: 2 };
    migrations::modify_field(c, "Customers", "balance", bal).unwrap();
    assert_eq!(all_rows(&db, "Customers").rows[1].values[3], json!(275));


    // Index survived the rebuilds; AutoNumber does not reuse deleted ids.
    assert!(schema::table_info(c, "Customers").unwrap().indexes.iter().any(|i| i.name == "idx_name"));
    let k = records::insert_record(c, "Customers", &values(json!({"full name": "D"}))).unwrap();
    assert_eq!(k, RecordKey::RowId(4));

    // A change that existing data violates is rolled back intact.
    let short = field("full name", FieldType::ShortText { max_length: Some(0) });
    assert_eq!(migrations::modify_field(c, "Customers", "full name", short).unwrap_err().kind(), "validation");
    let mut tiny = field("email", FieldType::YesNo);
    tiny.required = false;
    records::update_record(c, "Customers", &RecordKey::RowId(1), &values(json!({"email": "a@b.c"}))).unwrap();
    let err = migrations::modify_field(c, "Customers", "email", tiny).unwrap_err();
    assert_eq!(err.kind(), "constraint", "{err}");
    let def_after = schema::load_table(c, "Customers").unwrap().definition;
    assert!(matches!(def_after.field("email").unwrap().field_type, FieldType::ShortText { .. }));
    assert_eq!(all_rows(&db, "Customers").total, 3);

    // Text -> Currency reads numeric text in major units; other text blocks the change.
    let note = field("note", FieldType::ShortText { max_length: None });
    migrations::add_field(c, "Customers", note.clone(), None).unwrap();
    let keys: Vec<_> = all_rows(&db, "Customers").rows.into_iter().map(|r| r.key.unwrap()).take(3).collect();
    for (key, text) in keys.iter().zip([" 12 ", "-3.5", "1.2.3"]) {
        records::update_record(c, "Customers", key, &values(json!({"note": text}))).unwrap();
    }
    let mut money = note;
    money.field_type = FieldType::Currency { scale: 2 };
    assert_eq!(migrations::modify_field(c, "Customers", "note", money.clone()).unwrap_err().kind(), "constraint");
    records::update_record(c, "Customers", &keys[2], &values(json!({"note": ".25"}))).unwrap();
    migrations::modify_field(c, "Customers", "note", money).unwrap();
    let notes: Vec<_> = all_rows(&db, "Customers").rows.iter().map(|r| r.values.last().unwrap().clone()).collect();
    assert_eq!(notes[..3], [json!(1200), json!(-350), json!(25)]);
    migrations::drop_field(c, "Customers", "note", true).unwrap();

    // Dropping a field needs confirmation, and removes its index with a warning.
    assert_eq!(migrations::drop_field(c, "Customers", "full name", false).unwrap_err().kind(), "confirmation_required");
    let res = migrations::drop_field(c, "Customers", "full name", true).unwrap();
    assert!(res.warnings.iter().any(|w| w.contains("idx_name")), "{:?}", res.warnings);
    db.close().unwrap();

    let db = OpenDatabase::open(&path).unwrap();
    let info = schema::table_info(&db.conn, "Customers").unwrap();
    assert!(info.managed);
    assert!(info.definition.field("full name").is_none());
    assert_eq!(info.row_count, 3);
}

#[test]
fn relationships_survive_renames_and_block_drops() {
    let (_dir, _path, db) = temp_db();
    let c = &db.conn;
    migrations::create_table(c, &customers_def()).unwrap();
    let orders = TableDef {
        name: "Orders".into(),
        fields: vec![auto_id(), field("customer_id", FieldType::Lookup { value_type: LookupValueType::Integer, display_column: Some("name".into()) })],
        foreign_keys: vec![ForeignKeyDef {
            columns: vec!["customer_id".into()],
            ref_table: "Customers".into(),
            ref_columns: vec!["id".into()],
            on_delete: FkAction::Restrict,
            on_update: FkAction::Cascade,
        }],
        unique_constraints: vec![],
        checks: vec![],
        description: None,
    };
    migrations::create_table(c, &orders).unwrap();
    let cust = records::insert_record(c, "Customers", &values(json!({"name": "A"}))).unwrap();
    records::insert_record(c, "Orders", &values(json!({"customer_id": 1}))).unwrap();

    assert_eq!(migrations::drop_table(c, "Customers", true).unwrap_err().kind(), "constraint");
    assert_eq!(records::delete_records(c, "Customers", &[cust], true).unwrap_err().kind(), "constraint");

    migrations::rename_table(c, "Customers", "Clients").unwrap();
    migrations::rename_field(c, "Clients", "name", "label").unwrap();
    let info = schema::table_info(c, "Orders").unwrap();
    assert!(info.managed, "child metadata must follow renames");
    assert_eq!(info.definition.foreign_keys[0].ref_table, "Clients");
    assert!(matches!(&info.definition.fields[1].field_type, FieldType::Lookup { display_column: Some(d), .. } if d == "label"));
    assert_eq!(schema::load_table(c, "Clients").unwrap().definition.fields[1].name, "label");

    // Rebuilding the parent keeps children valid.
    migrations::add_field(c, "Clients", field("vip", FieldType::YesNo), None).unwrap();
    assert_eq!(records::insert_record(c, "Orders", &values(json!({"customer_id": 77}))).unwrap_err().kind(), "constraint");

    // FK targets must exist and be unique.
    let mut bad = orders.clone();
    bad.name = "Bad".into();
    bad.foreign_keys[0].ref_table = "Clients".into();
    bad.foreign_keys[0].ref_columns = vec!["label".into()];
    assert_eq!(migrations::create_table(c, &bad).unwrap_err().kind(), "validation");
    bad.foreign_keys[0].ref_table = "Missing".into();
    assert_eq!(migrations::create_table(c, &bad).unwrap_err().kind(), "validation");

    migrations::drop_table(c, "Orders", true).unwrap();
    migrations::drop_table(c, "Clients", true).unwrap();
    assert!(schema::list_objects(c).unwrap().is_empty());
}

#[test]
fn unmanaged_tables_and_views() {
    let (_dir, _path, db) = temp_db();
    let c = &db.conn;
    c.execute_batch(
        "CREATE TABLE plain (id INTEGER PRIMARY KEY, title TEXT NOT NULL DEFAULT 'x', score REAL, UNIQUE(title));
         CREATE TABLE checked (a INTEGER CHECK (a > 0));
         INSERT INTO plain (title, score) VALUES ('one', 1.5), ('two', 2.5);
         CREATE VIEW high AS SELECT title FROM plain WHERE score > 2;",
    )
    .unwrap();
    let info = schema::table_info(c, "plain").unwrap();
    assert!(!info.managed);
    assert!(info.unsupported_features.is_empty());
    assert!(matches!(info.definition.fields[0].field_type, FieldType::AutoNumber));
    assert!(info.definition.fields[1].unique);

    // Views are listed and readable but not writable.
    let page = records::list_records(c, "high", &RecordQuery::default()).unwrap();
    assert_eq!(page.rows.len(), 1);
    assert!(page.rows[0].key.is_none());
    assert_eq!(records::insert_record(c, "high", &values(json!({"title": "z"}))).unwrap_err().kind(), "unsupported");

    // A rebuild that would break a view is refused and rolled back.
    let err = migrations::drop_field(c, "plain", "title", true).unwrap_err();
    assert_eq!(err.kind(), "constraint", "{err}");
    assert_eq!(records::list_records(c, "high", &RecordQuery::default()).unwrap().rows.len(), 1);

    // Unmanaged tables become managed once saved through the designer.
    migrations::add_field(c, "plain", field("extra", FieldType::LongText), None).unwrap();
    assert!(schema::table_info(c, "plain").unwrap().managed);
    assert_eq!(all_rows(&db, "plain").total, 2);

    // CHECK constraints cannot be inferred, so rebuilds are refused.
    let err = migrations::add_field(c, "checked", field("b", FieldType::LongText), None).unwrap_err();
    assert_eq!(err.kind(), "unsupported");
    migrations::rename_field(c, "checked", "a", "aa").unwrap();
}

#[test]
fn identifiers_are_quoted_and_validated() {
    let (_dir, _path, db) = temp_db();
    let c = &db.conn;
    let evil = "x\"; DROP TABLE y; --";
    let def = TableDef {
        name: evil.into(),
        fields: vec![field("a\"b", FieldType::LongText)],
        foreign_keys: vec![],
        unique_constraints: vec![],
        checks: vec![],
        description: None,
    };
    migrations::create_table(c, &def).unwrap();
    records::insert_record(c, evil, &values(json!({"a\"b": "'; DELETE FROM x; --"}))).unwrap();
    assert_eq!(all_rows(&db, evil).rows[0].values[0], json!("'; DELETE FROM x; --"));

    let mut bad = def.clone();
    bad.name = "sqlite_evil".into();
    assert_eq!(migrations::create_table(c, &bad).unwrap_err().kind(), "invalid_identifier");
    bad.name = "dup".into();
    bad.fields.push(field("A\"B", FieldType::LongText));
    assert_eq!(migrations::create_table(c, &bad).unwrap_err().kind(), "validation");
    bad.fields.pop();
    bad.checks = vec!["1) ; DROP TABLE t; --".into()];
    assert_eq!(migrations::create_table(c, &bad).unwrap_err().kind(), "validation");
    assert_eq!(migrations::create_table(c, &def).unwrap_err().kind(), "already_exists");
    assert_eq!(migrations::rename_table(c, evil, "__app_tables").unwrap_err().kind(), "invalid_identifier");
}

#[test]
fn diagram_layout_round_trips_and_rejects_bad_positions() {
    let (_dir, path, db) = temp_db();
    assert!(metadata::load_layout(&db.conn).unwrap().is_empty());
    let mut layout = DiagramLayout::new();
    layout.insert("Customers".into(), TablePosition { x: 40.0, y: -12.5 });
    metadata::save_layout(&db.conn, &layout).unwrap();
    drop(db);

    let db = OpenDatabase::open(&path).unwrap();
    assert_eq!(metadata::load_layout(&db.conn).unwrap(), layout);
    // The layout row is not a user table.
    assert!(schema::list_objects(&db.conn).unwrap().is_empty());

    layout.insert("Orders".into(), TablePosition { x: f64::INFINITY, y: 0.0 });
    assert!(matches!(metadata::save_layout(&db.conn, &layout), Err(AppError::Validation(_))));
}

/// The Relationships screen links tables by saving the child's design with a
/// foreign key, turning the linking number field into a lookup.
#[test]
fn linking_an_existing_field_keeps_data_and_enforces_it() {
    let (_dir, _path, db) = temp_db();
    let categories = TableDef {
        name: "Categories".into(),
        fields: vec![auto_id(), field("name", FieldType::ShortText { max_length: None })],
        foreign_keys: vec![],
        unique_constraints: vec![],
        checks: vec![],
        description: None,
    };
    let mut products = TableDef {
        name: "Products".into(),
        fields: vec![auto_id(), field("category_id", FieldType::Number { real: false })],
        ..categories.clone()
    };
    migrations::create_table(&db.conn, &categories).unwrap();
    migrations::create_table(&db.conn, &products).unwrap();
    records::insert_record(&db.conn, "Categories", &values(json!({"name": "Tea"}))).unwrap();
    records::insert_record(&db.conn, "Products", &values(json!({"category_id": 1}))).unwrap();

    products.fields[1].field_type = FieldType::Lookup {
        value_type: LookupValueType::Integer,
        display_column: Some("name".into()),
    };
    products.foreign_keys.push(ForeignKeyDef {
        columns: vec!["category_id".into()],
        ref_table: "Categories".into(),
        ref_columns: vec!["id".into()],
        on_delete: FkAction::Restrict,
        on_update: FkAction::Cascade,
    });
    let design = TableDesign { definition: products.clone(), renames: vec![] };
    migrations::save_table_design(&db.conn, "Products", &design, false).unwrap();
    assert_eq!(all_rows(&db, "Products").rows[0].values[1], json!(1));
    let orphan = records::insert_record(&db.conn, "Products", &values(json!({"category_id": 99})));
    assert!(matches!(orphan, Err(AppError::Constraint(_))), "{orphan:?}");

    // Existing rows that would break a new link make the save fail untouched.
    let mut orders = TableDef { name: "Orders".into(), ..products.clone() };
    orders.fields[1].field_type = FieldType::Number { real: false };
    orders.foreign_keys.clear();
    migrations::create_table(&db.conn, &orders).unwrap();
    records::insert_record(&db.conn, "Orders", &values(json!({"category_id": 7}))).unwrap();
    orders.foreign_keys = products.foreign_keys.clone();
    let design = TableDesign { definition: orders, renames: vec![] };
    assert!(migrations::save_table_design(&db.conn, "Orders", &design, false).is_err());
    assert_eq!(all_rows(&db, "Orders").rows.len(), 1);
}

#[test]
fn queries_read_only_and_save_as_views() {
    let (_dir, _path, db) = temp_db();
    migrations::create_table(&db.conn, &customers_def()).unwrap();
    for n in ["Ann", "Bob"] {
        records::insert_record(&db.conn, "Customers", &values(json!({"name": n}))).unwrap();
    }

    let r = queries::run_query(&db.conn, "SELECT name FROM Customers ORDER BY name;").unwrap();
    assert_eq!(r.columns, vec!["name"]);
    assert_eq!(r.rows, vec![vec![json!("Ann")], vec![json!("Bob")]]);
    assert!(!r.truncated);

    let many = queries::run_query(
        &db.conn,
        "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c) SELECT x FROM c",
    )
    .unwrap();
    assert_eq!(many.rows.len(), queries::MAX_QUERY_ROWS);
    assert!(many.truncated);

    for bad in ["DELETE FROM Customers", "SELECT 1; DROP TABLE Customers", "PRAGMA foreign_keys = OFF"] {
        assert!(queries::run_query(&db.conn, bad).is_err(), "{bad}");
    }
    assert_eq!(all_rows(&db, "Customers").rows.len(), 2);

    // Named parameters are bound from the map; missing ones are NULL.
    let params = json!({"who": "Bob"}).as_object().unwrap().clone();
    let r = queries::run_query_with(&db.conn, "SELECT name FROM Customers WHERE name = :who", &params).unwrap();
    assert_eq!(r.rows, vec![vec![json!("Bob")]]);
    let r = queries::run_query_with(
        &db.conn,
        "SELECT name FROM Customers WHERE :missing IS NULL OR name = :missing ORDER BY name",
        &params,
    )
    .unwrap();
    assert_eq!(r.rows.len(), 2);

    queries::save_view(&db.conn, "Names", "SELECT name FROM Customers", None, false).unwrap();
    assert_eq!(queries::view_sql(&db.conn, "Names").unwrap(), "SELECT name FROM Customers");
    assert!(matches!(
        queries::save_view(&db.conn, "names", "SELECT 1", None, false),
        Err(AppError::ConfirmationRequired(_))
    ));
    assert!(matches!(
        queries::save_view(&db.conn, "Customers", "SELECT 1", None, true),
        Err(AppError::AlreadyExists(_))
    ));
    assert!(queries::save_view(&db.conn, "Broken", "SELECT nope FROM Customers", None, false).is_err());
    assert!(schema::find_object(&db.conn, "Broken").unwrap().is_none());

    queries::save_view(&db.conn, "Names", "SELECT name AS who FROM Customers", None, true).unwrap();
    assert_eq!(all_rows(&db, "Names").columns, vec!["who"]);

    assert!(matches!(queries::drop_view(&db.conn, "Names", false), Err(AppError::ConfirmationRequired(_))));
    assert!(queries::drop_view(&db.conn, "Customers", true).is_err());
    queries::drop_view(&db.conn, "Names", true).unwrap();
    assert!(schema::find_object(&db.conn, "Names").unwrap().is_none());
}

#[test]
fn saved_queries_keep_their_visual_design() {
    let (_dir, _path, db) = temp_db();
    migrations::create_table(&db.conn, &customers_def()).unwrap();
    let design = json!({"from": "Customers", "joins": [], "columns": []});

    queries::save_view(&db.conn, "All", "SELECT * FROM \"Customers\";", Some(&design), false).unwrap();
    let q = queries::saved_query(&db.conn, "all").unwrap();
    assert_eq!(q.sql, "SELECT * FROM \"Customers\"");
    assert_eq!(q.design, Some(design.clone()));
    let names: Vec<_> = schema::list_objects(&db.conn).unwrap().into_iter().map(|o| o.name).collect();
    assert_eq!(names, vec!["Customers", "All"], "the design store stays hidden");

    // A bad design rolls the whole save back.
    assert!(queries::save_view(&db.conn, "Bad", "SELECT 1", Some(&json!([1])), false).is_err());
    assert!(schema::find_object(&db.conn, "Bad").unwrap().is_none());

    // Replacing it from the SQL editor forgets the design.
    queries::save_view(&db.conn, "All", "SELECT name FROM Customers", None, true).unwrap();
    assert_eq!(queries::saved_query(&db.conn, "All").unwrap().design, None);

    // A view whose SQL no longer matches the design opens as SQL only.
    queries::save_view(&db.conn, "All", "SELECT * FROM Customers", Some(&design), true).unwrap();
    db.conn.execute_batch("DROP VIEW \"All\"; CREATE VIEW \"All\" AS SELECT 1").unwrap();
    assert_eq!(queries::saved_query(&db.conn, "All").unwrap().design, None);

    // Deleting the query deletes its design.
    queries::save_view(&db.conn, "All", "SELECT * FROM Customers", Some(&design), true).unwrap();
    queries::drop_view(&db.conn, "All", true).unwrap();
    queries::save_view(&db.conn, "All", "SELECT * FROM Customers", None, false).unwrap();
    assert_eq!(queries::saved_query(&db.conn, "All").unwrap().design, None);
}

#[test]
fn values_match_as_shown_and_views_keep_field_types() {
    let (_dir, _path, db) = temp_db();
    let c = &db.conn;
    migrations::create_table(c, &customers_def()).unwrap();
    records::insert_record(c, "Customers", &values(json!({"name": "Ann", "balance": "12.50", "active": true}))).unwrap();
    records::insert_record(c, "Customers", &values(json!({"name": "Bob", "balance": "3", "active": false}))).unwrap();
    let search = |table: &str, term: &str| {
        let q = RecordQuery { search: Some(term.into()), ..Default::default() };
        records::list_records(c, table, &q).unwrap().total
    };
    let contains = |table: &str, column: &str, term: &str| {
        let q = RecordQuery {
            filters: vec![FilterSpec { column: column.into(), op: FilterOp::Contains, value: json!(term) }],
            ..Default::default()
        };
        records::list_records(c, table, &q).unwrap().total
    };

    // Currency and Yes/No match the text the user sees, not the stored value.
    assert_eq!(search("Customers", "12.50"), 1);
    assert_eq!(search("Customers", "1250"), 0);
    assert_eq!(contains("Customers", "balance", "3.00"), 1);
    assert_eq!(contains("Customers", "active", "yes"), 1);

    // Views know the field types behind their columns.
    queries::save_view(c, "Rich", "SELECT name, balance AS money, count(*) AS n FROM Customers GROUP BY name", None, false)
        .unwrap();
    let page = all_rows(&db, "Rich");
    assert_eq!(page.types[1], Some(FieldType::Currency { scale: 2 }));
    assert_eq!(page.types[2], None);
    assert_eq!(contains("Rich", "money", "12.5"), 1);
    let q = RecordQuery {
        filters: vec![FilterSpec { column: "money".into(), op: FilterOp::Eq, value: json!("12.50") }],
        ..Default::default()
    };
    assert_eq!(records::list_records(c, "Rich", &q).unwrap().total, 1);

    let r = queries::run_query(c, "SELECT active, money FROM Customers JOIN Rich USING (name)").unwrap();
    assert_eq!(r.types, vec![Some(FieldType::YesNo), Some(FieldType::Currency { scale: 2 })]);
}

#[test]
fn aggregates_group_total_and_filter() {
    let (_dir, _path, db) = temp_db();
    let c = &db.conn;
    migrations::create_table(c, &customers_def()).unwrap();
    for (name, balance, active, joined) in [
        ("Ann", "12.50", true, "2024-01-05"),
        ("Bob", "3", false, "2024-01-20"),
        ("Cy", "10.01", true, "2024-03-02T10:00:00"),
    ] {
        let v = values(json!({"name": name, "balance": balance, "active": active, "joined": joined}));
        records::insert_record(c, "Customers", &v).unwrap();
    }
    let series = |column: Option<&str>, agg| SeriesSpec { column: column.map(String::from), agg };
    let query = |group_by, series, filters| AggregateQuery {
        group_by,
        series,
        filters,
        search: None,
        order: AggregateOrder::Group,
        limit: 0,
    };

    // Ungrouped: one row; money sums and averages stay whole minor units.
    let q = query(
        None,
        vec![
            series(None, Aggregate::Count),
            series(Some("balance"), Aggregate::Sum),
            series(Some("balance"), Aggregate::Avg),
        ],
        vec![],
    );
    let r = records::aggregate(c, "Customers", &q).unwrap();
    assert_eq!(r.rows, vec![vec![json!(3), json!(2551), json!(850)]]);
    assert_eq!(r.types, vec![None, Some(FieldType::Currency { scale: 2 }), Some(FieldType::Currency { scale: 2 })]);
    assert_eq!(r.columns, ["Count", "Sum of balance", "Average balance"]);

    // Grouped by month, filtered with typed values.
    let by_month = Some(GroupBy { column: "joined".into(), bucket: Some(DateBucket::Month) });
    let active = vec![FilterSpec { column: "active".into(), op: FilterOp::Eq, value: json!(true) }];
    let r = records::aggregate(c, "Customers", &query(by_month.clone(), vec![series(None, Aggregate::Count)], active))
        .unwrap();
    assert_eq!(r.rows, vec![vec![json!("2024-01"), json!(1)], vec![json!("2024-03"), json!(1)]]);

    // Ordered by value, limited.
    let mut q = query(
        Some(GroupBy { column: "name".into(), bucket: None }),
        vec![series(Some("balance"), Aggregate::Max)],
        vec![],
    );
    q.order = AggregateOrder::ValueDesc;
    q.limit = 2;
    let r = records::aggregate(c, "Customers", &q).unwrap();
    assert_eq!(r.rows, vec![vec![json!("Ann"), json!(1250)], vec![json!("Cy"), json!(1001)]]);
    assert!(r.truncated);
    assert_eq!(r.types[0], Some(FieldType::ShortText { max_length: Some(50) }));

    // Names are checked against the table; no SQL gets through.
    let bad = query(None, vec![series(Some("balance) FROM x; --"), Aggregate::Sum)], vec![]);
    assert!(matches!(records::aggregate(c, "Customers", &bad), Err(AppError::Validation(_))));
    assert!(records::aggregate(c, "Customers", &query(None, vec![], vec![])).is_err());
    assert!(records::aggregate(c, "Customers", &query(None, vec![series(None, Aggregate::Sum)], vec![])).is_err());

    // Views work too.
    queries::save_view(c, "Actives", "SELECT name, balance FROM Customers WHERE active", None, false).unwrap();
    let r = records::aggregate(c, "Actives", &query(None, vec![series(Some("balance"), Aggregate::Sum)], vec![]))
        .unwrap();
    assert_eq!(r.rows, vec![vec![json!(2251)]]);
    assert_eq!(r.types, vec![Some(FieldType::Currency { scale: 2 })]);
}

#[test]
fn search_ignores_case_in_every_alphabet() {
    let (_dir, _path, db) = temp_db();
    let c = &db.conn;
    migrations::create_table(c, &customers_def()).unwrap();
    for name in ["Élodie", "ÖZGÜR", "Ангелина", "bob"] {
        records::insert_record(c, "Customers", &values(json!({ "name": name }))).unwrap();
    }
    let count = |term: &str| {
        records::list_records(c, "Customers", &RecordQuery { search: Some(term.into()), ..Default::default() }).unwrap().total
    };
    assert_eq!(count("élo"), 1);
    assert_eq!(count("özgür"), 1);
    assert_eq!(count("АНГЕЛ"), 1);
    assert_eq!(count("BOB"), 1);
    let q = RecordQuery {
        filters: vec![FilterSpec { column: "name".into(), op: FilterOp::StartsWith, value: json!("é") }],
        ..Default::default()
    };
    assert_eq!(records::list_records(c, "Customers", &q).unwrap().total, 1);
}

#[test]
fn sorting_a_relation_follows_the_shown_name() {
    let (_dir, _path, db) = temp_db();
    let c = &db.conn;
    migrations::create_table(c, &customers_def()).unwrap();
    let orders = TableDef {
        name: "Orders".into(),
        fields: vec![auto_id(), field("customer_id", FieldType::Lookup { value_type: LookupValueType::Integer, display_column: Some("name".into()) })],
        foreign_keys: vec![ForeignKeyDef {
            columns: vec!["customer_id".into()],
            ref_table: "Customers".into(),
            ref_columns: vec!["id".into()],
            on_delete: FkAction::Restrict,
            on_update: FkAction::Cascade,
        }],
        unique_constraints: vec![],
        checks: vec![],
        description: None,
    };
    migrations::create_table(c, &orders).unwrap();
    // IDs 1, 2, 3 for Zoe, adam, Mia: by ID the names are out of order.
    for name in ["Zoe", "adam", "Mia"] {
        records::insert_record(c, "Customers", &values(json!({ "name": name }))).unwrap();
    }
    for id in [1, 2, 3] {
        records::insert_record(c, "Orders", &values(json!({ "customer_id": id }))).unwrap();
    }
    let sorted = |descending: bool| -> Vec<Value> {
        let q = RecordQuery { sort: vec![SortSpec { column: "customer_id".into(), descending }], ..Default::default() };
        records::list_records(c, "Orders", &q).unwrap().rows.into_iter().map(|r| r.values[1].clone()).collect()
    };
    // adam (2), Mia (3), Zoe (1), ignoring case.
    assert_eq!(sorted(false), vec![json!(2), json!(3), json!(1)]);
    assert_eq!(sorted(true), vec![json!(1), json!(3), json!(2)]);
}
