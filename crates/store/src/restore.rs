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
//!
//! **Windows note.** `std::fs::rename` never overwrites an existing
//! destination on this OS. That matters once `rename_in` has already
//! published the staged backup to `db_path`: if reopening that published
//! file then fails, `db_path` is OCCUPIED, so the plain "rename the aside
//! file back" rollback cannot land there. `rollback_after_publish` handles
//! exactly that case: it moves the unopenable published file aside first
//! (under an `abakus-obnova-zlyhala-*.db` name, never deleted), which frees
//! `db_path` for the normal rollback to put the original back.
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

/// Where `rollback_after_publish` moves a published-but-unopenable database,
/// so it is never lost, just renamed out of the way under a name that says
/// what happened. Visible (not dot-prefixed) and Slovak, like the safety
/// copy: this is a recovery artifact a user might need to hand to support,
/// not internal plumbing like the `staged`/`aside` temp files.
fn failed_restore_path(db_path: &Path) -> PathBuf {
    let dir = db_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let pid = std::process::id();
    let stamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S%3f");
    dir.join(format!("abakus-obnova-zlyhala-{stamp}-{pid}.db"))
}

/// The paths one restore attempt works with, grouped so the swap and
/// rollback functions below take one argument for them instead of four
/// (`clippy::too_many_arguments`, kept as a deny-by-proxy discipline here:
/// four unrelated `&Path`s in a row is exactly the shape that invites a
/// caller to pass two of them in the wrong order).
struct RestorePaths<'a> {
    db_path: &'a Path,
    aside_path: &'a Path,
    staged_path: &'a Path,
    safety_path: &'a Path,
}

/// Renames `aside_path` back to `db_path` and reopens it through
/// `open_conn`. Called only once the live connection has already been
/// closed and `db_path` is free (either it never moved, or `swap_and_reopen`
/// already stopped before publishing, or `rollback_after_publish` has just
/// moved the occupying file out of the way). This is the one path back to a
/// working database. Never deletes `aside_path`: a failed rollback rename
/// leaves it in place as a recovery copy, alongside `safety_path`, already
/// made before any of this started, and named in every error this function
/// can return so a failure is never a dead end.
fn rollback(store: &mut Store, paths: &RestorePaths, reason: String, open_conn: &dyn Fn(&Path) -> Result<Connection>) -> StoreError {
    if let Err(rollback_err) = std::fs::rename(paths.aside_path, paths.db_path) {
        store.conn = Connection::open_in_memory().expect("opening an in-memory SQLite connection cannot fail");
        return StoreError::Db(format!(
            "{reason} Obnova pôvodnej databázy tiež zlyhala ({rollback_err}). Databáza je momentálne nedostupná. \
             Pôvodné dáta sú v súbore {}, bezpečnostná kópia je v súbore {}.",
            paths.aside_path.display(),
            paths.safety_path.display()
        ));
    }
    match open_conn(paths.db_path) {
        Ok(conn) => {
            store.conn = conn;
            StoreError::Db(reason)
        }
        Err(open_err) => {
            store.conn = Connection::open_in_memory().expect("opening an in-memory SQLite connection cannot fail");
            StoreError::Db(format!(
                "{reason} Pôvodná databáza bola vrátená do súboru {}, ale nedala sa znova otvoriť ({open_err}). Bezpečnostná kópia je v súbore {}.",
                paths.db_path.display(),
                paths.safety_path.display()
            ))
        }
    }
}

