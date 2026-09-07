//! Owned by lane A. M01-M03 from recurring-acceptance.md: an existing
//! (pre-v4) database reaches schema version 4 atomically, a forced DDL
//! failure leaves the old version and data completely unchanged, and a
//! backup snapshot carries the new tables.
use chrono::NaiveDate;
use rusqlite::Connection;
use store::recurring::{Cadence, RecurringDecisionInput, RecurringSelection, SaveRecurringRequest};
use store::Store;

/// `crates/store/src/schema.sql` shape at schema version 3 (0.1.2 pre-
/// recurring): every table `schema.sql` already creates, `schema_version`
/// pinned to 3, no `recurring_decisions`/`recurring_members`.
const SCHEMA_V3: &str = "\
CREATE TABLE accounts (id INTEGER PRIMARY KEY, iban TEXT NOT NULL UNIQUE, kind TEXT NOT NULL CHECK (kind IN ('personal','business')), label TEXT NOT NULL DEFAULT '', has_password INTEGER NOT NULL DEFAULT 0);
CREATE TABLE statements (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id), number INTEGER NOT NULL, period_start TEXT NOT NULL, period_end TEXT NOT NULL, opening_cents INTEGER, closing_cents INTEGER, checksum_status TEXT NOT NULL, checksum_off_by INTEGER, file_hash TEXT NOT NULL UNIQUE, imported_at TEXT NOT NULL DEFAULT (datetime('now')));
CREATE TABLE categories (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES categories(id), name TEXT NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('expense','income')), sort INTEGER NOT NULL DEFAULT 0, system INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0, UNIQUE (parent_id, name));
CREATE TABLE rules (id INTEGER PRIMARY KEY, match_kind TEXT NOT NULL CHECK (match_kind IN ('exact','merchant','counterparty_account','seed')), key TEXT NOT NULL, place TEXT NOT NULL DEFAULT '', category_id INTEGER NOT NULL REFERENCES categories(id), created_at TEXT NOT NULL DEFAULT (datetime('now')), hit_count INTEGER NOT NULL DEFAULT 0, UNIQUE (match_kind, key, place));
CREATE TABLE transactions (id INTEGER PRIMARY KEY, statement_id INTEGER NOT NULL REFERENCES statements(id), account_id INTEGER NOT NULL REFERENCES accounts(id), fingerprint TEXT NOT NULL UNIQUE, posted_date TEXT NOT NULL, tx_date TEXT NOT NULL, kind TEXT NOT NULL, amount_cents INTEGER NOT NULL, orig_amount_cents INTEGER, orig_currency TEXT, rate_micros INTEGER, merchant_raw TEXT NOT NULL, merchant_norm TEXT NOT NULL, place TEXT, place_norm TEXT, counterparty_name TEXT, counterparty_iban TEXT, reference TEXT, card_last4 TEXT, raw_block TEXT NOT NULL, category_id INTEGER REFERENCES categories(id), status TEXT NOT NULL CHECK (status IN ('transfer','confirmed','suggested','unassigned')), rule_id INTEGER REFERENCES rules(id), source TEXT NOT NULL DEFAULT 'none', note TEXT NOT NULL DEFAULT '');
CREATE INDEX tx_date_idx ON transactions(tx_date);
CREATE INDEX tx_status_idx ON transactions(status);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE UNIQUE INDEX categories_top_uniq ON categories(name) WHERE parent_id IS NULL;
CREATE TABLE rule_sources (rule_id INTEGER NOT NULL REFERENCES rules(id) ON DELETE CASCADE, transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE, statement_id INTEGER NOT NULL REFERENCES statements(id) ON DELETE CASCADE, created_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY (rule_id, transaction_id));
CREATE TABLE net_log (id INTEGER PRIMARY KEY, started_at TEXT NOT NULL, url TEXT NOT NULL, status TEXT NOT NULL, duration_ms INTEGER NOT NULL, bytes_in INTEGER NOT NULL);
";

