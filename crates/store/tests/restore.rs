//! Plan 091/B10: restoring the live database from a previously exported
//! backup. These tests use REAL persistent files, the same shape the real
//! app is in (`Store::open`, never `open_in_memory`), because the swap under
//! test closes and reopens a real file-backed connection.
use parser::AccountKind;
use store::{Store, StoreError};

struct TempDir(std::path::PathBuf);
impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-restore-it-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }
    fn file_names(&self) -> std::collections::BTreeSet<String> {
        std::fs::read_dir(&self.0).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect()
    }
}
impl Drop for TempDir {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

// `std::fs::rename` is generic; passed directly as `&std::fs::rename` its
// inferred lifetime is not general enough for the `dyn Fn(&Path, &Path)`
// seam (see `Store::restore_from`'s own comment on this). A thin closure
// with the seam's exact signature sidesteps it.
fn real_rename(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> { std::fs::rename(from, to) }

fn fixture(name: &str) -> parser::Statement {
    parser::parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap()).unwrap()
}

const IBAN_A: &str = "SK4411000000000012345678";
const IBAN_B: &str = "SK3711000000000098765432";

/// A real file-backed store with one account (`IBAN_A`, matching the
/// fixture's own hardcoded IBAN) and one imported statement: enough for the
/// preview counts to be non-trivial and distinguishable from an empty
/// database.
fn seeded_store(path: &std::path::Path) -> Store {
    let mut s = Store::open(path).unwrap();
    s.upsert_account(IBAN_A, AccountKind::Personal, "Osobný").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    s
}

/// A distinct, distinguishable store: a real file-backed database with just
/// one account under `iban`, no import (the synthetic fixture's statement
/// text hardcodes `IBAN_A`, so a second account cannot import it). Enough to
/// prove which of two databases' content is where after a restore.
fn store_with_account(path: &std::path::Path, iban: &str, label: &str) -> Store {
    let mut s = Store::open(path).unwrap();
    s.upsert_account(iban, AccountKind::Personal, label).unwrap();
    s
}

#[test]
fn preview_of_a_valid_backup_reports_accurate_counts_and_current_schema_version() {
    let dir = TempDir::new("preview-ok");
    let path = dir.0.join("backup.db");
    let s = seeded_store(&path);
    drop(s);

    let preview = Store::preview_backup(&path).unwrap();

    assert_eq!(preview.accounts, 1);
    assert_eq!(preview.statements, 1);
    assert_eq!(preview.transactions, 8);
    assert_eq!(preview.schema_version, store::migrate::CURRENT_SCHEMA_VERSION);
}

#[test]
fn preview_of_a_random_non_database_file_is_refused_as_invalid_and_never_written_to() {
    let dir = TempDir::new("preview-garbage");
    let path = dir.0.join("not-a-db.db");
    std::fs::write(&path, b"this is not a sqlite file at all").unwrap();
    let before = std::fs::read(&path).unwrap();

    let e = Store::preview_backup(&path).unwrap_err();

    assert!(matches!(e, StoreError::BackupInvalid(_)), "got {e:?}");
    assert_eq!(std::fs::read(&path).unwrap(), before, "a read-only preview must never write to the candidate file");
}

#[test]
fn preview_of_a_foreign_sqlite_database_missing_abakus_tables_is_refused() {
    let dir = TempDir::new("preview-foreign");
    let path = dir.0.join("foreign.db");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE foo (id INTEGER PRIMARY KEY);").unwrap();
    drop(conn);

    let e = Store::preview_backup(&path).unwrap_err();

    assert!(matches!(e, StoreError::BackupInvalid(_)), "got {e:?}");
}

#[test]
fn preview_of_a_backup_with_a_newer_schema_version_than_supported_is_refused() {
    let dir = TempDir::new("preview-too-new");
    let path = dir.0.join("too-new.db");
    let s = seeded_store(&path);
    drop(s);
    let too_new = store::migrate::CURRENT_SCHEMA_VERSION + 1;
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute("UPDATE settings SET value = ?1 WHERE key = 'schema_version'", [too_new.to_string()]).unwrap();
    drop(raw);

    let e = Store::preview_backup(&path).unwrap_err();

    match e {
        StoreError::BackupTooNew { found, supported } => {
            assert_eq!(found, too_new);
            assert_eq!(supported, store::migrate::CURRENT_SCHEMA_VERSION);
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn restore_replaces_the_live_database_and_the_previous_content_is_recoverable_from_the_safety_copy() {
    let dir = TempDir::new("restore-ok");
    let db_path = dir.0.join("abakus.db");
    let backup_path = dir.0.join("incoming-backup.db");
    let mut live = seeded_store(&db_path);
    let backup_store = store_with_account(&backup_path, IBAN_B, "Zo zálohy");
    drop(backup_store);

    let outcome = live.restore_from(&backup_path, &db_path).unwrap();

    // The SAME in-process Store instance must reflect the restored content
    // immediately: the app never needs to be restarted to see it.
    let accounts = live.list_accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].iban, IBAN_B);

    // A fresh handle on the live path sees the same restored content.
    let reopened = Store::open(&db_path).unwrap();
    assert_eq!(reopened.list_accounts().unwrap()[0].iban, IBAN_B);

    // The pre-restore content is fully recoverable from the safety copy.
    let safety = Store::open(std::path::Path::new(&outcome.safety_copy_path)).unwrap();
    assert_eq!(safety.list_accounts().unwrap()[0].iban, IBAN_A);
}

#[test]
fn restore_is_refused_and_nothing_is_touched_when_the_backup_file_is_invalid() {
    let dir = TempDir::new("restore-invalid");
    let db_path = dir.0.join("abakus.db");
    let backup_path = dir.0.join("garbage.db");
    let mut live = seeded_store(&db_path);
    std::fs::write(&backup_path, b"garbage, not a database").unwrap();

    let e = live.restore_from(&backup_path, &db_path).unwrap_err();

    assert!(matches!(e, StoreError::BackupInvalid(_)), "got {e:?}");
    assert_eq!(live.list_accounts().unwrap()[0].iban, IBAN_A, "the live store must be untouched and still usable");
    let names = dir.file_names();
    let expected: std::collections::BTreeSet<String> = ["abakus.db".to_string(), "garbage.db".to_string()].into_iter().collect();
    assert_eq!(names, expected, "an invalid backup must be refused before any safety copy or staging file is created: {names:?}");
}

#[test]
fn a_rename_failure_moving_the_live_database_aside_leaves_the_original_database_intact_and_reopened() {
    let dir = TempDir::new("restore-rename-out-fails");
    let db_path = dir.0.join("abakus.db");
    let backup_path = dir.0.join("incoming-backup.db");
    let mut live = seeded_store(&db_path);
    let backup_store = store_with_account(&backup_path, IBAN_B, "Zo zálohy");
    drop(backup_store);
    let failing_rename_out = |_: &std::path::Path, _: &std::path::Path| -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "simulated: cannot move the live database aside"))
    };

    let e = live.restore_from_with(&backup_path, &db_path, &failing_rename_out, &real_rename).unwrap_err();

    assert!(matches!(e, StoreError::Db(_)), "got {e:?}");
    assert_eq!(live.list_accounts().unwrap()[0].iban, IBAN_A, "the original database must still be the live one, reopened and usable");
    assert!(db_path.exists(), "the live file must never have moved when the move-aside step itself failed");
}

