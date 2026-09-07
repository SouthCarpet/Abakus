//! 0.1.2: a consistent, independently-openable snapshot of the live database,
//! taken through SQLite's own online backup API (`rusqlite`'s `backup`
//! feature) rather than copying the file's bytes. The file can be open and
//! being written by this very process while the snapshot runs; a raw
//! filesystem copy of a live SQLite file has no such guarantee and can copy a
//! torn, inconsistent set of pages.
//!
//! The snapshot is built into a temporary file this call exclusively owns,
//! then published to `dest` with one atomic, create-only filesystem move
//! (`tempfile`'s `persist_noclobber`). `dest` is never visible in a partial
//! state: it either does not exist yet, or it already holds the complete
//! snapshot. The move fails instead of overwriting if `dest` already exists
//! for any reason (the store's own file, a hardlink or symlink alias of it,
//! a previous backup, or a competing backup that published first), so no
//! separate `exists()` check runs before it.
use crate::{Result, Store, StoreError};
use rusqlite::backup::{Backup, StepResult};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupOutcome {
    pub path: String,
    pub bytes: i64,
}

/// Pages copied per `Backup::step` call.
const BACKUP_PAGES_PER_STEP: i32 = 100;
/// `rusqlite` sets this on every `Connection::open` (`sqlite3_busy_timeout`,
/// 5 seconds); restored on the source connection once the backup step loop
/// below is done owning its retry pacing.
const RUSQLITE_DEFAULT_BUSY_TIMEOUT: Duration = Duration::from_millis(5000);
/// Sleep between steps that found the source busy or locked.
const BACKUP_BUSY_PAUSE: Duration = Duration::from_millis(50);
/// Ceiling on busy/locked steps: `rusqlite`'s own `run_to_completion` retries
/// `Busy`/`Locked` forever. Worse, doing that on top of the default 5-second
/// `busy_timeout` means every single retry could itself block for up to 5
/// seconds inside SQLite before even reporting `Busy`, so a naive outer retry
/// loop is not actually bounded. The source connection's `busy_timeout` is
/// set to zero for the duration of the backup (`step` then reports `Busy`
/// immediately instead of blocking inside SQLite), so this loop is the only
/// thing pacing retries, for at most `BACKUP_MAX_BUSY_RETRIES *
/// BACKUP_BUSY_PAUSE` before reporting a normal error and giving up. That
/// keeps whatever mutex guards the caller's `Store` from being held hostage
/// by another connection holding the source locked indefinitely.
const BACKUP_MAX_BUSY_RETRIES: u32 = 40;

/// Restores the source connection's `busy_timeout` when the backup step loop
/// is done, on every exit path including an early `?` return.
struct BusyTimeoutGuard<'a>(&'a Connection);
impl<'a> BusyTimeoutGuard<'a> {
    fn zero(conn: &'a Connection) -> Result<Self> {
        conn.busy_timeout(Duration::ZERO)?;
        Ok(Self(conn))
    }
}
impl Drop for BusyTimeoutGuard<'_> {
    fn drop(&mut self) { let _ = self.0.busy_timeout(RUSQLITE_DEFAULT_BUSY_TIMEOUT); }
}

impl Store {
    /// Never touches the OS keyring: only `self.conn`, already open, and the
    /// filesystem.
    pub fn backup_to(&self, dest: &Path) -> Result<BackupOutcome> {
        let dir = dest.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        let temp = tempfile::Builder::new()
            .prefix(".abakus-backup-")
            .suffix(".tmp")
            .tempfile_in(dir)
            .map_err(|e| StoreError::Db(e.to_string()))?
            .into_temp_path();

        let bytes = self.run_backup(&temp)?;

        match temp.persist_noclobber(dest) {
            Ok(()) => Ok(BackupOutcome { path: dest.display().to_string(), bytes }),
            // `e.path`, the still-owned temp file, is dropped (and deleted)
            // at the end of this match arm; `dest` itself was never opened.
            Err(e) if e.error.kind() == ErrorKind::AlreadyExists => {
                Err(StoreError::BackupTargetExists { path: dest.display().to_string() })
            }
            Err(e) => Err(StoreError::Db(e.error.to_string())),
        }
    }

    fn run_backup(&self, dest: &Path) -> Result<i64> {
        let mut dst = Connection::open(dest)?;
        let _busy_guard = BusyTimeoutGuard::zero(&self.conn)?;
        let backup = Backup::new(&self.conn, &mut dst)?;
        let mut busy_retries = 0u32;
        loop {
            match backup.step(BACKUP_PAGES_PER_STEP)? {
                StepResult::Done => break,
                StepResult::More => {}
                StepResult::Busy | StepResult::Locked => {
                    busy_retries += 1;
                    if busy_retries > BACKUP_MAX_BUSY_RETRIES {
                        return Err(StoreError::Db("backup timed out: the source database stayed locked".to_string()));
                    }
                    std::thread::sleep(BACKUP_BUSY_PAUSE);
                }
                _ => {}
            }
        }
        drop(backup);
        drop(dst);
        let bytes = std::fs::metadata(dest).map_err(|e| StoreError::Db(e.to_string()))?.len();
        Ok(bytes as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::AccountKind;

    #[test]
    fn an_existing_destination_is_refused_and_left_untouched() {
        let s = Store::open_in_memory().unwrap();
        let dir = std::env::temp_dir().join(format!("abakus-backup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("occupied.db");
        std::fs::write(&dest, b"not a database, just occupying the name").unwrap();

        let e = s.backup_to(&dest).unwrap_err();

        assert!(matches!(e, StoreError::BackupTargetExists { .. }), "got {e:?}");
        assert_eq!(std::fs::read(&dest).unwrap(), b"not a database, just occupying the name", "an occupied destination must be left exactly as it was");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_in_memory_store_can_still_back_up_to_a_real_file() {
        let mut s = Store::open_in_memory().unwrap();
        s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        let dir = std::env::temp_dir().join(format!("abakus-backup-test-mem-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("snapshot.db");

        let outcome = s.backup_to(&dest).unwrap();

        assert!(outcome.bytes > 0);
        let reopened = Store::open(&dest).unwrap();
        assert_eq!(reopened.list_accounts().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