const OLD_DATA: &str = "\
INSERT INTO accounts (id, iban, kind, label) VALUES (1, 'SK4411000000000012345678', 'personal', 'Osobný');
INSERT INTO statements (id, account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (1, 1, 6, '2026-06-01', '2026-06-30', 'ok', 'v3-hash');
INSERT INTO categories (id, parent_id, name, kind) VALUES (1, NULL, 'Jedlo', 'expense');
INSERT INTO categories (id, parent_id, name, kind) VALUES (2, 1, 'potraviny', 'expense');
INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, category_id, status, source, note) VALUES (1, 1, 1, 'fp-1', '2026-06-01', '2026-06-01', 'card', -2000, 'ALDI', 'aldi', 'raw', 2, 'confirmed', 'none', 'poznamka');
INSERT INTO settings (key, value) VALUES ('check_updates', '1');
INSERT INTO settings (key, value) VALUES ('schema_version', '3');
";

struct TempDb(std::path::PathBuf);
impl TempDb {
    fn with_v3(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-recurring-migration-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(SCHEMA_V3).unwrap();
        conn.execute_batch(OLD_DATA).unwrap();
        TempDb(path)
    }
}
impl Drop for TempDb {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

/// M01: a pre-v4 database gains the two recurring tables and reaches
/// schema_version 4, with every prior row and the notes column untouched.
#[test]
fn m01_a_v3_database_gains_recurring_tables_and_keeps_every_row() {
    let db = TempDb::with_v3("m01");
    let s = Store::open(&db.0).unwrap();

    let raw = Connection::open(&db.0).unwrap();
    let version: String = raw.query_row("SELECT value FROM settings WHERE key = 'schema_version'", [], |r| r.get(0)).unwrap();
    assert_eq!(version, "4");
    let recurring_tables: i64 = raw.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('recurring_decisions','recurring_members')", [], |r| r.get(0)).unwrap();
    assert_eq!(recurring_tables, 2);

    let rows = s.list_transactions(&store::TxFilter::default()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].note, "poznamka", "the pre-existing note column and its data must survive the v4 migration");
    assert!(s.get_check_updates().unwrap());
}

/// M02: a conflicting `recurring_decisions` table (wrong shape, created
/// before `Store::open` ever runs) makes the v4 `CREATE TABLE` fail
/// (no `IF NOT EXISTS` on the versioned DDL, unlike the base `schema.sql`).
/// The whole migration must roll back: schema_version stays 3, no
/// recurring table survives half-created, and a later open (once the
/// conflicting table is gone) still reaches v4 cleanly.
#[test]
fn m02_a_forced_ddl_failure_leaves_the_old_version_and_data_unchanged() {
    let db = TempDb::with_v3("m02");
    {
        let raw = Connection::open(&db.0).unwrap();
        raw.execute_batch("CREATE TABLE recurring_decisions (oops INTEGER);").unwrap();
    }

    let failed = Store::open(&db.0);
    assert!(failed.is_err(), "the v4 CREATE TABLE must fail against the pre-existing conflicting table");

    let raw = Connection::open(&db.0).unwrap();
    let version: String = raw.query_row("SELECT value FROM settings WHERE key = 'schema_version'", [], |r| r.get(0)).unwrap();
    assert_eq!(version, "3", "a rolled-back migration must not bump schema_version");
    let members_exists: i64 = raw.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = 'recurring_members'", [], |r| r.get(0)).unwrap();
    assert_eq!(members_exists, 0, "recurring_members must not have been created half-way through a rolled-back transaction");
    let row_count: i64 = raw.query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0)).unwrap();
    assert_eq!(row_count, 1, "old data must be completely unchanged by the failed attempt");
    drop(raw);

    // Remove the conflicting table (a real repair a user would need to do
    // manually is out of scope; this proves the rollback left SQLite in a
    // clean, reopenable state, not a corrupted one).
    let raw = Connection::open(&db.0).unwrap();
    raw.execute_batch("DROP TABLE recurring_decisions;").unwrap();
    drop(raw);
    let s = Store::open(&db.0).unwrap();
    assert_eq!(s.list_transactions(&store::TxFilter::default()).unwrap().len(), 1, "a later, unblocked open must still reach v4 with every row intact");
}

