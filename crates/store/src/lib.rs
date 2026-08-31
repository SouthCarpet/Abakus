//! SQLite store: accounts, statements, transactions, categories, rules, summaries.
pub mod accounts;
pub mod categories;
pub mod import;
pub mod query;
pub mod rules_repo;
pub mod seed_categories;

pub use accounts::Account;
pub use categories::{Category, CategoryKind};
pub use import::ImportOutcome;
pub use query::{TxFilter, TxRow};

use rusqlite::Connection;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(String),
    #[error("unknown account {iban}")]
    UnknownAccount { iban: String, kind: parser::AccountKind },
    #[error("{0}")]
    Parse(String),
}
impl From<rusqlite::Error> for StoreError { fn from(e: rusqlite::Error) -> Self { StoreError::Db(e.to_string()) } }
pub type Result<T> = std::result::Result<T, StoreError>;

pub struct Store { pub(crate) conn: Connection }

impl Store {
    pub fn open(path: &Path) -> Result<Self> { Self::init(Connection::open(path)?) }
    pub fn open_in_memory() -> Result<Self> { Self::init(Connection::open_in_memory()?) }
    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(include_str!("schema.sql"))?;
        let mut s = Store { conn };
        if s.conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get::<_, i64>(0))? == 0 { s.seed_categories()?; s.seed_rules()?; }
        Ok(s)
    }
}
