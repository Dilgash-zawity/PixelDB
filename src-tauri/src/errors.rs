//! Structured application errors. Every error is serialized to the frontend
//! as `{ kind, message }` so the UI can react to the category of failure.

use rusqlite::ErrorCode;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("no database is open")]
    NoDatabase,
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("constraint violation: {0}")]
    Constraint(String),
    #[error("not a valid SQLite database: {0}")]
    InvalidDatabase(String),
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    #[error("confirmation required: {0}")]
    ConfirmationRequired(String),
    #[error("database error: {0}")]
    Sqlite(String),
    #[error("i/o error: {0}")]
    Io(String),
    #[error("could not connect: {0}")]
    Connection(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::NoDatabase => "no_database",
            AppError::InvalidPath(_) => "invalid_path",
            AppError::AlreadyExists(_) => "already_exists",
            AppError::NotFound(_) => "not_found",
            AppError::InvalidIdentifier(_) => "invalid_identifier",
            AppError::Validation(_) => "validation",
            AppError::Constraint(_) => "constraint",
            AppError::InvalidDatabase(_) => "invalid_database",
            AppError::Unsupported(_) => "unsupported",
            AppError::ConfirmationRequired(_) => "confirmation_required",
            AppError::Sqlite(_) => "sqlite",
            AppError::Io(_) => "io",
            AppError::Connection(_) => "connection",
            AppError::Internal(_) => "internal",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        match &err {
            rusqlite::Error::SqliteFailure(e, msg) => {
                let text = msg.clone().unwrap_or_else(|| e.to_string());
                match e.code {
                    ErrorCode::ConstraintViolation => AppError::Constraint(text),
                    ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt => {
                        AppError::InvalidDatabase(text)
                    }
                    _ => AppError::Sqlite(text),
                }
            }
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("no matching row".into()),
            _ => AppError::Sqlite(err.to_string()),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Internal(format!("metadata serialization: {err}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    /// Prefixes the message with context while keeping the error kind.
    pub fn context(self, ctx: impl std::fmt::Display) -> AppError {
        let wrap = |m: String| format!("{ctx}: {m}");
        match self {
            AppError::InvalidPath(m) => AppError::InvalidPath(wrap(m)),
            AppError::AlreadyExists(m) => AppError::AlreadyExists(wrap(m)),
            AppError::NotFound(m) => AppError::NotFound(wrap(m)),
            AppError::InvalidIdentifier(m) => AppError::InvalidIdentifier(wrap(m)),
            AppError::Validation(m) => AppError::Validation(wrap(m)),
            AppError::Constraint(m) => AppError::Constraint(wrap(m)),
            AppError::InvalidDatabase(m) => AppError::InvalidDatabase(wrap(m)),
            AppError::Unsupported(m) => AppError::Unsupported(wrap(m)),
            AppError::ConfirmationRequired(m) => AppError::ConfirmationRequired(wrap(m)),
            AppError::Sqlite(m) => AppError::Sqlite(wrap(m)),
            AppError::Io(m) => AppError::Io(wrap(m)),
            AppError::Connection(m) => AppError::Connection(wrap(m)),
            AppError::Internal(m) => AppError::Internal(wrap(m)),
            AppError::NoDatabase => AppError::NoDatabase,
        }
    }
}
