use chrono::NaiveDate;
use parser::{AccountKind, Statement, Transaction, TxKind};
use rusqlite::{types::Value, Connection};
use store::Store;

pub const IBAN: &str = "SK4411000000000012345678";
pub const OTHER_IBAN: &str = "SK3711000000000098765432";

pub struct Database(pub std::path::PathBuf);

impl Database {
    pub fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-review21-{name}-{}.db", std::process::id()));
        assert!(!path.exists(), "each synthetic test owns a fresh database");
        Self(path)
    }

    pub fn open(&self) -> Store { Store::open(&self.0).unwrap() }
    pub fn raw(&self) -> Connection { Connection::open(&self.0).unwrap() }
}

impl Drop for Database {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

pub fn statement() -> Statement {
    let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
    Statement {
        iban: IBAN.into(), account_kind: AccountKind::Personal, number: 1,
        period_start: date, period_end: date, opening_cents: Some(1000),
        closing_cents: Some(1000), transactions: vec![], warnings: vec![],
    }
}

pub fn transaction(merchant: &str) -> Transaction {
    let mut row = Transaction::blank(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(), 0, TxKind::Card, merchant.into());
    row.merchant_raw = merchant.into();
    row
}

pub fn account(store: &mut Store) { store.upsert_account(IBAN, AccountKind::Personal, "Kontrola").unwrap(); }

pub fn rows(connection: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut query = connection.prepare(sql).unwrap();
    let columns = query.column_count();
    query.query_map([], |row| (0..columns).map(|index| row.get(index)).collect::<rusqlite::Result<Vec<Value>>>())
        .unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap()
}
