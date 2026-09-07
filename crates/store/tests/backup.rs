//! 0.1.2: `backup_to` snapshots the live database through SQLite's own
//! online backup API. These tests use REAL persistent files for both the
//! source (`Store::open`, not `open_in_memory`) and the destination: the
//! shape the real app is in when a user clicks "Zálohovať".
use parser::AccountKind;
use store::{Store, StoreError, TxFilter};

struct TempPath(std::path::PathBuf);
impl TempPath {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-backup-it-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        TempPath(path)
    }
}
impl Drop for TempPath {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

fn fixture(name: &str) -> parser::Statement {
    parser::parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap()).unwrap()
}

/// A real file-backed store with an account, an imported statement, a
/// LEARNED (not seed) rule from a manual `assign`, a note, and a setting
/// changed from its default: everything the contract promises a snapshot
/// keeps.
fn seeded_store(path: &std::path::Path) -> (Store, i64) {
    let mut s = Store::open(path).unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    s.set_check_updates(true).unwrap();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    let rows = s.list_transactions(&TxFilter::default()).unwrap();
    let note_row = rows[0].id;
    s.assign(&[note_row], cat, false).unwrap();
    s.save_transaction_note(note_row, "záloha si toto poznamenala").unwrap();
    (s, note_row)
}

#[test]
fn a_snapshot_retains_rules_notes_and_settings_and_opens_independently() {
    let src = TempPath::new("source");
    let dst = TempPath::new("dest");
    let (s, noted_id) = seeded_store(&src.0);

    let outcome = s.backup_to(&dst.0).unwrap();

    assert!(outcome.bytes > 0, "a non-empty database must produce a non-empty snapshot");
    let backup = Store::open(&dst.0).unwrap();
    assert!(backup.get_check_updates().unwrap(), "a changed setting must survive the snapshot");
    assert!(backup.list_rules().unwrap().iter().any(|r| r.kind == rules::RuleKind::Exact), "the learned rule from `assign` must survive the snapshot");
    let row = backup.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == noted_id).unwrap();
    assert_eq!(row.note, "záloha si toto poznamenala");
}

#[test]
fn writes_after_the_backup_never_reach_the_snapshot() {
    let src = TempPath::new("source-iso");
    let dst = TempPath::new("dest-iso");
    let (mut s, noted_id) = seeded_store(&src.0);

    s.backup_to(&dst.0).unwrap();
    s.save_transaction_note(noted_id, "napisane AZ po zalohe").unwrap();
    s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný po zálohe").unwrap();

    let backup = Store::open(&dst.0).unwrap();
    let row = backup.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == noted_id).unwrap();
    assert_eq!(row.note, "záloha si toto poznamenala", "a write after the backup must not reach the snapshot");
    assert_eq!(backup.list_accounts().unwrap().len(), 1, "an account added after the backup must not reach the snapshot");
}

#[test]
fn a_real_file_backed_source_refuses_backing_up_onto_its_own_file() {
    let src = TempPath::new("self");
    let (s, _) = seeded_store(&src.0);

    let e = s.backup_to(&src.0).unwrap_err();

    assert!(matches!(e, StoreError::BackupTargetExists { .. }), "got {e:?}");
}

#[test]
fn an_occupied_destination_is_refused_and_its_content_is_left_exactly_as_it_was() {
    let src = TempPath::new("occ-source");
    let dst = TempPath::new("occ-dest");
    let (s, _) = seeded_store(&src.0);
    std::fs::write(&dst.0, b"a previous backup, not to be touched").unwrap();

    let e = s.backup_to(&dst.0).unwrap_err();

    assert!(matches!(e, StoreError::BackupTargetExists { .. }), "got {e:?}");
    assert_eq!(std::fs::read(&dst.0).unwrap(), b"a previous backup, not to be touched");
    assert_eq!(s.list_accounts().unwrap().len(), 1, "a refused backup must not touch the live source either");
}

#[test]
fn an_empty_pre_existing_destination_is_refused_and_left_empty() {
    let src = TempPath::new("empty-occ-source");
    let dst = TempPath::new("empty-occ-dest");
    let (s, _) = seeded_store(&src.0);
    std::fs::write(&dst.0, b"").unwrap();

    let e = s.backup_to(&dst.0).unwrap_err();

    assert!(matches!(e, StoreError::BackupTargetExists { .. }), "got {e:?}");
    assert_eq!(std::fs::metadata(&dst.0).unwrap().len(), 0, "an empty existing file at the destination must not be treated as free space to write into");
}

/// The bug this repairs: `dest.exists()` then `Connection::open(dest)` left a
/// window between the check and the open where a competing writer could
/// create the same path; whichever call opened it second could overwrite or,
/// on cleanup, remove a file it never created. Many real threads racing for
/// the same destination is the closest a single-process test gets to that
/// window: with the fix (`create_new` claims the path atomically) exactly one
/// can ever win, and every loser's own cleanup only ever touches a file it
/// exclusively created, so the winner's snapshot is always left intact.
#[test]
fn concurrent_backups_to_the_same_destination_race_safely_and_exactly_one_wins() {
    let src = TempPath::new("race-source");
    let dst = TempPath::new("race-dest");
    let (seed, _) = seeded_store(&src.0);
    drop(seed);

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let src_path = src.0.clone();
            let dest_path = dst.0.clone();
            std::thread::spawn(move || Store::open(&src_path).unwrap().backup_to(&dest_path))
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    let successes = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(successes, 1, "exactly one concurrent backup_to the same destination may win: {results:?}");
    for r in &results {
        if let Err(e) = r {
            assert!(matches!(e, StoreError::BackupTargetExists { .. }), "a losing race must be reported as an existing target, not a corrupted write: got {e:?}");
        }
    }
    let backup = Store::open(&dst.0).unwrap();
    assert_eq!(backup.list_accounts().unwrap().len(), 1, "the winning snapshot must be a complete, valid database, not a partial file from a lost race");
}

#[test]
fn a_destination_whose_directory_does_not_exist_fails_cleanly_and_leaves_the_source_intact() {
    let src = TempPath::new("err-source");
    let (s, _) = seeded_store(&src.0);
    let bad_dest = std::env::temp_dir().join(format!("abakus-nonexistent-dir-{}", std::process::id())).join("backup.db");

    let e = s.backup_to(&bad_dest).unwrap_err();

    assert!(matches!(e, StoreError::Db(_)), "got {e:?}");
    assert!(!bad_dest.exists(), "a failed backup must leave no partial file behind");
    assert_eq!(s.list_accounts().unwrap().len(), 1, "the live source must be unaffected by a failed backup");
}
