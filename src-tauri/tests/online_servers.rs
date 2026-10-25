//! Runs the online record layer against real servers. Each test is skipped
//! unless its variable is set to the `host:port` of a server holding the
//! database `shop`:
//! - `SLATEDB_TEST_PG`: PostgreSQL, user `postgres` without a password,
//!   schema in `tests/fixtures/online_shop.sql`;
//! - `SLATEDB_TEST_MYSQL`: MySQL or MariaDB, user `root` without a password,
//!   schema in `tests/fixtures/online_shop_mysql.sql`.

use serde_json::{Map, Value as JsonValue, json};

use app_lib::models::{
    Aggregate, AggregateQuery, FieldType, FilterOp, FilterSpec, GroupBy, ObjectKind, RecordKey, RecordQuery,
    SeriesSpec, SortSpec,
};
use app_lib::online::{ConnectionSettings, Engine};
use app_lib::online_data::OnlinePools;
use app_lib::online_records::{self as online, Online};

fn settings(var: &str, engine: Engine) -> Option<ConnectionSettings> {
    let target = std::env::var(var).ok()?;
    let (host, port) = target.split_once(':')?;
    let (provider, user) = match engine {
        Engine::Postgres => ("postgres", "postgres"),
        _ => ("mysql", "root"),
    };
    Some(ConnectionSettings {
        provider: provider.into(),
        engine,
        host: host.into(),
        port: port.parse().ok(),
        database: "shop".into(),
        user: user.into(),
        ssl: false,
    })
}

fn values(pairs: &[(&str, JsonValue)]) -> Map<String, JsonValue> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

