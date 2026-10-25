//! Reading rows from sqlx drivers (Postgres, MySQL) without knowing their
//! types up front: statements are sent unprepared, so values arrive as text.

use futures_util::{Stream, TryStreamExt};
use serde_json::Value as JsonValue;
use sqlx::{Column, Database, Decode, Row, TypeInfo, ValueRef};

use super::{binary_label, text_value};
use crate::errors::{AppError, AppResult};
use crate::models::RemoteResult;

/// Reads up to `limit` rows; `truncated` tells whether there were more.
pub async fn collect<DB, S>(mut rows: S, limit: usize) -> AppResult<RemoteResult>
where
    DB: Database,
    S: Stream<Item = Result<DB::Row, sqlx::Error>> + Unpin,
    for<'r> &'r str: Decode<'r, DB>,
    for<'r> &'r [u8]: Decode<'r, DB>,
    usize: sqlx::ColumnIndex<DB::Row>,
{
    let mut out = RemoteResult::default();
    while let Some(row) = rows.try_next().await.map_err(remote_error)? {
        if out.columns.is_empty() {
            out.columns = row.columns().iter().map(|c| c.name().to_string()).collect();
        }
        if out.rows.len() == limit {
            out.truncated = true;
            break;
        }
        let mut values = Vec::with_capacity(row.len());
        for i in 0..row.len() {
            let raw = row.try_get_raw(i).map_err(remote_error)?;
            if raw.is_null() {
                values.push(JsonValue::Null);
                continue;
            }
            let type_name = raw.type_info().name().to_string();
            values.push(match <&str as Decode<DB>>::decode(raw) {
                Ok(text) => text_value(&type_name, text),
                Err(_) => {
                    let raw = row.try_get_raw(i).map_err(remote_error)?;
                    binary_label(<&[u8] as Decode<DB>>::decode(raw).map_or(0, |b| b.len()))
                }
            });
        }
        out.rows.push(values);
    }
    Ok(out)
}

/// The server's own message where there is one.
pub fn remote_error(e: sqlx::Error) -> AppError {
    match e {
        sqlx::Error::Database(db) => AppError::Remote(db.message().to_string()),
        sqlx::Error::Io(io) => AppError::Remote(format!("couldn't reach the server: {io}")),
        sqlx::Error::Tls(t) => AppError::Remote(format!("secure connection failed: {t}")),
        sqlx::Error::PoolTimedOut => AppError::Remote("the server did not answer in time".into()),
        other => AppError::Remote(other.to_string()),
    }
}
