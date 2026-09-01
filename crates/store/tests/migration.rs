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
    fn with_old_schema(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-migration-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch(OLD_DATA).unwrap();
        TempDb(path)
    }
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