#[test]
fn a_deferred_commit_failure_rolls_back_the_version_tables_and_data() {
    let db = TempDb::with_v3("commit-failure");
    {
        let raw = Connection::open(&db.0).unwrap();
        raw.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE commit_parent (id INTEGER PRIMARY KEY);
             CREATE TABLE commit_guard (parent_id INTEGER REFERENCES commit_parent(id) DEFERRABLE INITIALLY DEFERRED);
             CREATE TRIGGER fail_schema_commit AFTER UPDATE OF value ON settings
             WHEN NEW.key = 'schema_version'
             BEGIN
               INSERT INTO commit_guard(parent_id) VALUES (999);
             END;",
        )
        .unwrap();
    }

    assert!(Store::open(&db.0).is_err(), "the deferred foreign-key violation must fail COMMIT");

    let raw = Connection::open(&db.0).unwrap();
    let version: String = raw.query_row("SELECT value FROM settings WHERE key = 'schema_version'", [], |row| row.get(0)).unwrap();
    let recurring_tables: i64 = raw
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('recurring_decisions','recurring_members')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let note: String = raw.query_row("SELECT note FROM transactions WHERE id = 1", [], |row| row.get(0)).unwrap();
    assert_eq!(version, "3");
    assert_eq!(recurring_tables, 0);
    assert_eq!(note, "poznamka");
}

#[test]
fn an_existing_minimal_database_is_migrated_without_a_full_reseed() {
    let db = TempDb::with_v3("minimal-no-reseed");
    {
        let raw = Connection::open(&db.0).unwrap();
        raw.execute_batch("DELETE FROM transactions; DELETE FROM categories;").unwrap();
    }

    let s = Store::open(&db.0).unwrap();

    assert!(s.list_categories().unwrap().is_empty());
    assert!(s.list_rules().unwrap().is_empty());
}

#[test]
fn v4_migration_runs_the_exact_spotify_repair_and_preserves_confirmed_rows() {
    let db = TempDb::with_v3("spotify-migration");
    {
        let raw = Connection::open(&db.0).unwrap();
        raw.execute_batch(
            "INSERT INTO categories (id, parent_id, name, kind) VALUES (10, NULL, 'Predplatné', 'expense');
             INSERT INTO categories (id, parent_id, name, kind) VALUES (11, 10, 'Apple', 'expense');
             INSERT INTO rules (id, match_kind, key, place, category_id) VALUES (20, 'seed', 'spotify', '', 11);
             UPDATE transactions SET merchant_raw='SPOTIFY', merchant_norm='spotify', category_id=11, status='suggested', rule_id=20, source='seed' WHERE id=1;
             INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, category_id, status, rule_id, source, note)
             VALUES (2, 1, 1, 'fp-confirmed', '2026-06-02', '2026-06-02', 'card', -2100, 'SPOTIFY', 'spotify', 'raw confirmed', 11, 'confirmed', 20, 'seed', 'keep me');",
        )
        .unwrap();
    }

    let s = Store::open(&db.0).unwrap();
    let spotify = s.category_by_path("Predplatné/Spotify").unwrap().unwrap();
    let rows = s.list_transactions(&store::TxFilter::default()).unwrap();
    let open = rows.iter().find(|row| row.id == 1).unwrap();
    let confirmed = rows.iter().find(|row| row.id == 2).unwrap();
    assert_eq!(open.category_id, Some(spotify));
    assert_eq!(confirmed.category_id, Some(11));
    assert_eq!(confirmed.note, "keep me");
}

/// M02b: a database written by a future, unsupported schema version is
/// refused outright, never silently downgraded.
#[test]
fn a_future_schema_version_is_refused_without_downgrading_its_marker() {
    let db = TempDb::with_v3("future");
    {
        let raw = Connection::open(&db.0).unwrap();
        raw.execute("UPDATE settings SET value = '99' WHERE key = 'schema_version'", []).unwrap();
    }
    let err = Store::open(&db.0);
    assert!(err.is_err());
    let raw = Connection::open(&db.0).unwrap();
    let version: String = raw.query_row("SELECT value FROM settings WHERE key = 'schema_version'", [], |r| r.get(0)).unwrap();
    assert_eq!(version, "99", "a refused future version must not be downgraded");
}

