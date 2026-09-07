//! A17/F1 migration discipline: the app has a live database on the user's
//! machine (`%LOCALAPPDATA%\Abakus\abakus.db`). Everything here starts from a
//! file written by the OLD schema, never from a fresh one.
use rules::RuleKind;
use store::Store;

/// `crates/store/src/schema.sql` exactly as it stood before A17 (commit
/// fc7a649): no `rule_sources`, no `schema_version`. Kept verbatim on purpose,
/// so this test keeps testing the real old shape even after schema.sql moves on.
const SCHEMA_V1: &str = "\
CREATE TABLE IF NOT EXISTS accounts (id INTEGER PRIMARY KEY, iban TEXT NOT NULL UNIQUE, kind TEXT NOT NULL CHECK (kind IN ('personal','business')), label TEXT NOT NULL DEFAULT '', has_password INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS statements (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id), number INTEGER NOT NULL, period_start TEXT NOT NULL, period_end TEXT NOT NULL, opening_cents INTEGER, closing_cents INTEGER, checksum_status TEXT NOT NULL, checksum_off_by INTEGER, file_hash TEXT NOT NULL UNIQUE, imported_at TEXT NOT NULL DEFAULT (datetime('now')));
CREATE TABLE IF NOT EXISTS categories (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES categories(id), name TEXT NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('expense','income')), sort INTEGER NOT NULL DEFAULT 0, system INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0, UNIQUE (parent_id, name));
CREATE TABLE IF NOT EXISTS rules (id INTEGER PRIMARY KEY, match_kind TEXT NOT NULL CHECK (match_kind IN ('exact','merchant','counterparty_account','seed')), key TEXT NOT NULL, place TEXT NOT NULL DEFAULT '', category_id INTEGER NOT NULL REFERENCES categories(id), created_at TEXT NOT NULL DEFAULT (datetime('now')), hit_count INTEGER NOT NULL DEFAULT 0, UNIQUE (match_kind, key, place));
CREATE TABLE IF NOT EXISTS transactions (id INTEGER PRIMARY KEY, statement_id INTEGER NOT NULL REFERENCES statements(id), account_id INTEGER NOT NULL REFERENCES accounts(id), fingerprint TEXT NOT NULL UNIQUE, posted_date TEXT NOT NULL, tx_date TEXT NOT NULL, kind TEXT NOT NULL, amount_cents INTEGER NOT NULL, orig_amount_cents INTEGER, orig_currency TEXT, rate_micros INTEGER, merchant_raw TEXT NOT NULL, merchant_norm TEXT NOT NULL, place TEXT, place_norm TEXT, counterparty_name TEXT, counterparty_iban TEXT, reference TEXT, card_last4 TEXT, raw_block TEXT NOT NULL, category_id INTEGER REFERENCES categories(id), status TEXT NOT NULL CHECK (status IN ('transfer','confirmed','suggested','unassigned')), rule_id INTEGER REFERENCES rules(id), source TEXT NOT NULL DEFAULT 'none');
CREATE INDEX IF NOT EXISTS tx_date_idx ON transactions(tx_date);
CREATE INDEX IF NOT EXISTS tx_status_idx ON transactions(status);
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE UNIQUE INDEX IF NOT EXISTS categories_top_uniq ON categories(name) WHERE parent_id IS NULL;
CREATE TABLE IF NOT EXISTS net_log (id INTEGER PRIMARY KEY, started_at TEXT NOT NULL, url TEXT NOT NULL, status TEXT NOT NULL, duration_ms INTEGER NOT NULL, bytes_in INTEGER NOT NULL);
";

