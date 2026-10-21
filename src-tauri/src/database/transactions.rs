use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::errors::AppResult;

/// Runs `f` inside an IMMEDIATE transaction. The transaction is rolled back
/// when `f` returns an error (or panics) and committed otherwise.
pub fn with_transaction<T>(
    conn: &Connection,
    f: impl FnOnce(&Transaction<'_>) -> AppResult<T>,
) -> AppResult<T> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    let out = f(&tx)?;
    tx.commit()?;
    Ok(out)
}

/// Disables foreign key enforcement until dropped. Required by SQLite's
/// table-rebuild procedure; the pragma is a no-op inside a transaction, so
/// the guard must be created before the transaction starts.
pub struct ForeignKeysOff<'a> {
    conn: &'a Connection,
    was_on: bool,
}

impl<'a> ForeignKeysOff<'a> {
    pub fn new(conn: &'a Connection) -> AppResult<Self> {
        let was_on: bool = conn.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
        conn.pragma_update(None, "foreign_keys", false)?;
        Ok(ForeignKeysOff { conn, was_on })
    }
}

impl Drop for ForeignKeysOff<'_> {
    fn drop(&mut self) {
        if self.was_on
            && let Err(e) = self.conn.pragma_update(None, "foreign_keys", true) {
                log::error!("failed to re-enable foreign keys: {e}");
            }
    }
}
