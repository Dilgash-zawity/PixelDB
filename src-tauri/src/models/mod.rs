//! Serializable data types shared between the backend and the frontend.

pub mod database;
pub mod field;
pub mod query;
pub mod record;
pub mod table;

pub use database::*;
pub use field::*;
pub use query::*;
pub use record::*;
pub use table::*;

#[cfg(test)]
mod wire_format_tests {
    //! The frontend's TypeScript types (src/lib/api/types.ts) mirror these
    //! shapes; keep both in sync.
    use super::*;
    use serde_json::json;

    #[test]
    fn json_shapes() {
        let mut f = FieldDef::new("a", FieldType::Currency { scale: 2 });
        f.default = Some(DefaultValue::CurrentTimestamp);
        let v = serde_json::to_value(&f).unwrap();
        assert_eq!(v["field_type"], json!({"type": "currency", "scale": 2}));
        assert_eq!(v["default"], json!({"kind": "current_timestamp"}));
        let back: DefaultValue = serde_json::from_value(json!({"kind": "current_date"})).unwrap();
        assert_eq!(back, DefaultValue::CurrentDate);
        let lit: DefaultValue = serde_json::from_value(json!({"kind": "literal", "value": 5})).unwrap();
        assert_eq!(lit, DefaultValue::Literal(json!(5)));
        // Optional type parameters may be omitted.
        let t: FieldType = serde_json::from_value(json!({"type": "short_text"})).unwrap();
        assert_eq!(t, FieldType::ShortText { max_length: None });
        let minimal: FieldDef = serde_json::from_value(json!({"name": "x", "field_type": {"type": "yes_no"}})).unwrap();
        assert!(!minimal.required);

        assert_eq!(serde_json::to_value(RecordKey::RowId(3)).unwrap(), json!({"row_id": 3}));
        let op: BatchOp = serde_json::from_value(json!({"op": "delete", "key": {"row_id": 1}})).unwrap();
        assert!(matches!(op, BatchOp::Delete { key: RecordKey::RowId(1) }));
        let q: RecordQuery = serde_json::from_value(json!({"filters": [{"column": "a", "op": "is_null"}]})).unwrap();
        assert_eq!(q.filters[0].op, FilterOp::IsNull);
        assert_eq!(serde_json::to_value(ObjectKind::View).unwrap(), json!("view"));
        let a: AggregateQuery = serde_json::from_value(json!({
            "group_by": {"column": "d", "bucket": "month"},
            "series": [{"agg": "count"}, {"column": "x", "agg": "count_distinct"}],
            "order": "value_desc"
        }))
        .unwrap();
        assert_eq!(a.group_by.unwrap().bucket, Some(DateBucket::Month));
        assert_eq!(a.series[1].agg, Aggregate::CountDistinct);
        assert_eq!(a.order, AggregateOrder::ValueDesc);
        let e = serde_json::to_value(crate::errors::AppError::NoDatabase).unwrap();
        assert_eq!(e, json!({"kind": "no_database", "message": "no database is open"}));
    }
}
