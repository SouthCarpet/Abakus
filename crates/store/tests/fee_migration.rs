use chrono::NaiveDate;
use parser::{AccountKind, Statement, Transaction, TxKind};
use rusqlite::types::Value;
use rusqlite::Connection;
use store::{Store, TxFilter};

const PERSONAL_IBAN: &str = "SK4411000000000012345678";
const BUSINESS_IBAN: &str = "SK3711000000000098765432";

struct TempDb(std::path::PathBuf);

impl TempDb {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-fee-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        Self(path)
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn row(day: u32, kind: TxKind, amount_cents: i64, merchant: &str, raw_block: &str) -> Transaction {
    let date = NaiveDate::from_ymd_opt(2026, 6, day).unwrap();
    let mut row = Transaction::blank(date, amount_cents, kind, raw_block.to_string());
    row.merchant_raw = merchant.to_string();
    row
}

fn legacy_fee_database(name: &str) -> TempDb {
    let database = TempDb::new(name);
    {
        let mut store = Store::open(&database.0).unwrap();
        store.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
        store.upsert_account(BUSINESS_IBAN, AccountKind::Business, "Firemný").unwrap();
        let mut internal = row(
            3,
            TxKind::TransferOut,
            -5_000,
            "Vlastný prevod",
            "03.06.2026    Poplatok za účet                       50.00-",
        );
        internal.counterparty_iban = Some(BUSINESS_IBAN.to_string());
        let statement = Statement {
            iban: PERSONAL_IBAN.into(),
            account_kind: AccountKind::Personal,
            number: 7,
            period_start: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            period_end: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            opening_cents: None,
            closing_cents: None,
            transactions: vec![
                row(1, TxKind::Fee, -250, "Poplatok za vedenie účtu", "01.06.2026    Poplatok za vedenie účtu                       2.50-"),
                row(2, TxKind::Other, -100, "Neznámy bankový záznam", "02.06.2026    Neznámy bankový záznam                       1.00-"),
                internal,
            ],
            warnings: Vec::new(),
        };
        store.import_statement(&statement, "legacy-fee").unwrap();
        let fee = store.list_transactions(&TxFilter { kind: Some(TxKind::Fee), ..Default::default() }).unwrap()[0].id;
        let category = store.category_by_path("Poplatky/banka").unwrap().unwrap();
        store.assign(&[fee], category, false).unwrap();
        store.save_transaction_note(fee, "zachovať presne").unwrap();
    }
    let connection = Connection::open(&database.0).unwrap();
    connection.execute("UPDATE transactions SET kind = 'other'", []).unwrap();
    connection.execute("UPDATE settings SET value = '4' WHERE key = 'schema_version'", []).unwrap();
    drop(connection);
    database
}

fn transaction_rows(connection: &Connection) -> Vec<Vec<Value>> {
    let mut statement = connection.prepare("SELECT * FROM transactions ORDER BY id").unwrap();
    let column_count = statement.column_count();
    statement
        .query_map([], |row| (0..column_count).map(|index| row.get(index)).collect::<rusqlite::Result<Vec<Value>>>())
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

#[test]
fn v5_backfills_only_recognized_non_transfer_fees_and_preserves_every_other_column() {
    let database = legacy_fee_database("backfill");
    let before = transaction_rows(&Connection::open(&database.0).unwrap());

    let store = Store::open(&database.0).unwrap();
    let rows = store.list_transactions(&TxFilter::default()).unwrap();
    assert_eq!(rows.iter().find(|row| row.merchant_raw == "Poplatok za vedenie účtu").unwrap().kind, "fee");
    assert_eq!(rows.iter().find(|row| row.merchant_raw == "Neznámy bankový záznam").unwrap().kind, "other");
    assert_eq!(rows.iter().find(|row| row.merchant_raw == "Vlastný prevod").unwrap().kind, "other", "a transfer status blocks fee backfill even when its raw description looks like a fee");
    drop(store);

    let connection = Connection::open(&database.0).unwrap();
    let mut after = transaction_rows(&connection);
    assert_eq!(after[0][6], Value::Text("fee".into()));
    after[0][6] = Value::Text("other".into());
    assert_eq!(after, before, "ID, fingerprint, category, status, rule, note, amount and every other column stay byte-for-byte equivalent");
    let version: String = connection.query_row("SELECT value FROM settings WHERE key = 'schema_version'", [], |row| row.get(0)).unwrap();
    assert_eq!(version, "6");
    drop(connection);

    let reopened = Store::open(&database.0).unwrap();
    drop(reopened);
    let reopened_rows = transaction_rows(&Connection::open(&database.0).unwrap());
    assert_eq!(reopened_rows[0][6], Value::Text("fee".into()), "reopening is idempotent");
    after[0][6] = Value::Text("fee".into());
    assert_eq!(reopened_rows, after, "reopening preserves every stored column");
}

#[test]
fn a_failed_fee_backfill_rolls_back_rows_and_version_then_retries_cleanly() {
    let database = legacy_fee_database("atomic");
    let connection = Connection::open(&database.0).unwrap();
    // Plan 091 atomicity oracle: one earlier fee write must be undone when
    // a later eligible row fails. Reuse the synthetic unknown row as that fee.
    connection.execute("UPDATE transactions SET raw_block = '02.06.2026    Poplatok za účet                       1.00-' WHERE id = 2", []).unwrap();
    let before = transaction_rows(&connection);
    connection.execute_batch("CREATE TRIGGER reject_fee_update BEFORE UPDATE OF kind ON transactions WHEN NEW.id = 2 AND EXISTS (SELECT 1 FROM transactions WHERE id = 1 AND kind = 'fee') BEGIN SELECT RAISE(ABORT, 'synthetic fee migration failure'); END;").unwrap();
    drop(connection);

    assert!(Store::open(&database.0).is_err(), "the synthetic database write failure must reach the migration boundary");
    let connection = Connection::open(&database.0).unwrap();
    assert_eq!(transaction_rows(&connection), before, "a failed migration leaves every transaction untouched");
    let version: String = connection.query_row("SELECT value FROM settings WHERE key = 'schema_version'", [], |row| row.get(0)).unwrap();
    assert_eq!(version, "4", "a failed migration does not advertise version 5");
    connection.execute_batch("DROP TRIGGER reject_fee_update;").unwrap();
    drop(connection);

    let store = Store::open(&database.0).unwrap();
    let fee = store.list_transactions(&TxFilter { kind: Some(TxKind::Fee), ..Default::default() }).unwrap();
    assert_eq!(fee.len(), 2, "both recognized rows migrate after the write failure is removed");
}
