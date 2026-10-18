use chrono::NaiveDate;
use parser::{AccountKind, Statement, Transaction, TxKind};
use rules::RuleKind;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::Duration;
use store::{Store, TxFilter};
use tempfile::TempDir;

const IBAN: &str = "SK4411000000000012345678";

struct TestDb {
    _dir: TempDir,
    path: PathBuf,
}

impl TestDb {
    fn new(rows: Vec<Transaction>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("assignment-undo.db");
        let mut store = Store::open(&path).unwrap();
        store
            .upsert_account(IBAN, AccountKind::Personal, "Osobný")
            .unwrap();
        store
            .import_statement(&statement(rows), "undo-input")
            .unwrap();
        drop(store);
        Self { _dir: dir, path }
    }

    fn open(&self) -> Store {
        Store::open(&self.path).unwrap()
    }

    fn connection(&self) -> Connection {
        Connection::open(&self.path).unwrap()
    }
}

fn statement(rows: Vec<Transaction>) -> Statement {
    Statement {
        iban: IBAN.into(),
        account_kind: AccountKind::Personal,
        number: 91,
        period_start: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
        period_end: NaiveDate::from_ymd_opt(2026, 8, 31).unwrap(),
        opening_cents: Some(120_000),
        closing_cents: Some(110_000),
        transactions: rows,
        warnings: Vec::new(),
    }
}

fn row(day: u32, merchant: &str, place: Option<&str>) -> Transaction {
    let date = NaiveDate::from_ymd_opt(2026, 8, day).unwrap();
    let mut row = Transaction::blank(
        date,
        -i64::from(day) * 137,
        TxKind::Card,
        format!("raw-row-{day}"),
    );
    row.merchant_raw = merchant.into();
    row.place = place.map(str::to_string);
    row.reference = Some(format!("reference-{day}"));
    row
}

fn transfer(day: u32) -> Transaction {
    let date = NaiveDate::from_ymd_opt(2026, 8, day).unwrap();
    let mut row = Transaction::blank(date, -999, TxKind::TransferOut, "raw-transfer".into());
    row.merchant_raw = "OWN TRANSFER".into();
    row.counterparty_iban = Some(IBAN.into());
    row
}

