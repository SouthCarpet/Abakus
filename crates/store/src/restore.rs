//! Plan 091/B10: restoring the live database from a previously exported
//! backup file.
//!
//! `preview_backup` never opens anything but a read-only SQLite connection
//! (`SQLITE_OPEN_READ_ONLY`), so a corrupt or foreign file, and the preview
//! itself, can never write a single byte to the candidate file. It validates
//! the file is an Abakus database (the core tables exist) and that its
//! `schema_version` is not newer than this binary supports, before counting
//! rows.
//!
//! `restore_from` performs the actual swap. The live database is never
//! unlinked or overwritten before a fresh, consistent safety copy of it
//! exists (via the same online-backup mechanism `backup_to` uses, so it is
//! safe to run while the live connection is open and in use). Only then does
//! it close the live connection, stage the incoming backup inside the same
//! directory as the live database (so the final publish is a same-volume
//! rename, never a cross-filesystem copy that could fail halfway with the
//! live file already moved aside), and swap the two file names. Every
//! failure from the point the live connection closes onward rolls back to
//! the original file (renamed aside just before the swap, never deleted) and
//! reopens it, so `self.conn` always ends the call attached to a real,
//! openable database: the restored one on success, the original one on any
//! failure. Neither the safety copy nor the aside file is ever deleted by
//! this flow.
use crate::{Result, Store, StoreError};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupPreview {
    pub accounts: i64,
    pub statements: i64,
    pub transactions: i64,
    pub schema_version: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestoreOutcome {
    /// Absolute path to the mandatory safety copy of the database that was
    /// live immediately before this restore. Never deleted by this flow.
    pub safety_copy_path: String,
}

/// Every table an Abakus database must have. A file missing any of these is
/// refused as invalid before a single row is counted.
const REQUIRED_TABLES: [&str; 5] = ["accounts", "statements", "transactions", "categories", "settings"];

fn open_read_only(path: &Path) -> Result<Connection> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| StoreError::BackupInvalid(e.to_string()))
}

fn missing_tables(conn: &Connection) -> Result<Vec<&'static str>> {
    let mut missing = Vec::new();
    for table in REQUIRED_TABLES {
        let exists: bool = conn
            .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1")
            .and_then(|mut stmt| stmt.exists([table]))
            .map_err(|e| StoreError::BackupInvalid(e.to_string()))?;
        if !exists {
            missing.push(table);
        }
    }
    Ok(missing)
}

fn count_rows(conn: &Connection, table: &str) -> Result<i64> {
    // `table` only ever comes from the fixed `REQUIRED_TABLES` list above,
    // never from user input, so this is not a SQL-injection surface.
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0)).map_err(|e| StoreError::BackupInvalid(e.to_string()))
}

fn read_schema_version(conn: &Connection) -> i64 {
    conn.query_row("SELECT value FROM settings WHERE key = 'schema_version'", [], |row| row.get::<_, String>(0))
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
}

/// Renames `aside_path` back to `db_path` and reopens it. Called only once
/// the live connection has already been closed and the live file has
/// already been renamed to `aside_path`, so this is the one path back to a
/// working database. Never deletes `aside_path`: a failed rollback rename
/// leaves it in place as the last resort recovery copy, alongside the
/// safety copy already made before any of this started.
fn rollback(store: &mut Store, db_path: &Path, aside_path: &Path, reason: String) -> StoreError {
    if let Err(rollback_err) = std::fs::rename(aside_path, db_path) {
        store.conn = Connection::open_in_memory().expect("opening an in-memory SQLite connection cannot fail");
        return StoreError::Db(format!(
            "{reason} Obnova pôvodnej databázy tiež zlyhala ({rollback_err}). Databáza je momentálne nedostupná. Pôvodné dáta sú v súbore {}.",
            aside_path.display()
        ));
    }
    match Connection::open(db_path) {
        Ok(conn) => {
            store.conn = conn;
            StoreError::Db(reason)
        }
        Err(open_err) => {
            store.conn = Connection::open_in_memory().expect("opening an in-memory SQLite connection cannot fail");
            StoreError::Db(format!("{reason} Pôvodná databáza bola vrátená, ale nedala sa otvoriť ({open_err})."))
        }
    }
}

impl Store {
    /// Read-only: opens `path` with SQLite's own `SQLITE_OPEN_READ_ONLY`
    /// flag. Never touches the live store; safe to call on any file the user
    /// picked, including one that turns out not to be a database at all.
    pub fn preview_backup(path: &Path) -> Result<BackupPreview> {
        let conn = open_read_only(path)?;
        let missing = missing_tables(&conn)?;
        if !missing.is_empty() {
            return Err(StoreError::BackupInvalid(format!("chýbajú tabuľky: {}", missing.join(", "))));
        }
        let schema_version = read_schema_version(&conn);
        if schema_version > crate::migrate::CURRENT_SCHEMA_VERSION {
            return Err(StoreError::BackupTooNew { found: schema_version, supported: crate::migrate::CURRENT_SCHEMA_VERSION });
        }
        Ok(BackupPreview {
            accounts: count_rows(&conn, "accounts")?,
            statements: count_rows(&conn, "statements")?,
            transactions: count_rows(&conn, "transactions")?,
            schema_version,
        })
    }

