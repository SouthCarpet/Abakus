//! SQLite store: accounts, statements, transactions, categories, rules, summaries.
pub mod accounts;
pub mod assign;
pub mod categories;
pub mod delete_account;
pub mod delete_statement;
pub mod import;
pub mod migrate;
pub mod net_log;
pub mod query;
pub mod rules_repo;
pub mod seed_categories;
pub mod settings;
pub mod summary;

pub use accounts::Account;
pub use assign::AssignOutcome;
pub use categories::{Category, CategoryKind};
pub use delete_account::{AccountDeleteOutcome, AccountDeletePreview};
pub use delete_statement::{StatementDeleteOutcome, StatementDeletePreview};
pub use import::ImportOutcome;
pub use net_log::NetLogRow;
pub use query::{RecentStatement, TxFilter, TxRow};
pub use rules_repo::RuleView;
pub use summary::*;

use rusqlite::Connection;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(String),
    #[error("unknown account {iban}")]
    UnknownAccount { iban: String, kind: parser::AccountKind },
    #[error("no account with id {id}")]
    UnknownAccountId { id: i64 },
    #[error("no statement with id {id}")]
    UnknownStatement { id: i64 },
    /// A17/F2: the account kind decides how every historical row of that
    /// account reads (personal or business), so changing it after an import
    /// needs an explicit acknowledgement. The counts travel with the error so
    /// the command layer can say what exactly would be recast.
    #[error("Účet {iban} už obsahuje výpisy: {statements}. Transakcie: {transactions}. Zmena typu účtu vyžaduje potvrdenie.")]
    AccountKindLocked { iban: String, statements: i64, transactions: i64 },
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
        // Runs on every open, does its work once: an existing user database
        // must reach the current schema, not only a freshly created one.
        s.migrate()?;
        Ok(s)
    }
}