fn id(store: &Store, merchant: &str) -> i64 {
    store
        .list_transactions(&TxFilter::default())
        .unwrap()
        .into_iter()
        .find(|row| row.merchant_raw == merchant)
        .unwrap()
        .id
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RowImage {
    id: i64,
    status: String,
    category_id: Option<i64>,
    rule_id: Option<i64>,
    source: String,
    amount_cents: i64,
    fingerprint: String,
    note: String,
    raw_block: String,
    reference: Option<String>,
}

fn row_image(connection: &Connection, id: i64) -> RowImage {
    connection
        .query_row(
            "SELECT id, status, category_id, rule_id, source, amount_cents, fingerprint, note, raw_block, reference FROM transactions WHERE id = ?1",
            [id],
            |row| Ok(RowImage {
                id: row.get(0)?,
                status: row.get(1)?,
                category_id: row.get(2)?,
                rule_id: row.get(3)?,
                source: row.get(4)?,
                amount_cents: row.get(5)?,
                fingerprint: row.get(6)?,
                note: row.get(7)?,
                raw_block: row.get(8)?,
                reference: row.get(9)?,
            }),
        )
        .unwrap()
}

fn assignment_fields(image: &RowImage) -> (&str, Option<i64>, Option<i64>, &str) {
    (
        &image.status,
        image.category_id,
        image.rule_id,
        &image.source,
    )
}

#[test]
fn undo_restores_selected_and_matching_assignment_fields_only() {
    let db = TestDb::new(vec![
        row(1, "ALFA", Some("Nitra")),
        row(2, "ALFA", Some("Žilina")),
        row(3, "CONFIRMED", Some("Nitra")),
        transfer(4),
    ]);
    let mut store = db.open();
    let selected = id(&store, "ALFA");
    let confirmed = id(&store, "CONFIRMED");
    let transfer_id = id(&store, "OWN TRANSFER");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    drop(store);
    let connection = db.connection();
    connection
        .execute(
            "UPDATE transactions SET status = 'confirmed', category_id = ?2, note = 'protected-note' WHERE id = ?1",
            params![confirmed, category],
        )
        .unwrap();
    let matching: i64 = connection
        .query_row(
            "SELECT id FROM transactions WHERE merchant_raw = 'ALFA' AND id <> ?1",
            [selected],
            |row| row.get(0),
        )
        .unwrap();
    let before_selected = row_image(&connection, selected);
    let before_matching = row_image(&connection, matching);
    let before_confirmed = row_image(&connection, confirmed);
    let before_transfer = row_image(&connection, transfer_id);
    drop(connection);
    store = db.open();

    let assigned = store.assign_undoable(&[selected], category, true).unwrap();
    let undo_id = assigned.undo_id.unwrap();
    assert_eq!(assigned.updated, 2);
    let undone = store.undo_last_assignment(&undo_id).unwrap().unwrap();
    assert_eq!(undone.restored_rows, 2);
    drop(store);

    let connection = db.connection();
    let after_selected = row_image(&connection, selected);
    let after_matching = row_image(&connection, matching);
    assert_eq!(
        assignment_fields(&after_selected),
        assignment_fields(&before_selected)
    );
    assert_eq!(
        assignment_fields(&after_matching),
        assignment_fields(&before_matching)
    );
    assert_eq!(
        after_selected, before_selected,
        "money, fingerprint, note and raw input remain exact"
    );
    assert_eq!(
        after_matching, before_matching,
        "matching row data remains exact"
    );
    assert_eq!(row_image(&connection, confirmed), before_confirmed);
    assert_eq!(row_image(&connection, transfer_id), before_transfer);
}

#[test]
fn undo_removes_created_rules_restores_retargeted_rules_and_reverts_only_new_provenance() {
    let db = TestDb::new(vec![row(1, "BETA", Some("Košice"))]);
    let mut store = db.open();
    let transaction_id = id(&store, "BETA");
    let old_category = store.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    let new_category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let merchant_rule = store
        .insert_rule(RuleKind::Merchant, "beta", None, old_category)
        .unwrap();
    store
        .record_rule_source(merchant_rule, transaction_id)
        .unwrap();

    let assigned = store
        .assign_undoable(&[transaction_id], new_category, false)
        .unwrap();
    assert_eq!(assigned.rules_created, 1);
    let undo_id = assigned.undo_id.unwrap();
    store.undo_last_assignment(&undo_id).unwrap().unwrap();
    drop(store);

    let connection = db.connection();
    let restored_category: i64 = connection
        .query_row(
            "SELECT category_id FROM rules WHERE id = ?1",
            [merchant_rule],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(restored_category, old_category);
    let exact_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM rules WHERE match_kind = 'exact' AND key = 'beta' AND place = 'kosice'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(exact_count, 0);
    let provenance: Vec<(i64, i64)> = connection
        .prepare(
            "SELECT rule_id, transaction_id FROM rule_sources ORDER BY rule_id, transaction_id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(provenance, vec![(merchant_rule, transaction_id)]);
}

#[test]
fn stale_id_does_not_consume_the_newer_assignment_record() {
    let db = TestDb::new(vec![row(1, "FIRST", None), row(2, "SECOND", None)]);
    let mut store = db.open();
    let first = id(&store, "FIRST");
    let second = id(&store, "SECOND");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let first_id = store
        .assign_undoable(&[first], category, false)
        .unwrap()
        .undo_id
        .unwrap();
    let second_id = store
        .assign_undoable(&[second], category, false)
        .unwrap()
        .undo_id
        .unwrap();

    assert_ne!(first_id, second_id);
    assert_eq!(store.undo_last_assignment(&first_id).unwrap(), None);
    assert_eq!(
        store
            .undo_last_assignment(&second_id)
            .unwrap()
            .unwrap()
            .restored_rows,
        1
    );
    drop(store);

    let connection = db.connection();
    assert_eq!(row_image(&connection, first).status, "confirmed");
    assert_eq!(row_image(&connection, second).status, "unassigned");
}

#[test]
fn second_undo_returns_none_and_changes_nothing() {
    let db = TestDb::new(vec![row(1, "ONCE", None)]);
    let mut store = db.open();
    let transaction_id = id(&store, "ONCE");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let undo_id = store
        .assign_undoable(&[transaction_id], category, false)
        .unwrap()
        .undo_id
        .unwrap();
    store.undo_last_assignment(&undo_id).unwrap().unwrap();
    let after_first = row_image(&db.connection(), transaction_id);

    assert_eq!(store.undo_last_assignment(&undo_id).unwrap(), None);
    assert_eq!(row_image(&db.connection(), transaction_id), after_first);
}

#[test]
fn reopened_store_has_no_assignment_undo_record() {
    let db = TestDb::new(vec![row(1, "REOPEN", None)]);
    let mut store = db.open();
    let transaction_id = id(&store, "REOPEN");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let undo_id = store
        .assign_undoable(&[transaction_id], category, false)
        .unwrap()
        .undo_id
        .unwrap();
    drop(store);

    let mut reopened = db.open();
    assert_eq!(reopened.undo_last_assignment(&undo_id).unwrap(), None);
    assert_eq!(
        row_image(&db.connection(), transaction_id).status,
        "confirmed"
    );
}

#[test]
fn same_connection_write_expires_undo_without_restoring_rows() {
    let db = TestDb::new(vec![row(1, "LOCAL WRITE", None)]);
    let mut store = db.open();
    let transaction_id = id(&store, "LOCAL WRITE");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let undo_id = store
        .assign_undoable(&[transaction_id], category, false)
        .unwrap()
        .undo_id
        .unwrap();
    store
        .save_transaction_note(transaction_id, "keep this note")
        .unwrap();

    assert_eq!(store.undo_last_assignment(&undo_id).unwrap(), None);
    let image = row_image(&db.connection(), transaction_id);
    assert_eq!(image.status, "confirmed");
    assert_eq!(image.note, "keep this note");
}

#[test]
fn external_connection_commit_expires_undo_inside_the_write_lock() {
    let db = TestDb::new(vec![row(1, "EXTERNAL", None)]);
    let mut store = db.open();
    let transaction_id = id(&store, "EXTERNAL");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let undo_id = store
        .assign_undoable(&[transaction_id], category, false)
        .unwrap()
        .undo_id
        .unwrap();
    db.connection()
        .execute(
            "UPDATE transactions SET note = 'external' WHERE id = ?1",
            [transaction_id],
        )
        .unwrap();

    assert_eq!(store.undo_last_assignment(&undo_id).unwrap(), None);
    let image = row_image(&db.connection(), transaction_id);
    assert_eq!(image.status, "confirmed");
    assert_eq!(image.note, "external");
}

#[test]
fn read_only_work_keeps_undo_available() {
    let db = TestDb::new(vec![row(1, "READ ONLY", None)]);
    let mut store = db.open();
    let transaction_id = id(&store, "READ ONLY");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let undo_id = store
        .assign_undoable(&[transaction_id], category, false)
        .unwrap()
        .undo_id
        .unwrap();
    assert_eq!(
        store.list_transactions(&TxFilter::default()).unwrap().len(),
        1
    );
    store.summary(None, None, None).unwrap();
    let backup_dir = tempfile::tempdir().unwrap();
    store.backup_to(&backup_dir.path().join("copy.db")).unwrap();

    assert_eq!(
        store
            .undo_last_assignment(&undo_id)
            .unwrap()
            .unwrap()
            .restored_rows,
        1
    );
}

#[test]
fn failed_undo_rolls_back_restores_consumes_record_and_keeps_connection_usable() {
    let db = TestDb::new(vec![
        row(1, "ROLLBACK ONE", None),
        row(2, "ROLLBACK TWO", None),
    ]);
    let mut store = db.open();
    let first = id(&store, "ROLLBACK ONE");
    let second = id(&store, "ROLLBACK TWO");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    drop(store);
    db.connection().execute_batch(&format!(
        "CREATE TRIGGER fail_second_undo BEFORE UPDATE OF status ON transactions WHEN OLD.id = {second} AND OLD.status = 'confirmed' AND NEW.status = 'unassigned' BEGIN SELECT RAISE(ABORT, 'forced undo failure'); END;"
    )).unwrap();
    store = db.open();
    let undo_id = store
        .assign_undoable(&[first, second], category, false)
        .unwrap()
        .undo_id
        .unwrap();
    let assigned_first = row_image(&db.connection(), first);
    let assigned_second = row_image(&db.connection(), second);

    let error = store.undo_last_assignment(&undo_id).unwrap_err();

    assert!(error.to_string().contains("forced undo failure"));
    assert_eq!(row_image(&db.connection(), first), assigned_first);
    assert_eq!(row_image(&db.connection(), second), assigned_second);
    assert_eq!(
        store.undo_last_assignment(&undo_id).unwrap(),
        None,
        "a restoration DML failure consumes the record"
    );
    store
        .save_transaction_note(first, "connection usable")
        .unwrap();
    assert_eq!(row_image(&db.connection(), first).note, "connection usable");
}

#[test]
fn failed_undoable_assignment_publishes_no_partial_state_or_record() {
    let db = TestDb::new(vec![row(1, "FAIL ONE", None), row(2, "FAIL TWO", None)]);
    let store = db.open();
    let first = id(&store, "FAIL ONE");
    let second = id(&store, "FAIL TWO");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    drop(store);
    let before_first = row_image(&db.connection(), first);
    let before_second = row_image(&db.connection(), second);
    db.connection().execute_batch(&format!(
        "CREATE TRIGGER fail_second_assignment BEFORE UPDATE OF status ON transactions WHEN OLD.id = {second} AND NEW.status = 'confirmed' BEGIN SELECT RAISE(ABORT, 'forced assignment failure'); END;"
    )).unwrap();
    let mut store = db.open();

    let error = store
        .assign_undoable(&[first, second], category, false)
        .unwrap_err();

    assert!(error.to_string().contains("forced assignment failure"));
    assert_eq!(row_image(&db.connection(), first), before_first);
    assert_eq!(row_image(&db.connection(), second), before_second);
    assert_eq!(store.undo_last_assignment("unknown-id").unwrap(), None);
}

#[test]
fn empty_or_transfer_only_assignment_returns_no_undo_and_clears_prior_record() {
    let db = TestDb::new(vec![row(1, "ASSIGNED", None), transfer(2)]);
    let mut store = db.open();
    let assigned_id = id(&store, "ASSIGNED");
    let transfer_id = id(&store, "OWN TRANSFER");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let old_undo_id = store
        .assign_undoable(&[assigned_id], category, false)
        .unwrap()
        .undo_id
        .unwrap();

    let empty = store.assign_undoable(&[], category, true).unwrap();
    assert_eq!(
        (
            empty.updated,
            empty.rules_created,
            empty.skipped_transfers,
            empty.undo_id
        ),
        (0, 0, 0, None)
    );
    assert_eq!(store.undo_last_assignment(&old_undo_id).unwrap(), None);

    let transfer_only = store
        .assign_undoable(&[transfer_id], category, true)
        .unwrap();
    assert_eq!(
        (
            transfer_only.updated,
            transfer_only.rules_created,
            transfer_only.skipped_transfers,
            transfer_only.undo_id
        ),
        (0, 0, 1, None)
    );
}

#[test]
fn undo_ids_are_distinct_across_store_instances() {
    let first_db = TestDb::new(vec![row(1, "STORE ONE", None)]);
    let second_db = TestDb::new(vec![row(1, "STORE TWO", None)]);
    let mut first_store = first_db.open();
    let mut second_store = second_db.open();
    let first_id = id(&first_store, "STORE ONE");
    let second_id = id(&second_store, "STORE TWO");
    let first_category = first_store
        .category_by_path("Nákupy/obchod")
        .unwrap()
        .unwrap();
    let second_category = second_store
        .category_by_path("Nákupy/obchod")
        .unwrap()
        .unwrap();

    let first_undo = first_store
        .assign_undoable(&[first_id], first_category, false)
        .unwrap()
        .undo_id
        .unwrap();
    let second_undo = second_store
        .assign_undoable(&[second_id], second_category, false)
        .unwrap()
        .undo_id
        .unwrap();

    assert_ne!(first_undo, second_undo);
}

#[test]
fn competing_write_lock_keeps_undo_record_for_retry_after_begin_fails() {
    let db = TestDb::new(vec![row(1, "BUSY UNDO", None)]);
    let mut store = db.open();
    let transaction_id = id(&store, "BUSY UNDO");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let before = row_image(&db.connection(), transaction_id);
    let undo_id = store
        .assign_undoable(&[transaction_id], category, false)
        .unwrap()
        .undo_id
        .unwrap();
    let assigned = row_image(&db.connection(), transaction_id);
    let competitor = db.connection();
    competitor.busy_timeout(Duration::ZERO).unwrap();
    competitor.execute_batch("BEGIN IMMEDIATE").unwrap();

    let error = store.undo_last_assignment(&undo_id).unwrap_err();

    assert!(
        error.to_string().contains("database is locked"),
        "BEGIN IMMEDIATE must report the real competing write lock: {error}"
    );
    assert_eq!(row_image(&competitor, transaction_id), assigned);
    assert_eq!(
        store.list_transactions(&TxFilter::default()).unwrap().len(),
        1,
        "the Store connection remains usable for reads while the competitor owns the write lock"
    );
    competitor.execute_batch("ROLLBACK").unwrap();

    let restored = store.undo_last_assignment(&undo_id).unwrap().unwrap();
    assert_eq!(restored.restored_rows, 1);
    assert_eq!(row_image(&db.connection(), transaction_id), before);
}

#[test]
fn deferred_foreign_key_commit_failure_rolls_back_and_consumes_undo_record() {
    let db = TestDb::new(vec![row(1, "COMMIT FAILURE", None)]);
    let store = db.open();
    let transaction_id = id(&store, "COMMIT FAILURE");
    let category = store.category_by_path("Nákupy/obchod").unwrap().unwrap();
    drop(store);
    let setup = db.connection();
    setup
        .execute_batch(&format!(
            "PRAGMA foreign_keys = ON;
         CREATE TABLE undo_commit_parent (id INTEGER PRIMARY KEY);
         CREATE TABLE undo_commit_guard (
             id INTEGER PRIMARY KEY,
             parent_id INTEGER NOT NULL REFERENCES undo_commit_parent(id)
                 DEFERRABLE INITIALLY DEFERRED
         );
         CREATE TRIGGER fail_undo_commit
         AFTER UPDATE OF status ON transactions
         WHEN OLD.id = {transaction_id}
           AND OLD.status = 'confirmed'
           AND NEW.status = 'unassigned'
         BEGIN
             INSERT INTO undo_commit_guard(parent_id) VALUES (999999);
         END;"
        ))
        .unwrap();
    drop(setup);
    let mut store = db.open();
    let undo_id = store
        .assign_undoable(&[transaction_id], category, false)
        .unwrap()
        .undo_id
        .unwrap();
    let assigned = row_image(&db.connection(), transaction_id);

    let error = store.undo_last_assignment(&undo_id).unwrap_err();

    assert!(
        error.to_string().contains("FOREIGN KEY constraint failed"),
        "the deferred violation must fail COMMIT, after restoration DML succeeded: {error}"
    );
    let connection = db.connection();
    assert_eq!(row_image(&connection, transaction_id), assigned);
    let guard_rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM undo_commit_guard", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(guard_rows, 0, "ROLLBACK removes the deferred guard write");
    drop(connection);
    assert_eq!(
        store.undo_last_assignment(&undo_id).unwrap(),
        None,
        "a COMMIT failure consumes the matching undo record"
    );
    store
        .save_transaction_note(transaction_id, "connection usable")
        .unwrap();
    assert_eq!(
        row_image(&db.connection(), transaction_id).note,
        "connection usable"
    );
}