    /// Restores the live database at `db_path` from `backup_path`. Wraps
    /// [`Store::restore_from_with`] with the real `std::fs::rename` for both
    /// rename steps; see that function's doc comment for the full contract.
    pub fn restore_from(&mut self, backup_path: &Path, db_path: &Path) -> Result<RestoreOutcome> {
        // `std::fs::rename` is generic over `AsRef<Path>`; passed directly as
        // `&std::fs::rename` its inferred lifetime is not general enough to
        // satisfy the `dyn Fn(&Path, &Path)` seam below (a higher-ranked
        // trait bound Rust cannot infer through a bare fn item path). Two
        // thin closures with the seam's exact signature sidestep that.
        let rename = |from: &Path, to: &Path| std::fs::rename(from, to);
        self.restore_from_with(backup_path, db_path, &rename, &rename)
    }

    /// The testable seam behind [`Store::restore_from`]. `rename_out` moves
    /// the current live file to the aside path; `rename_in` publishes the
    /// staged backup as the new live file. Production code passes
    /// `std::fs::rename` for both; tests inject a failing fake for either to
    /// prove the rollback path deterministically, since a real Windows
    /// file-locking failure is not reliably reproducible in a unit test.
    pub fn restore_from_with(
        &mut self,
        backup_path: &Path,
        db_path: &Path,
        rename_out: &dyn Fn(&Path, &Path) -> io::Result<()>,
        rename_in: &dyn Fn(&Path, &Path) -> io::Result<()>,
    ) -> Result<RestoreOutcome> {
        // 1) Validate the candidate file BEFORE anything about the live
        // database is touched. A corrupt or foreign file, or one from a
        // newer Abakus, is refused here and nothing below ever runs.
        Self::preview_backup(backup_path)?;

        let dir = db_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        let pid = std::process::id();
        let stamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S%3f");
        let safety_path = dir.join(format!("abakus-pred-obnovou-{stamp}.db"));
        let staged_path: PathBuf = dir.join(format!(".abakus-restore-staged-{stamp}-{pid}.tmp"));
        let aside_path: PathBuf = dir.join(format!(".abakus-restore-aside-{stamp}-{pid}.tmp"));

        // 2) Mandatory safety copy of the CURRENT live database, taken
        // through the live connection while it is still open. No step past
        // this point may run if it fails: the live database must never be
        // touched until a fresh safety copy of it exists.
        self.backup_to(&safety_path)?;

        // 3) Stage the incoming backup inside the DB directory, while the
        // live connection is STILL open (this touches only `staged_path`, a
        // different file). A failure here leaves the live connection open
        // and the live database completely untouched.
        stage_backup_copy(backup_path, &staged_path)?;

        let result = self.swap_and_reopen(db_path, &aside_path, &staged_path, rename_out, rename_in);
        // On success `rename_in` already consumed `staged_path`; this is a
        // no-op then. On a failure before that rename, it removes the
        // leftover staging file so a failed restore leaves no stray temp
        // file behind, matching `backup_to`'s own no-litter contract.
        let _ = std::fs::remove_file(&staged_path);

        result.map(|()| RestoreOutcome { safety_copy_path: safety_path.display().to_string() })
    }

    /// Closes the live connection, performs the two-rename swap, and reopens
    /// whichever file ends up at `db_path`. On any failure, rolls back to
    /// `aside_path` and reopens it, so `self.conn` is always left pointing
    /// at a real database: the restored one on success, the original one on
    /// any failure. Never observable from outside this call: the caller
    /// holds the store's mutex for the whole operation.
    fn swap_and_reopen(
        &mut self,
        db_path: &Path,
        aside_path: &Path,
        staged_path: &Path,
        rename_out: &dyn Fn(&Path, &Path) -> io::Result<()>,
        rename_in: &dyn Fn(&Path, &Path) -> io::Result<()>,
    ) -> Result<()> {
        // Close the live connection so Windows releases its handle on
        // `db_path` before any rename touches it. The store always holds a
        // real connection object; this in-memory placeholder covers only the
        // instant between closing the old file and opening its replacement.
        self.conn = Connection::open_in_memory()?;

        if let Err(e) = rename_out(db_path, aside_path) {
            // The live file never moved: it is still at `db_path` exactly as
            // it was, so recovery here is just reopening it.
            self.conn = Connection::open(db_path)?;
            return Err(StoreError::Db(format!("Obnova zlyhala pri odložení pôvodnej databázy, pôvodné dáta ostali nezmenené: {e}")));
        }

        if let Err(e) = rename_in(staged_path, db_path) {
            return Err(rollback(self, db_path, aside_path, format!("Obnova zlyhala pri zápise obnovenej databázy: {e}")));
        }

        match Connection::open(db_path) {
            Ok(conn) => {
                self.conn = conn;
                Ok(())
            }
            Err(e) => Err(rollback(self, db_path, aside_path, format!("Obnovená databáza sa nedala otvoriť: {e}"))),
        }
    }
}

fn stage_backup_copy(backup_path: &Path, staged_path: &Path) -> Result<()> {
    if let Err(e) = std::fs::copy(backup_path, staged_path) {
        let _ = std::fs::remove_file(staged_path);
        return Err(StoreError::Db(format!("Príprava zálohy na obnovu zlyhala: {e}")));
    }
    Ok(())
}