/// A small but complete pre-A17 database: one account, one statement, three
/// transactions, a seed rule, a learned exact rule that one row uses, and a
/// learned merchant rule that NO row points at (the shape a pre-A17 assignment
/// leaves behind), plus a user setting.
const OLD_DATA: &str = "\
INSERT INTO accounts (id, iban, kind, label) VALUES (1, 'SK4411000000000012345678', 'personal', 'Osobný');
INSERT INTO statements (id, account_id, number, period_start, period_end, opening_cents, closing_cents, checksum_status, file_hash) VALUES (1, 1, 6, '2026-06-01', '2026-06-30', 69392, 42424, 'ok', 'old-hash');
INSERT INTO categories (id, parent_id, name, kind) VALUES (1, NULL, 'Jedlo', 'expense');
INSERT INTO categories (id, parent_id, name, kind) VALUES (2, 1, 'potraviny', 'expense');
INSERT INTO rules (id, match_kind, key, place, category_id, hit_count) VALUES (1, 'seed', 'aldi', '', 2, 3);
INSERT INTO rules (id, match_kind, key, place, category_id, hit_count) VALUES (2, 'exact', 'bauhaus', 'neuss', 2, 1);
INSERT INTO rules (id, match_kind, key, place, category_id, hit_count) VALUES (3, 'merchant', 'bauhaus', '', 2, 0);
INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, place, place_norm, raw_block, category_id, status, rule_id, source) VALUES (1, 1, 1, 'fp-1', '2026-06-01', '2026-06-01', 'card', -2000, 'BAUHAUS', 'bauhaus', 'Neuss', 'neuss', 'raw 1', 2, 'confirmed', 2, 'exact_rule');
INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, place, place_norm, raw_block, category_id, status, rule_id, source) VALUES (2, 1, 1, 'fp-2', '2026-06-02', '2026-06-02', 'card', -199, 'ALDI SUED', 'aldi sued', 'Neuss', 'neuss', 'raw 2', 2, 'suggested', 1, 'seed');
INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, place, place_norm, raw_block, status, source) VALUES (3, 1, 1, 'fp-3', '2026-06-03', '2026-06-03', 'card', -500, 'OF', 'of', 'London', 'london', 'raw 3', 'unassigned', 'none');
INSERT INTO settings (key, value) VALUES ('check_updates', '1');
";

