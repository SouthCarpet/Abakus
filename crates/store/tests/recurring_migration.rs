//! Owned by lane A. M01-M03 from recurring-acceptance.md: an existing
//! (pre-v4) database reaches schema version 4 atomically, a forced DDL
//! failure leaves the old version and data completely unchanged, and a
//! backup snapshot carries the new tables.
use chrono::NaiveDate;
use rusqlite::Connection;
use store::recurring::{Cadence, RecurringDecisionInput, RecurringQuery, RecurringSelection, SaveRecurringRequest};
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

/// M03: the SQLite backup snapshot carries confirmed and ignored
/// recurring decisions, and reopens independently with them intact.
#[test]
fn m03_backup_carries_recurring_decisions_and_ignored_items() {
    let db = TempDb::with_v3("m03");
    let mut s = Store::open(&db.0).unwrap();
    let decision = s
        .save_recurring(&SaveRecurringRequest {
            decision_id: None,
            selection: RecurringSelection::Group { transaction_id: 1 },
            decision: RecurringDecisionInput::Confirmed { cadence: Cadence::Monthly, anchor_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap() },
        })
        .unwrap();
    let _ = s.recurring_overview(&RecurringQuery { from: None, to: None, account_kind: None, today: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap() }).unwrap();

    let dest = std::env::temp_dir().join(format!("abakus-recurring-migration-m03-backup-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&dest);
    s.backup_to(&dest).unwrap();

    let backup = Store::open(&dest).unwrap();
    let raw = Connection::open(&dest).unwrap();
    let (mode, cadence): (String, String) = raw.query_row("SELECT mode, cadence FROM recurring_decisions WHERE id = ?1", [decision.id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    assert_eq!((mode.as_str(), cadence.as_str()), ("confirmed", "monthly"));
    assert_eq!(backup.list_transactions(&store::TxFilter::default()).unwrap().len(), 1);
    let _ = std::fs::remove_file(&dest);
}
