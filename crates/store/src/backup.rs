//! 0.1.2: a consistent, independently-openable snapshot of the live database,
//! taken through SQLite's own online backup API (`rusqlite`'s `backup`
//! feature) rather than copying the file's bytes. The file can be open and
//! being written by this very process while the snapshot runs; a raw
//! filesystem copy of a live SQLite file has no such guarantee and can copy a
//! torn, inconsistent set of pages.
use crate::{Result, Store, StoreError};
use rusqlite::backup::Backup;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupOutcome {
    pub path: String,
    pub bytes: i64,
}

impl Store {
    /// Refuses an existing destination outright, including the store's own
    /// file, an alias, or a previous backup. `create_new` is the actual
    /// guard: it atomically claims `dest`, failing instead of opening or
    /// truncating it if the path already exists at that instant. A prior
    /// separate `exists()` check followed by `Connection::open` would leave a
    /// window for a competing writer to create `dest` in between, so this
    /// call never does a plain existence check before opening. Never touches
    /// the OS keyring: only `self.conn`, already open, and the filesystem.
    pub fn backup_to(&self, dest: &Path) -> Result<BackupOutcome> {
        match std::fs::File::options().write(true).create_new(true).open(dest) {
            Ok(file) => drop(file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(StoreError::BackupTargetExists { path: dest.display().to_string() });
            }
            Err(e) => return Err(StoreError::Db(e.to_string())),
        }
        match self.run_backup(dest) {
            Ok(bytes) => Ok(BackupOutcome { path: dest.display().to_string(), bytes }),
            Err(e) => {
                // This call is the exclusive creator of `dest` (claimed
                // above via `create_new`), so cleanup here can never remove
                // a file another process owns.
                let _ = std::fs::remove_file(dest);
                Err(e)
            }
        }
    }

    fn run_backup(&self, dest: &Path) -> Result<i64> {
        let mut dst = Connection::open(dest)?;
        let backup = Backup::new(&self.conn, &mut dst)?;
        // `run_to_completion` (not the one-shot `Connection::backup` helper)
        // sleeps and retries on `Busy`/`Locked`, the shape SQLite's own docs
        // recommend for backing up a database that is still being written.
        backup.run_to_completion(100, Duration::from_millis(50), None)?;
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