struct TempDb(std::path::PathBuf);
impl TempDb {
    fn with_schema(name: &str, schema: &str, data: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-migration-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(schema).unwrap();
        conn.execute_batch(data).unwrap();
        TempDb(path)
    }
    fn with_old_schema(name: &str) -> Self { Self::with_schema(name, SCHEMA_V1, OLD_DATA) }
}
impl Drop for TempDb {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

/// The migration must run against a file that already exists, and it must not
/// cost the user anything: every row is still there afterwards.
#[test]
fn opening_an_old_database_keeps_every_row_and_reaches_the_current_schema() {
    let db = TempDb::with_old_schema("survives");

    let s = Store::open(&db.0).unwrap();

    assert_eq!(s.statement_count().unwrap(), 1);
    assert_eq!(s.list_transactions(&store::TxFilter::default()).unwrap().len(), 3);
    assert_eq!(s.list_accounts().unwrap()[0].label, "Osobný");
    assert!(s.get_check_updates().unwrap(), "a user setting survives the migration");
    assert_eq!(s.list_categories().unwrap().len(), 2, "seeding must not re-run over an existing category tree");
    assert_eq!(s.list_rules().unwrap().len(), 3, "no rule is created or dropped by the migration");
}

/// Provenance is backfilled from the only evidence an old database has: which
/// transaction points at which learned rule. A learned rule nothing points at
/// keeps an unknown origin, and unknown origin never authorizes a delete.
#[test]
fn the_migration_backfills_provenance_and_a_delete_then_respects_it() {
    let db = TempDb::with_old_schema("backfill");
    let mut s = Store::open(&db.0).unwrap();

    let outcome = s.delete_statement(1).unwrap();

    assert_eq!(outcome.transactions_deleted, 3);
    assert_eq!(outcome.rules_deleted, 1, "only the learned rule whose row was backfilled");
    let rules = s.list_rules().unwrap();
    assert!(!rules.iter().any(|r| r.kind == RuleKind::Exact), "the backfilled exact rule loses its last row and goes");
    assert!(rules.iter().any(|r| r.kind == RuleKind::Seed), "seed rules are never deleted");
    assert!(rules.iter().any(|r| r.kind == RuleKind::Merchant), "a rule with no known origin is never deleted");
    assert_eq!(s.statement_count().unwrap(), 0);
}

/// A statement of the account the old database already holds, whose merchant
/// the old merchant rule (id 3, no provenance) classifies.
const LATER_BAUHAUS: &str = "\
Osobný účet     SK44 1100 0000 0000 1234 5678          Mena  EUR                          BIC (SWIFT)   TATRSKBX
IBAN SK44 1100 0000 0000 1234 5678
Číslo klienta:  1234567
Majiteľ účtu:   JANA VZOROVÁ
Tatra banka, a.s., Hodžovo nám. 3
811 06 Bratislava
Dialog  0800 00 1100              ID:   00                                    Výpis číslo:        9
Osobný účet     SK44 1100 0000 0000 1234 5678     Majiteľ Jana Vzorová                  Dátum 30.09.2026
Dátum sprac.  Popis                                     Dátum zúčt.                              Suma
--------------------------------------------------------------------------------------------------
              Posledný výpis  31.08.2026                                                       500.00
01.09.2026    EUR AP nákup POS                                                                  20.00-
              Miesto platby:    Erfttal               BAUHAUS
              Dátum:  01.09.26  Čas:  12:00:00        Suma:          20.00- EUR
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                                       480.00
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        9        Strana:        1
";

/// `crates/store/src/schema.sql` exactly as it stood at schema version 2
/// (before 0.1.2's `transactions.note` column): `rule_sources` and its
/// provenance already exist, but `note` does not.
const SCHEMA_V2: &str = "\
CREATE TABLE IF NOT EXISTS accounts (id INTEGER PRIMARY KEY, iban TEXT NOT NULL UNIQUE, kind TEXT NOT NULL CHECK (kind IN ('personal','business')), label TEXT NOT NULL DEFAULT '', has_password INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS statements (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id), number INTEGER NOT NULL, period_start TEXT NOT NULL, period_end TEXT NOT NULL, opening_cents INTEGER, closing_cents INTEGER, checksum_status TEXT NOT NULL, checksum_off_by INTEGER, file_hash TEXT NOT NULL UNIQUE, imported_at TEXT NOT NULL DEFAULT (datetime('now')));
CREATE TABLE IF NOT EXISTS categories (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES categories(id), name TEXT NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('expense','income')), sort INTEGER NOT NULL DEFAULT 0, system INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0, UNIQUE (parent_id, name));
CREATE TABLE IF NOT EXISTS rules (id INTEGER PRIMARY KEY, match_kind TEXT NOT NULL CHECK (match_kind IN ('exact','merchant','counterparty_account','seed')), key TEXT NOT NULL, place TEXT NOT NULL DEFAULT '', category_id INTEGER NOT NULL REFERENCES categories(id), created_at TEXT NOT NULL DEFAULT (datetime('now')), hit_count INTEGER NOT NULL DEFAULT 0, UNIQUE (match_kind, key, place));
CREATE TABLE IF NOT EXISTS transactions (id INTEGER PRIMARY KEY, statement_id INTEGER NOT NULL REFERENCES statements(id), account_id INTEGER NOT NULL REFERENCES accounts(id), fingerprint TEXT NOT NULL UNIQUE, posted_date TEXT NOT NULL, tx_date TEXT NOT NULL, kind TEXT NOT NULL, amount_cents INTEGER NOT NULL, orig_amount_cents INTEGER, orig_currency TEXT, rate_micros INTEGER, merchant_raw TEXT NOT NULL, merchant_norm TEXT NOT NULL, place TEXT, place_norm TEXT, counterparty_name TEXT, counterparty_iban TEXT, reference TEXT, card_last4 TEXT, raw_block TEXT NOT NULL, category_id INTEGER REFERENCES categories(id), status TEXT NOT NULL CHECK (status IN ('transfer','confirmed','suggested','unassigned')), rule_id INTEGER REFERENCES rules(id), source TEXT NOT NULL DEFAULT 'none');
CREATE INDEX IF NOT EXISTS tx_date_idx ON transactions(tx_date);
CREATE INDEX IF NOT EXISTS tx_status_idx ON transactions(status);
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE UNIQUE INDEX IF NOT EXISTS categories_top_uniq ON categories(name) WHERE parent_id IS NULL;
CREATE TABLE IF NOT EXISTS rule_sources (
  rule_id INTEGER NOT NULL REFERENCES rules(id) ON DELETE CASCADE,
  transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
  statement_id INTEGER NOT NULL REFERENCES statements(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (rule_id, transaction_id)
);
CREATE INDEX IF NOT EXISTS rule_sources_stmt_idx ON rule_sources(statement_id);
CREATE TABLE IF NOT EXISTS net_log (id INTEGER PRIMARY KEY, started_at TEXT NOT NULL, url TEXT NOT NULL, status TEXT NOT NULL, duration_ms INTEGER NOT NULL, bytes_in INTEGER NOT NULL);
";

/// A small but complete schema-version-2 database: one account, one
/// statement, three transactions (confirmed via a REAL learned exact rule
/// with a genuine `rule_sources` row, a suggested seed match, and an
/// unassigned row), and a user setting, plus the `schema_version` row itself
/// (the A17 backfill already ran on this database, unlike `OLD_DATA`'s v1
/// shape).
const OLD_DATA_V2: &str = "\
INSERT INTO accounts (id, iban, kind, label) VALUES (1, 'SK4411000000000012345678', 'personal', 'Osobný');
INSERT INTO statements (id, account_id, number, period_start, period_end, opening_cents, closing_cents, checksum_status, file_hash) VALUES (1, 1, 6, '2026-06-01', '2026-06-30', 69392, 42424, 'ok', 'v2-hash');
INSERT INTO categories (id, parent_id, name, kind) VALUES (1, NULL, 'Jedlo', 'expense');
INSERT INTO categories (id, parent_id, name, kind) VALUES (2, 1, 'potraviny', 'expense');
INSERT INTO rules (id, match_kind, key, place, category_id, hit_count) VALUES (1, 'seed', 'aldi', '', 2, 1);
INSERT INTO rules (id, match_kind, key, place, category_id, hit_count) VALUES (2, 'exact', 'bauhaus', 'neuss', 2, 1);
INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, place, place_norm, raw_block, category_id, status, rule_id, source) VALUES (1, 1, 1, 'fp-1', '2026-06-01', '2026-06-01', 'card', -2000, 'BAUHAUS', 'bauhaus', 'Neuss', 'neuss', 'raw 1', 2, 'confirmed', 2, 'exact_rule');
INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, place, place_norm, raw_block, category_id, status, rule_id, source) VALUES (2, 1, 1, 'fp-2', '2026-06-02', '2026-06-02', 'card', -199, 'ALDI SUED', 'aldi sued', 'Neuss', 'neuss', 'raw 2', 2, 'suggested', 1, 'seed');
INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, place, place_norm, raw_block, status, source) VALUES (3, 1, 1, 'fp-3', '2026-06-03', '2026-06-03', 'card', -500, 'OF', 'of', 'London', 'london', 'raw 3', 'unassigned', 'none');
INSERT INTO rule_sources (rule_id, transaction_id, statement_id) VALUES (2, 1, 1);
INSERT INTO settings (key, value) VALUES ('check_updates', '1');
INSERT INTO settings (key, value) VALUES ('schema_version', '2');
";

