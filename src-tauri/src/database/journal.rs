//! Undo and redo for record changes, using SQLite's session extension: a
//! change runs inside a session that records every row it touched (rows
//! removed by cascading deletes and trigger changes included), and undoing
//! applies the inverted changeset. A changeset that no longer fits — the row
//! was changed again elsewhere, or a foreign key would break — is refused as
//! a whole, so undo never half-applies.
//!
//! The bundled SQLite is compiled with SQLITE_ENABLE_SESSION (see
//! `.cargo/config.toml`); its functions are called through `rusqlite::ffi`.
//! Only tables with a PRIMARY KEY are recorded (every table made here has
//! one). Structure changes clear the journal, since old changesets describe
//! columns that may be gone.

use std::ffi::{CStr, CString, c_int, c_void};
use std::ptr;

use rusqlite::{Connection, ffi};
use serde::Serialize;

use crate::errors::{AppError, AppResult};
use crate::validation::identifiers::is_reserved_name;

/// Steps kept per database.
pub const MAX_STEPS: usize = 50;
/// Changesets larger than this are not kept (the step can't be undone).
pub const MAX_STEP_BYTES: usize = 32 * 1024 * 1024;

struct Step {
    label: String,
    changeset: Vec<u8>,
}

#[derive(Default)]
pub struct Journal {
    undo: Vec<Step>,
    redo: Vec<Step>,
}

/// What can be undone and redone, by label.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct UndoState {
    pub undo: Option<String>,
    pub redo: Option<String>,
}

impl Journal {
    pub fn state(&self) -> UndoState {
        UndoState { undo: self.undo.last().map(|s| s.label.clone()), redo: self.redo.last().map(|s| s.label.clone()) }
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Adds a step that was just made; a new change ends what could be redone.
    fn push(&mut self, label: String, changeset: Vec<u8>) {
        if changeset.is_empty() {
            return;
        }
        self.redo.clear();
        if changeset.len() > MAX_STEP_BYTES {
            // Too big to keep: earlier steps can't be undone across it either.
            self.undo.clear();
            return;
        }
        self.undo.push(Step { label, changeset });
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
    }
}

fn sqlite_error(conn: &Connection, rc: c_int) -> AppError {
    if rc == ffi::SQLITE_ABORT || rc == ffi::SQLITE_CONSTRAINT {
        return AppError::Constraint("the records were changed since, so this can't be undone".into());
    }
    // SAFETY: the handle is valid for the connection's lifetime; errmsg returns a NUL-terminated string owned by SQLite.
    let msg = unsafe { CStr::from_ptr(ffi::sqlite3_errmsg(conn.handle())) }.to_string_lossy().into_owned();
    AppError::Sqlite(msg)
}

/// A session recording changes to the user's tables on `main`; deleted when dropped.
struct RecordingSession(*mut ffi::sqlite3_session);

impl Drop for RecordingSession {
    fn drop(&mut self) {
        // SAFETY: created by sqlite3session_create and deleted only here.
        unsafe { ffi::sqlite3session_delete(self.0) };
    }
}

impl RecordingSession {
    fn start(conn: &Connection) -> AppResult<Self> {
        let mut raw = ptr::null_mut();
        // SAFETY: valid connection handle and NUL-terminated schema name; `raw` receives the new session.
        let rc = unsafe { ffi::sqlite3session_create(conn.handle(), c"main".as_ptr(), &mut raw) };
        if rc != ffi::SQLITE_OK {
            return Err(sqlite_error(conn, rc));
        }
        let session = RecordingSession(raw);
        let mut stmt = conn.prepare("SELECT name FROM sqlite_schema WHERE type = 'table'")?;
        let names: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        for name in names.into_iter().filter(|n| !is_reserved_name(n)) {
            let c = CString::new(name).map_err(|e| AppError::Internal(e.to_string()))?;
            // SAFETY: live session and NUL-terminated table name (SQLite copies it).
            let rc = unsafe { ffi::sqlite3session_attach(session.0, c.as_ptr()) };
            if rc != ffi::SQLITE_OK {
                return Err(sqlite_error(conn, rc));
            }
        }
        Ok(session)
    }