/// Called once `rename_in` has already published the staged backup to
/// `db_path` and opening that published file has failed. Unlike a bare
/// `rollback`, `db_path` is OCCUPIED here by the file that just failed to
/// open, and Windows `std::fs::rename` never overwrites an existing
/// destination (see the module doc comment). So the occupying file is moved
/// aside first, under `failed_restore_path`, never deleted, which frees
/// `db_path` for the exact same rollback every other failure path uses.
fn rollback_after_publish(store: &mut Store, paths: &RestorePaths, reason: String, open_conn: &dyn Fn(&Path) -> Result<Connection>) -> StoreError {
    let failed_path = failed_restore_path(paths.db_path);
    if let Err(move_err) = std::fs::rename(paths.db_path, &failed_path) {
        // `db_path` still holds the unopenable published file, so the
        // original cannot be put back there either. Both existing recovery
        // copies are untouched and named in full.
        store.conn = Connection::open_in_memory().expect("opening an in-memory SQLite connection cannot fail");
        return StoreError::Db(format!(
            "{reason} Odsunutie neotvoriteľnej obnovenej databázy spod cesty {} zlyhalo ({move_err}). Databáza je momentálne nedostupná. \
             Pôvodné dáta sú v súbore {}, bezpečnostná kópia je v súbore {}.",
            paths.db_path.display(),
            paths.aside_path.display(),
            paths.safety_path.display()
        ));
    }
    rollback(store, paths, format!("{reason} Neotvoriteľná obnovená databáza je odložená v súbore {}.", failed_path.display()), open_conn)
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
    /// Reopens always go through `Store::open_connection` (same
    /// `PRAGMA foreign_keys = ON` plus migration pipeline `Store::open`
    /// uses), so a restored backup on an older-but-supported schema is
    /// migrated to current immediately, in this same session.
    pub fn restore_from_with(
        &mut self,
        backup_path: &Path,
        db_path: &Path,
        rename_out: &dyn Fn(&Path, &Path) -> io::Result<()>,
        rename_in: &dyn Fn(&Path, &Path) -> io::Result<()>,
    ) -> Result<RestoreOutcome> {
        self.restore_from_with_seams(backup_path, db_path, rename_out, rename_in, &Store::open_connection)
    }

    /// The fuller seam behind [`Store::restore_from_with`], additionally
    /// exposing the reopen step itself. Production code (via
    /// `restore_from_with`) always passes `Store::open_connection`. Tests
    /// use this directly to inject a reopen failure exactly once, proving
    /// the occupied-destination rollback (`rollback_after_publish`) runs
    /// correctly on the one OS where a real file lock is not reliably
    /// reproducible in a unit test: Windows.
    pub fn restore_from_with_seams(
        &mut self,
        backup_path: &Path,
        db_path: &Path,
        rename_out: &dyn Fn(&Path, &Path) -> io::Result<()>,
        rename_in: &dyn Fn(&Path, &Path) -> io::Result<()>,
        open_conn: &dyn Fn(&Path) -> Result<Connection>,
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

        let paths = RestorePaths { db_path, aside_path: &aside_path, staged_path: &staged_path, safety_path: &safety_path };
        let result = self.swap_and_reopen(&paths, rename_out, rename_in, open_conn);
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
    /// any failure that can be recovered from at all (see the module doc
    /// comment for the one Windows case that cannot: an occupied
    /// destination that also cannot be moved aside). Never observable from
    /// outside this call: the caller holds the store's mutex for the whole
    /// operation.
    fn swap_and_reopen(
        &mut self,
        paths: &RestorePaths,
        rename_out: &dyn Fn(&Path, &Path) -> io::Result<()>,
        rename_in: &dyn Fn(&Path, &Path) -> io::Result<()>,
        open_conn: &dyn Fn(&Path) -> Result<Connection>,
    ) -> Result<()> {
        // Close the live connection so Windows releases its handle on
        // `db_path` before any rename touches it. The store always holds a
        // real connection object; this in-memory placeholder covers only the
        // instant between closing the old file and opening its replacement.
        self.conn = Connection::open_in_memory()?;

        if let Err(e) = rename_out(paths.db_path, paths.aside_path) {
            return Err(self.recover_from_failed_move_out(paths, open_conn, e));
        }

        if let Err(e) = rename_in(paths.staged_path, paths.db_path) {
            return Err(rollback(self, paths, format!("Obnova zlyhala pri zápise obnovenej databázy: {e}"), open_conn));
        }

        match open_conn(paths.db_path) {
            Ok(conn) => {
                self.conn = conn;
                Ok(())
            }
            Err(e) => Err(rollback_after_publish(self, paths, format!("Obnovená databáza sa nedala otvoriť: {e}"), open_conn)),
        }
    }

    /// `rename_out` itself failed, so the live file never moved: it is still
    /// exactly at `db_path`. Recovery here is just reopening it, but that
    /// reopen can itself fail (for example the very lock that made the
    /// rename fail also blocks the open), so this names both `db_path` and
    /// the mandatory safety copy in that case rather than propagating a bare
    /// error with no recovery path in it at all.
    fn recover_from_failed_move_out(&mut self, paths: &RestorePaths, open_conn: &dyn Fn(&Path) -> Result<Connection>, move_err: io::Error) -> StoreError {
        match open_conn(paths.db_path) {
            Ok(conn) => {
                self.conn = conn;
                StoreError::Db(format!("Obnova zlyhala pri odložení pôvodnej databázy, pôvodné dáta ostali nezmenené: {move_err}"))
            }
            Err(open_err) => {
                self.conn = Connection::open_in_memory().expect("opening an in-memory SQLite connection cannot fail");
                StoreError::Db(format!(
                    "Obnova zlyhala pri odložení pôvodnej databázy: {move_err} Pôvodné dáta ostali v súbore {}, ale nedali sa znova otvoriť ({open_err}). \
                     Bezpečnostná kópia je v súbore {}.",
                    paths.db_path.display(),
                    paths.safety_path.display()
                ))
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use parser::AccountKind;

    fn tmp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("abakus-restore-unit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// I2, occupied destination on Windows: `db_path` already holds a real
    /// file (the just-published database that failed to open) when
    /// `rollback_after_publish` runs. `std::fs::rename` never overwrites an
    /// existing destination, so this proves the occupying file is moved
    /// aside FIRST, under its own failed name, before the original comes
    /// back from `aside_path` and reopens successfully.
    #[test]
    fn rollback_after_publish_moves_the_occupied_destination_aside_before_restoring_the_original() {
        let dir = tmp_dir("occupied-dest");
        let db_path = dir.join("abakus.db");
        let aside_path = dir.join(".abakus-restore-aside-test.tmp");
        let safety_path = dir.join("abakus-pred-obnovou-test.db");
        // The just-published file currently occupying `db_path`: a real,
        // openable database (this test is about the RENAME being blocked by
        // an occupied destination, not about the file being unopenable).
        Store::open(&db_path).unwrap();
        // The real original database, already renamed aside, waiting to be
        // restored.
        {
            let mut original = Store::open(&aside_path).unwrap();
            original.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Test").unwrap();
        }
        std::fs::write(&safety_path, b"safety copy bytes").unwrap();
        let mut store = Store::open_in_memory().unwrap();

        let paths = RestorePaths { db_path: &db_path, aside_path: &aside_path, staged_path: &db_path, safety_path: &safety_path };
        let err = rollback_after_publish(&mut store, &paths, "Obnovená databáza sa nedala otvoriť: simulated".to_string(), &Store::open_connection);

        assert!(matches!(err, StoreError::Db(_)), "got {err:?}");
        assert_eq!(store.list_accounts().unwrap()[0].iban, "SK4411000000000012345678", "self.conn must be the reopened original, restored and usable");
        assert!(db_path.exists(), "the original must be back at db_path");
        assert!(!aside_path.exists(), "the aside name is consumed by the rename back onto db_path");
        let names: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names.iter().filter(|n| n.contains("obnova-zlyhala")).count(), 1, "the unopenable published file must survive under its failed name: {names:?}");
        assert!(safety_path.exists(), "the safety copy must survive untouched");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