#[test]
fn a_rename_failure_publishing_the_staged_backup_rolls_back_to_the_original_database() {
    let dir = TempDir::new("restore-rename-in-fails");
    let db_path = dir.0.join("abakus.db");
    let backup_path = dir.0.join("incoming-backup.db");
    let mut live = seeded_store(&db_path);
    let backup_store = store_with_account(&backup_path, IBAN_B, "Zo zálohy");
    drop(backup_store);
    let failing_rename_in = |_: &std::path::Path, _: &std::path::Path| -> std::io::Result<()> {
        Err(std::io::Error::other("simulated: cannot publish the restored database"))
    };

    let e = live.restore_from_with(&backup_path, &db_path, &real_rename, &failing_rename_in).unwrap_err();

    assert!(matches!(e, StoreError::Db(_)), "got {e:?}");
    assert_eq!(live.list_accounts().unwrap()[0].iban, IBAN_A, "a failed publish must roll back to the original data, reopened and usable");
    assert!(db_path.exists(), "the live path must hold a real database again after rollback");
    let reopened = Store::open(&db_path).unwrap();
    assert_eq!(reopened.list_accounts().unwrap()[0].iban, IBAN_A);
}

/// Per the brief: "the aside/safety copies are never deleted by this flow".
/// A successful restore therefore deliberately leaves BOTH the explicit
/// safety copy AND the renamed-aside original on disk, an intentional
/// redundant recovery trail, never just one of the two. Only the transient
/// staged-copy temp file is ever cleaned up (consumed by the publish rename
/// on success).
#[test]
fn a_successful_restore_never_deletes_the_safety_copy_or_the_aside_original_and_leaves_no_other_stray_temp_file() {
    let dir = TempDir::new("restore-clean");
    let db_path = dir.0.join("abakus.db");
    let backup_path = dir.0.join("incoming-backup.db");
    let mut live = seeded_store(&db_path);
    let backup_store = store_with_account(&backup_path, IBAN_B, "Zo zálohy");
    drop(backup_store);

    let outcome = live.restore_from(&backup_path, &db_path).unwrap();

    let names = dir.file_names();
    assert!(names.contains("abakus.db"));
    assert!(names.contains("incoming-backup.db"));
    let safety_name = std::path::Path::new(&outcome.safety_copy_path).file_name().unwrap().to_string_lossy().into_owned();
    assert!(names.contains(&safety_name), "the safety copy must survive on disk: {names:?}");
    let aside_count = names.iter().filter(|n| n.contains("restore-aside")).count();
    assert_eq!(aside_count, 1, "the renamed-aside original must survive on disk too, as its own recovery trail: {names:?}");
    let staged_count = names.iter().filter(|n| n.contains("restore-staged")).count();
    assert_eq!(staged_count, 0, "the transient staged-copy temp file must not survive a successful restore: {names:?}");
    assert_eq!(names.len(), 4, "exactly source, live, safety copy and aside must remain, nothing else: {names:?}");
}