#[tokio::test]
async fn postgres_end_to_end() {
    let Some(s) = settings("SLATEDB_TEST_PG", Engine::Postgres) else {
        eprintln!("SLATEDB_TEST_PG not set; skipping");
        return;
    };
    let pools = OnlinePools::default();
    let o = Online::new(&pools, &s, "");

    // Listing: the default schema unqualified, others as schema.table.
    let objects = online::list_objects(&o).await.unwrap();
    let names: Vec<_> = objects.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["big_orders", "customers", "log", "orders", "sales.leads"]);
    assert!(matches!(objects[0].kind, ObjectKind::View));

    // Table definitions with server types mapped.
    let info = online::table_info(&o, "orders").await.unwrap();
    let f = |n: &str| info.definition.fields.iter().find(|x| x.name == n).unwrap().clone();
    assert_eq!(f("id").field_type, FieldType::AutoNumber);
    assert!(f("id").primary_key);
    assert!(matches!(f("customer_id").field_type, FieldType::Lookup { display_column: Some(ref c), .. } if c == "name"));
    assert_eq!(info.definition.foreign_keys[0].ref_table, "customers");
    assert!(matches!(f("total_with_tax").field_type, FieldType::Calculated { .. }));
    assert!(f("total").required && !f("status").required, "columns with defaults aren't required");
    let customers = o.describe("customers").await.unwrap();
    let col = |n: &str| customers.columns.iter().find(|c| c.name == n).unwrap().clone();
    assert_eq!(col("email").server_type, "character varying(80)");
    assert!(col("email").unique);
    assert_eq!(col("active").field_type, FieldType::YesNo);
    assert_eq!(col("prefs").field_type, FieldType::Unknown { declared: "jsonb".into() });
    assert!(customers.editable);
    assert!(!o.describe("log").await.unwrap().editable, "no primary key");
    assert!(!o.describe("big_orders").await.unwrap().editable);

    // Paging, sorting, filters and search.
    let q = RecordQuery {
        sort: vec![SortSpec { column: "name".into(), descending: true }],
        ..Default::default()
    };
    let page = online::list_records(&o, "customers", &q).await.unwrap();
    assert_eq!(page.total, 3);
    let name_i = page.columns.iter().position(|c| c == "name").unwrap();
    let order: Vec<_> = page.rows.iter().map(|r| r.values[name_i].clone()).collect();
    assert_eq!(order, [json!("Linus"), json!("Grace"), json!("Ada")]);
    assert!(matches!(&page.rows[0].key, Some(RecordKey::PrimaryKey(m)) if m["id"] == json!(3)));
    let balance_i = page.columns.iter().position(|c| c == "balance").unwrap();
    assert_eq!(page.rows[2].values[balance_i], json!("12.50"), "decimals stay exact");

    let filtered = RecordQuery {
        filters: vec![
            FilterSpec { column: "active".into(), op: FilterOp::Eq, value: json!(true) },
            FilterSpec { column: "balance".into(), op: FilterOp::Gt, value: json!(10) },
            FilterSpec { column: "joined".into(), op: FilterOp::Le, value: json!("2999-01-01") },
            FilterSpec { column: "id".into(), op: FilterOp::In, value: json!([1, 2, 3]) },
            FilterSpec { column: "email".into(), op: FilterOp::Ne, value: json!("nobody@x.io") },
        ],
        search: Some("GRA".into()),
        ..Default::default()
    };
    let page = online::list_records(&o, "customers", &filtered).await.unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.rows[0].values[name_i], json!("Grace"));

    let leads = online::list_records(&o, "sales.leads", &RecordQuery::default()).await.unwrap();
    assert_eq!(leads.columns, ["code", "Full Name", "score"]);

    // Totals for charts.
    let agg = online::aggregate(
        &o,
        "orders",
        &AggregateQuery {
            group_by: Some(GroupBy { column: "placed".into(), bucket: Some(app_lib::models::DateBucket::Month) }),
            series: vec![SeriesSpec { column: Some("total".into()), agg: Aggregate::Sum }, SeriesSpec { column: None, agg: Aggregate::Count }],
            filters: Vec::new(),
            search: None,
            order: Default::default(),
            limit: 0,
        },
    )
    .await
    .unwrap();
    assert_eq!(agg.rows, vec![vec![json!("2026-01"), json!(150.0), json!(1)], vec![json!("2026-02"), json!(320.5), json!(2)]]);

    // Writes by primary key, in the column's own types.
    let key = online::insert_record(
        &o,
        "customers",
        &values(&[("name", json!("Barbara")), ("active", json!(false)), ("balance", json!("7.25")), ("prefs", json!("{\"a\":1}")), ("id", JsonValue::Null)]),
    )
    .await
    .unwrap();
    let RecordKey::PrimaryKey(m) = &key else { panic!("expected a primary key") };
    let new_id = m["id"].as_i64().unwrap();
    assert!(new_id > 3);
    let row = online::get_record(&o, "customers", &key).await.unwrap();
    assert_eq!(row.values[name_i], json!("Barbara"));
    let key2 = online::update_record(&o, "customers", &key, &values(&[("name", json!("Barbara L.")), ("joined", json!("2026-03-01 10:00:00+00"))])).await.unwrap();
    assert_eq!(key2, key);
    let order_key = online::insert_record(&o, "orders", &values(&[("customer_id", json!(new_id)), ("total", json!(10))])).await.unwrap();
    assert!(online::insert_record(&o, "log", &values(&[("message", json!("x"))])).await.is_err(), "read-only without a key");
    let dup = online::insert_record(&o, "customers", &values(&[("name", json!("Copy")), ("email", json!("ada@x.io"))])).await;
    assert!(matches!(dup, Err(app_lib::errors::AppError::Constraint(_))), "{dup:?}");
    assert!(online::delete_records(&o, "customers", std::slice::from_ref(&key), false).await.is_err(), "needs confirm");
    assert_eq!(online::delete_records(&o, "orders", &[order_key], true).await.unwrap(), 1);
    assert_eq!(online::delete_records(&o, "customers", &[key], true).await.unwrap(), 1);

    // Queries with parameters, plans and server info.
    let mut params = Map::new();
    params.insert("min".into(), json!(100));
    params.insert("status".into(), json!("paid"));
    let r = online::run_query(&o, "SELECT id, total::text AS t FROM orders WHERE total > :min AND status = :status ORDER BY id", &params).await.unwrap();
    assert_eq!(r.rows.len(), 2);
    assert!(online::run_query(&o, "DELETE FROM orders", &Map::new()).await.is_err(), "read-only");
    let plan = online::explain(&o, "SELECT * FROM orders WHERE id = 1", true).await.unwrap();
    assert!(plan.rows.iter().any(|r| r[0].as_str().unwrap_or("").contains("Execution Time")));
    let info = online::info(&o).await.unwrap();
    assert!(info.version.starts_with("PostgreSQL"));
    assert_eq!((info.tables, info.views), (4, 1));
    assert_eq!(info.schemas, ["public", "sales"]);
    assert!(info.size.unwrap() > 0);
}

