//! App packages: an app and its databases written to one file and read back
//! into another workspace under free names.

use app_lib::database::{OpenDatabase, migrations, records, schema};
use app_lib::models::*;
use app_lib::{apps, package, workspace};
use serde_json::json;

fn shop(dir: &std::path::Path, name: &str) {
    let db_dir = workspace::databases_dir(dir).unwrap();
    let path = workspace::new_database_path(&db_dir, name).unwrap();
    let db = OpenDatabase::create(path.to_str().unwrap()).unwrap();
    let mut id = FieldDef::new("ID", FieldType::AutoNumber);
    id.primary_key = true;
    let def = TableDef {
        name: "Products".into(),
        fields: vec![id, FieldDef::new("Name", FieldType::ShortText { max_length: None })],
        foreign_keys: vec![],
        unique_constraints: vec![],
        checks: vec![],
        description: None,
    };
    migrations::create_table(&db.conn, &def).unwrap();
    records::import(&db.conn, "Products", &[json!({"Name": "Pen"}).as_object().unwrap().clone()]).unwrap();
}

#[test]
fn apps_travel_with_their_databases() {
    let from = tempfile::tempdir().unwrap();
    shop(from.path(), "Shop");
    let def = json!({"version": 1, "sources": [{"alias": "shop", "database": "Shop"}, {"alias": "again", "database": "Shop"}], "pages": []});
    apps::create_app(from.path(), "Till", &def).unwrap();
    let bytes = package::write_package(from.path(), "Till").unwrap();

    // Into a workspace that already has a "Shop" database and a "Till" app.
    let to = tempfile::tempdir().unwrap();
    shop(to.path(), "Shop");
    apps::create_app(to.path(), "Till", &json!({"version": 1, "sources": [], "pages": []})).unwrap();
    let got = package::read_package(to.path(), &bytes).unwrap();
    assert_eq!(got.app, "Till 2");
    assert_eq!(got.databases, vec![["Shop".to_string(), "Shop 2".to_string()]]);
    let loaded = apps::load_app(to.path(), "Till 2").unwrap();
    assert_eq!(loaded["sources"][0]["database"], "Shop 2");
    assert_eq!(loaded["sources"][1]["database"], "Shop 2");

    let db_dir = workspace::databases_dir(to.path()).unwrap();
    let db = OpenDatabase::open(workspace::resolve_database(&db_dir, "Shop 2").unwrap().to_str().unwrap()).unwrap();
    assert_eq!(schema::list_objects(&db.conn).unwrap().len(), 1);
    let page = records::list_records(&db.conn, "Products", &RecordQuery::default()).unwrap();
    assert_eq!(page.total, 1);
}

#[test]
fn bad_packages_change_nothing() {
    let to = tempfile::tempdir().unwrap();
    assert!(package::read_package(to.path(), b"not a zip").is_err());

    // A package whose database entry is not SQLite: nothing is left behind.
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    use std::io::Write;
    zip.start_file("manifest.json", o).unwrap();
    zip.write_all(br#"{"format":1,"app":"X","databases":["A","B"]}"#).unwrap();
    zip.start_file("app.baseapp", o).unwrap();
    zip.write_all(br#"{"version":1,"sources":[],"pages":[]}"#).unwrap();
    let from = tempfile::tempdir().unwrap();
    shop(from.path(), "A");
    let good = std::fs::read(workspace::resolve_database(&workspace::databases_dir(from.path()).unwrap(), "A").unwrap()).unwrap();
    zip.start_file("databases/0.db", o).unwrap();
    zip.write_all(&good).unwrap();
    zip.start_file("databases/1.db", o).unwrap();
    zip.write_all(b"garbage").unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    assert!(package::read_package(to.path(), &bytes).is_err());
    let dir = workspace::databases_dir(to.path()).unwrap();
    assert!(std::fs::read_dir(&dir).unwrap().next().is_none(), "left files behind");
    assert!(apps::list_apps(to.path()).unwrap().is_empty());

    // Newer formats are refused.
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("manifest.json", o).unwrap();
    zip.write_all(br#"{"format":99,"app":"X","databases":[]}"#).unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    assert!(package::read_package(to.path(), &bytes).unwrap_err().to_string().contains("newer"));
}
