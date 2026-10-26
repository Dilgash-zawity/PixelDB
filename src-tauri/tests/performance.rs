//! Timings on a large database: 100,000 orders over 2,000 customers. Not run
//! by default (it takes a few seconds); run with
//! `cargo test --test performance -- --ignored --nocapture` and read the times.
//! Each step must stay under its budget, which is generous for slow CI machines.

use std::time::{Duration, Instant};

use app_lib::database::{OpenDatabase, indexes, journal, migrations, queries, records};
use app_lib::models::*;
use serde_json::{Value, json};

const CUSTOMERS: usize = 2_000;
const ORDERS: usize = 100_000;

fn values(v: Value) -> RecordValues {
    v.as_object().unwrap().clone()
}

fn id() -> FieldDef {
    let mut f = FieldDef::new("ID", FieldType::AutoNumber);
    f.primary_key = true;
    f
}

fn timed<T>(label: &str, budget_ms: u64, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    let took = start.elapsed();
    println!("{label:<48} {:>8.1} ms", took.as_secs_f64() * 1000.0);
    assert!(took < Duration::from_millis(budget_ms), "{label} took {took:?}, over {budget_ms} ms");
    out
}

#[test]
#[ignore]
fn large_tables_stay_fast() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = OpenDatabase::create(dir.path().join("big.db").to_str().unwrap()).unwrap();
    let text = || FieldType::ShortText { max_length: None };
    migrations::create_table(&db.conn, &TableDef { name: "Customers".into(), fields: vec![id(), FieldDef::new("Name", text())], foreign_keys: vec![], unique_constraints: vec![], checks: vec![], description: None }).unwrap();
    let fk = ForeignKeyDef { columns: vec!["Customer".into()], ref_table: "Customers".into(), ref_columns: vec![], on_delete: FkAction::Restrict, on_update: FkAction::Cascade };
    migrations::create_table(
        &db.conn,
        &TableDef {
            name: "Orders".into(),
            fields: vec![
                id(),
                FieldDef::new("Item", text()),
                FieldDef::new("Price", FieldType::Currency { scale: 2 }),
                FieldDef::new("Paid", FieldType::YesNo),
                FieldDef::new("Date", FieldType::DateTime),
                FieldDef::new("Customer", FieldType::Lookup { value_type: LookupValueType::Integer, display_column: Some("Name".into()) }),
            ],
            foreign_keys: vec![fk],
            unique_constraints: vec![],
            checks: vec![],
            description: None,
        },
    )
    .unwrap();

    let customers: Vec<_> = (0..CUSTOMERS).map(|i| values(json!({"Name": format!("Customer {i:05}")}))).collect();
    timed("import 2,000 customers", 2_000, || records::import(&db.conn, "Customers", &customers).unwrap());
    let orders: Vec<_> = (0..ORDERS)
        .map(|i| {
            values(json!({
                "Item": format!("Item {}", i % 997),
                "Price": format!("{}.{:02}", i % 500, i % 100),
                "Paid": i % 3 == 0,
                "Date": format!("2024-{:02}-{:02}", i % 12 + 1, i % 28 + 1),
                "Customer": (i % CUSTOMERS) as i64 + 1
            }))
        })
        .collect();
    timed("import 100,000 orders (with undo)", 30_000, || {
        db.recorded("Import", |c| records::import(c, "Orders", &orders)).unwrap()
    });

    let page = |q: RecordQuery| records::list_records(&db.conn, "Orders", &q).unwrap();
    let first = timed("first page", 300, || page(RecordQuery { limit: 100, ..Default::default() }));
    assert_eq!(first.total, ORDERS as i64);
    timed("last page", 500, || page(RecordQuery { limit: 100, offset: ORDERS as u64 - 100, ..Default::default() }));
    timed("search all columns", 3_000, || page(RecordQuery { limit: 100, search: Some("item 99".into()), ..Default::default() }));
    timed("filter price > 400 (no index)", 1_500, || {
        page(RecordQuery { limit: 100, filters: vec![FilterSpec { column: "Price".into(), op: FilterOp::Gt, value: json!("400") }], ..Default::default() })
    });
    timed("sort by price (no index)", 1_500, || page(RecordQuery { limit: 100, sort: vec![SortSpec { column: "Price".into(), descending: true }], ..Default::default() }));
    timed("sort by customer name (relation)", 3_000, || {
        page(RecordQuery { limit: 100, sort: vec![SortSpec { column: "Customer".into(), descending: false }], ..Default::default() })
    });
    timed("add an index on price", 2_000, || indexes::create_index(&db.conn, "Orders", &["Price".into()], false).unwrap());
    timed("sort by price (indexed)", 300, || page(RecordQuery { limit: 100, sort: vec![SortSpec { column: "Price".into(), descending: true }], ..Default::default() }));
    timed("sum per month", 2_000, || {
        records::aggregate(
            &db.conn,
            "Orders",
            &AggregateQuery {
                group_by: Some(GroupBy { column: "Date".into(), bucket: Some(DateBucket::Month) }),
                series: vec![SeriesSpec { column: Some("Price".into()), agg: Aggregate::Sum }],
                filters: vec![],
                search: None,
                order: AggregateOrder::default(),
                limit: 0,
            },
        )
        .unwrap()
    });
    timed("query: join and group", 2_000, || {
        queries::run_query(&db.conn, "SELECT c.Name, count(*) n, sum(o.Price) FROM Orders o JOIN Customers c ON c.ID = o.Customer GROUP BY c.ID ORDER BY n DESC").unwrap()
    });
    timed("export read: every order", 5_000, || records::list_all(&db.conn, "Orders", &RecordQuery::default(), 1_000_000).unwrap());
    let keys: Vec<_> = first.rows.iter().filter_map(|r| r.key.clone()).collect();
    timed("delete 100 records (with undo)", 2_000, || db.recorded("Delete", |c| records::delete_records(c, "Orders", &keys, true)).unwrap());
    timed("undo the delete", 2_000, || journal::undo(&db.conn, &mut db.journal).unwrap());
    timed("structure change: add a field (rebuild)", 10_000, || {
        let mut def = app_lib::database::schema::table_info(&db.conn, "Orders").unwrap().definition;
        def.fields.push(FieldDef::new("Note", FieldType::LongText));
        db.restructured(|c| migrations::save_table_design(c, "Orders", &TableDesign { definition: def, renames: vec![] }, false)).unwrap()
    });
}