#[tokio::test]
async fn mysql_end_to_end() {
    let Some(s) = settings("SLATEDB_TEST_MYSQL", Engine::Mysql) else {
        eprintln!("SLATEDB_TEST_MYSQL not set; skipping");
        return;
    };
    let pools = OnlinePools::default();
    let o = Online::new(&pools, &s, "");

    let objects = online::list_objects(&o).await.unwrap();
    let names: Vec<_> = objects.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["big_orders", "customers", "log", "orders"], "the connected database is unqualified");

    let info = online::table_info(&o, "orders").await.unwrap();
    let f = |n: &str| info.definition.fields.iter().find(|x| x.name == n).unwrap().clone();
    assert_eq!(f("id").field_type, FieldType::AutoNumber);
    assert!(matches!(f("customer_id").field_type, FieldType::Lookup { display_column: Some(ref c), .. } if c == "name"));
    assert!(matches!(f("total_with_tax").field_type, FieldType::Calculated { .. }));
    let customers = o.describe("customers").await.unwrap();
    let col = |n: &str| customers.columns.iter().find(|c| c.name == n).unwrap().clone();
    assert_eq!(col("active").field_type, FieldType::YesNo);
    assert_eq!(col("notes").field_type, FieldType::LongText);
    assert!(col("email").unique);
    assert!(!o.describe("log").await.unwrap().editable);

    let page = online::list_records(
        &o,
        "customers",
        &RecordQuery {
            sort: vec![SortSpec { column: "name".into(), descending: true }],
            filters: vec![
                FilterSpec { column: "active".into(), op: FilterOp::Eq, value: json!(true) },
                FilterSpec { column: "balance".into(), op: FilterOp::Ge, value: json!(0) },
                FilterSpec { column: "email".into(), op: FilterOp::Ne, value: json!("ada@x.io") },
            ],
            search: Some("i".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let name_i = page.columns.iter().position(|c| c == "name").unwrap();
    let order: Vec<_> = page.rows.iter().map(|r| r.values[name_i].clone()).collect();
    assert_eq!(order, [json!("Linus"), json!("Grace")], "Ada is excluded; search reaches the email column too");
    let active_i = page.columns.iter().position(|c| c == "active").unwrap();
    assert_eq!(page.rows[0].values[active_i], json!(true));

    let agg = online::aggregate(
        &o,
        "orders",
        &AggregateQuery {
            group_by: Some(GroupBy { column: "placed".into(), bucket: Some(app_lib::models::DateBucket::Month) }),
            series: vec![SeriesSpec { column: Some("total".into()), agg: Aggregate::Sum }],
            filters: Vec::new(),
            search: None,
            order: Default::default(),
            limit: 0,
        },
    )
    .await
    .unwrap();
    assert_eq!(agg.rows, vec![vec![json!("2026-01"), json!(150.0)], vec![json!("2026-02"), json!(320.5)]]);

    let key = online::insert_record(&o, "customers", &values(&[("name", json!("Barbara")), ("active", json!(false)), ("kind", json!("company"))]))
        .await
        .unwrap();
    let RecordKey::PrimaryKey(m) = &key else { panic!("expected a primary key") };
    assert!(m["id"].as_i64().unwrap() > 3);
    online::update_record(&o, "customers", &key, &values(&[("joined", json!("2026-03-01T10:00"))])).await.unwrap();
    let row = online::get_record(&o, "customers", &key).await.unwrap();
    let joined_i = page.columns.iter().position(|c| c == "joined").unwrap();
    assert_eq!(row.values[joined_i], json!("2026-03-01 10:00:00"));
    let dup = online::insert_record(&o, "customers", &values(&[("name", json!("Copy")), ("email", json!("ada@x.io"))])).await;
    assert!(matches!(dup, Err(app_lib::errors::AppError::Constraint(_))), "{dup:?}");
    assert_eq!(online::delete_records(&o, "customers", &[key], true).await.unwrap(), 1);

    let mut params = Map::new();
    params.insert("min".into(), json!(100));
    let r = online::run_query(&o, "SELECT id FROM orders WHERE total > :min ORDER BY id", &params).await.unwrap();
    assert_eq!(r.rows.len(), 2);
    assert!(online::run_query(&o, "DELETE FROM orders", &Map::new()).await.is_err());
    assert!(!online::explain(&o, "SELECT * FROM orders", false).await.unwrap().rows.is_empty());
    let info = online::info(&o).await.unwrap();
    assert_eq!((info.tables, info.views), (3, 1));
}