    fn changeset(&self, conn: &Connection) -> AppResult<Vec<u8>> {
        let mut n: c_int = 0;
        let mut p: *mut c_void = ptr::null_mut();
        // SAFETY: live session; on success `p` points to `n` bytes that we copy and free with sqlite3_free.
        let rc = unsafe { ffi::sqlite3session_changeset(self.0, &mut n, &mut p) };
        if rc != ffi::SQLITE_OK {
            return Err(sqlite_error(conn, rc));
        }
        Ok(unsafe { take_buffer(p, n) })
    }
}

/// Copies and frees a buffer SQLite allocated.
///
/// # Safety
/// `p` must be null or point to `n` bytes allocated by SQLite.
unsafe fn take_buffer(p: *mut c_void, n: c_int) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    let out = unsafe { std::slice::from_raw_parts(p as *const u8, n as usize) }.to_vec();
    unsafe { ffi::sqlite3_free(p) };
    out
}

/// Runs `f`, recording its row changes as one step called `label`.
pub fn record<T>(conn: &Connection, journal: &mut Journal, label: impl Into<String>, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
    let session = RecordingSession::start(conn)?;
    let out = f(conn)?;
    let changeset = session.changeset(conn)?;
    drop(session);
    journal.push(label.into(), changeset);
    Ok(out)
}

/// Any conflict (the row changed or is gone, a key or foreign key breaks) aborts the whole apply.
unsafe extern "C" fn abort_on_conflict(_ctx: *mut c_void, _kind: c_int, _iter: *mut ffi::sqlite3_changeset_iter) -> c_int {
    ffi::SQLITE_CHANGESET_ABORT
}

fn apply(conn: &Connection, changeset: &[u8]) -> AppResult<()> {
    // SAFETY: SQLite only reads the changeset; the callbacks match the expected signatures.
    let rc = unsafe {
        ffi::sqlite3changeset_apply(
            conn.handle(),
            changeset.len() as c_int,
            changeset.as_ptr() as *mut c_void,
            None,
            Some(abort_on_conflict),
            ptr::null_mut(),
        )
    };
    if rc != ffi::SQLITE_OK {
        return Err(sqlite_error(conn, rc));
    }
    Ok(())
}

fn invert(conn: &Connection, changeset: &[u8]) -> AppResult<Vec<u8>> {
    let mut n: c_int = 0;
    let mut p: *mut c_void = ptr::null_mut();
    // SAFETY: reads `changeset`; on success `p` holds `n` bytes allocated by SQLite.
    let rc = unsafe { ffi::sqlite3changeset_invert(changeset.len() as c_int, changeset.as_ptr() as *const c_void, &mut n, &mut p) };
    if rc != ffi::SQLITE_OK {
        return Err(sqlite_error(conn, rc));
    }
    Ok(unsafe { take_buffer(p, n) })
}

/// Undoes the latest step; returns its label, or None when there is nothing to undo.
pub fn undo(conn: &Connection, journal: &mut Journal) -> AppResult<Option<String>> {
    let Some(step) = journal.undo.pop() else { return Ok(None) };
    // A step that can't be undone now never will be; it is dropped and the older steps stay.
    apply(conn, &invert(conn, &step.changeset)?)?;
    let label = step.label.clone();
    journal.redo.push(step);
    Ok(Some(label))
}

/// Redoes the latest undone step.
pub fn redo(conn: &Connection, journal: &mut Journal) -> AppResult<Option<String>> {
    let Some(step) = journal.redo.pop() else { return Ok(None) };
    apply(conn, &step.changeset)?;
    let label = step.label.clone();
    journal.undo.push(step);
    Ok(Some(label))
}