/// 0.1.2: a database already at version 2 (the A17 backfill long done, a REAL
/// provenance row already in place) must reach version 3 by gaining
/// `transactions.note`, nothing else. Every row, status and the existing
/// provenance survive untouched.
#[test]
fn a_v2_database_gains_the_notes_column_and_keeps_every_row_status_and_provenance() {
    let db = TempDb::with_schema("v2-to-v3", SCHEMA_V2, OLD_DATA_V2);

    let mut s = Store::open(&db.0).unwrap();

    let rows = s.list_transactions(&store::TxFilter::default()).unwrap();
    assert_eq!(rows.len(), 3, "no row is created or dropped by the notes migration");
    assert!(rows.iter().all(|r| r.note.is_empty()), "every legacy row migrates to an empty note");
    let bauhaus = rows.iter().find(|r| r.merchant_raw == "BAUHAUS").unwrap();
    assert_eq!(bauhaus.status, rules::Status::Confirmed, "statuses survive the migration untouched");
    assert_eq!(s.statement_delete_preview(1).unwrap().rules_deleted, 1, "the real rule_sources provenance from before the migration is still respected");
    assert!(s.get_check_updates().unwrap(), "a user setting survives");

    // And it is a real, writable column, not just present in the schema.
    s.save_transaction_note(bauhaus.id, "funguje aj po migrácii").unwrap();
    assert_eq!(s.list_transactions(&store::TxFilter::default()).unwrap().iter().find(|r| r.id == bauhaus.id).unwrap().note, "funguje aj po migrácii");
}

