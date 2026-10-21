//! All SQLite access lives here. Functions take a `&Connection` and are free
//! of Tauri types so they can be tested directly.

pub mod connection;
pub mod metadata;
pub mod migrations;
pub mod records;
pub mod schema;
pub mod transactions;

pub use connection::{DbManager, OpenDatabase};