/// M03: a WAL-backed SQLite snapshot carries both scopes, membership and
/// unrelated v3 data. Later source mutations cannot change the destination.
#[test]
fn m03_backup_carries_recurring_decisions_and_ignored_items() {
    let db = TempDb::with_v3("m03");
    {
        let raw = Connection::open(&db.0).unwrap();
        raw.execute_batch(
            "PRAGMA journal_mode = WAL;
             INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source, note)
             VALUES (2, 1, 1, 'fp-2', '2026-06-15', '2026-06-15', 'card', -3000, 'POISTENIE', 'poistenie', 'raw two', 'unassigned', 'none', 'druha poznamka');",
        )
        .unwrap();
    }
    let mut s = Store::open(&db.0).unwrap();
    let group = s
        .save_recurring(&SaveRecurringRequest {
            decision_id: None,
            selection: RecurringSelection::Group { transaction_id: 1 },
            decision: RecurringDecisionInput::Confirmed { cadence: Cadence::Monthly, anchor_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap() },
        })
        .unwrap();
    let selected = s
        .save_recurring(&SaveRecurringRequest {
            decision_id: None,
            selection: RecurringSelection::Selected { transaction_ids: vec![2] },
            decision: RecurringDecisionInput::Ignored,
        })
        .unwrap();
    let rule = s.insert_rule(rules::RuleKind::Exact, "aldi", None, 2).unwrap();
    s.record_rule_source(rule, 1).unwrap();
    let snapshot_note = s.list_transactions(&store::TxFilter::default()).unwrap().into_iter().find(|row| row.id == 1).unwrap().note;

    let dest = std::env::temp_dir().join(format!("abakus-recurring-migration-m03-backup-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&dest);
    s.backup_to(&dest).unwrap();

    s.save_transaction_note(1, "source changed after snapshot").unwrap();
    s.reset_recurring(group.id).unwrap();
    s.set_check_updates(false).unwrap();

    let backup = Store::open(&dest).unwrap();
    let raw = Connection::open(&dest).unwrap();
    let integrity: String = raw.query_row("PRAGMA integrity_check", [], |row| row.get(0)).unwrap();
    let foreign_key_errors: i64 = raw.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| row.get(0)).unwrap();
    let (mode, cadence): (String, String) = raw.query_row("SELECT mode, cadence FROM recurring_decisions WHERE id = ?1", [group.id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    let (selected_mode, selected_scope): (String, String) = raw.query_row("SELECT mode, scope FROM recurring_decisions WHERE id = ?1", [selected.id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    let member: String = raw.query_row("SELECT fingerprint FROM recurring_members WHERE decision_id = ?1", [selected.id], |row| row.get(0)).unwrap();
    let backed_up_note: String = raw.query_row("SELECT note FROM transactions WHERE id = 1", [], |row| row.get(0)).unwrap();
    let provenance: i64 = raw.query_row("SELECT COUNT(*) FROM rule_sources WHERE rule_id = ?1 AND transaction_id = 1", [rule], |row| row.get(0)).unwrap();
    let version: String = raw.query_row("SELECT value FROM settings WHERE key = 'schema_version'", [], |row| row.get(0)).unwrap();
    let check_updates: String = raw.query_row("SELECT value FROM settings WHERE key = 'check_updates'", [], |row| row.get(0)).unwrap();
    assert_eq!(integrity, "ok");
    assert_eq!(foreign_key_errors, 0);
    assert_eq!((mode.as_str(), cadence.as_str()), ("confirmed", "monthly"));
    assert_eq!((selected_mode.as_str(), selected_scope.as_str()), ("ignored", "selected"));
    assert_eq!(member, "fp-2");
    assert_eq!(backed_up_note, snapshot_note);
    assert_eq!(provenance, 1);
    assert_eq!((version.as_str(), check_updates.as_str()), ("4", "1"));
    assert_eq!(backup.list_transactions(&store::TxFilter::default()).unwrap().len(), 2);
    assert!(s.list_transactions(&store::TxFilter::default()).unwrap().iter().any(|row| row.note == "source changed after snapshot"), "the source remains usable after backup");
    let _ = std::fs::remove_file(&dest);
}