/// Reopening an already-current database must not re-run any migration step
/// (an `ALTER TABLE ADD COLUMN note` a second time would itself error) and
/// must not perturb any data.
#[test]
fn reopening_an_up_to_date_database_is_a_no_op_and_stays_idempotent() {
    let db = TempDb::with_old_schema("reopen");
    let id = { let mut s = Store::open(&db.0).unwrap(); let id = s.list_transactions(&store::TxFilter::default()).unwrap()[0].id; s.save_transaction_note(id, "prežije reopen").unwrap(); id };

    for _ in 0..3 {
        let s = Store::open(&db.0).unwrap();
        assert_eq!(s.list_transactions(&store::TxFilter::default()).unwrap().iter().find(|r| r.id == id).unwrap().note, "prežije reopen");
    }
}

/// `Store::migrate` wraps every step in one transaction (0.1.2 correction: it
/// did not before). A writer already holding the file's write lock makes
/// `Store::open` fail outright instead of completing the schema/PRAGMA/
/// migrate sequence halfway; once the lock is released, a fresh open still
/// reaches the current version with every row intact, proving the failed
/// attempt left nothing partially applied.
#[test]
fn a_locked_database_fails_the_open_cleanly_and_a_later_open_still_completes_it() {
    let db = TempDb::with_old_schema("locked");
    let blocker = rusqlite::Connection::open(&db.0).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE; UPDATE settings SET value = value WHERE key = 'check_updates';").unwrap();

    let failed = Store::open(&db.0);
    assert!(failed.is_err(), "opening a file another connection holds an exclusive write lock on must fail, not silently skip the migration");

    blocker.execute_batch("COMMIT").unwrap();
    drop(blocker);

    let mut s = Store::open(&db.0).unwrap();
    let rows = s.list_transactions(&store::TxFilter::default()).unwrap();
    assert_eq!(rows.len(), 3, "no data lost across the failed-then-retried open");
    let id = rows[0].id;
    s.save_transaction_note(id, "stĺpec note existuje po neúspešnom pokuse").unwrap();
    assert_eq!(s.list_transactions(&store::TxFilter::default()).unwrap().iter().find(|r| r.id == id).unwrap().note, "stĺpec note existuje po neúspešnom pokuse", "a later, unblocked open must still reach the current version (the notes column really is writable)");
}

/// The backfill reconstructs provenance from incomplete evidence, so it must
/// run once and never again. A second run would read "this row is classified
/// by that rule" as "this statement produced that rule" for data written after
/// the migration, and a later delete would then take a rule it never produced.
#[test]
fn the_backfill_runs_once_so_a_later_import_never_inherits_provenance() {
    let db = TempDb::with_old_schema("once");
    let later = {
        let mut s = Store::open(&db.0).unwrap();
        s.import_statement(&parser::parse_text(LATER_BAUHAUS).unwrap(), "later-hash").unwrap().statement_id
    };

    // The reopen is where a backfill without the version guard would fire again.
    let mut s = Store::open(&db.0).unwrap();
    assert_eq!(s.statement_delete_preview(later).unwrap().rules_deleted, 0, "the new statement used the old rule, it did not produce it");

    s.delete_statement(later).unwrap();

    assert!(s.list_rules().unwrap().iter().any(|r| r.kind == RuleKind::Merchant && r.key == "bauhaus"), "a rule of unknown origin survives a statement that merely used it");
}
