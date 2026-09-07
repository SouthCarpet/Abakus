//! SQLite store: accounts, statements, transactions, categories, rules, summaries.
pub mod accounts;
pub mod assign;
pub mod backup;
pub mod categories;
pub mod delete_account;
pub mod delete_statement;
pub mod history;
pub mod import;
pub mod migrate;
pub mod net_log;
pub mod notes;
pub mod query;
pub mod recurring;
pub mod report;
pub mod rules_repo;
pub mod seed_categories;
pub mod seed_repair;
pub mod settings;
pub mod summary;

pub use accounts::Account;
pub use assign::AssignOutcome;
pub use backup::BackupOutcome;
pub use categories::{Category, CategoryKind, CategoryUpdatePreview, CategoryUpdateRequest};
pub use delete_account::{AccountDeleteOutcome, AccountDeletePreview};
pub use delete_statement::{StatementDeleteOutcome, StatementDeletePreview};
pub use history::StatementHistoryRow;
pub use import::ImportOutcome;
pub use net_log::NetLogRow;
pub use notes::NOTE_MAX_CHARS;
pub use query::{RecentStatement, TxFilter, TxRow};
pub use rules_repo::{RuleRedirectOutcome, RuleView};
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
    #[error("Transakcia s id {id} neexistuje.")]
    UnknownTransaction { id: i64 },
    #[error("Kategória s id {id} neexistuje.")]
    UnknownCategory { id: i64 },
    #[error("Pravidlo s id {id} neexistuje.")]
    UnknownRule { id: i64 },
    #[error("Poznámka nesmie obsahovať znak NUL.")]
    NoteContainsNul,
    #[error("Poznámka je príliš dlhá. Limit je {max} znakov, poznámka má {actual}.")]
    NoteTooLong { max: usize, actual: usize },
    #[error("Cieľový súbor už existuje: {path}. Zvoľte iný názov.")]
    BackupTargetExists { path: String },
    #[error("{0}")]
    Parse(String),
}
impl From<rusqlite::Error> for StoreError { fn from(e: rusqlite::Error) -> Self { StoreError::Db(e.to_string()) } }
pub type Result<T> = std::result::Result<T, StoreError>;

pub struct Store { pub(crate) conn: Connection }

impl Store {
    pub fn open(path: &Path) -> Result<Self> { Self::init(Connection::open(path)?) }
    pub fn open_in_memory() -> Result<Self> { Self::init(Connection::open_in_memory()?) }

    /// Base DDL, every migration, fresh-only category/rule seeding, the exact
    /// Spotify repair and the final version marker share one transaction.
    /// A failed body or COMMIT therefore leaves an existing database at its
    /// original version and never leaves a fresh database half initialized.
    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let mut s = Store { conn };
        s.conn.execute_batch("BEGIN IMMEDIATE")?;
        let init_result = s.initialize_tx();
        if let Err(error) = init_result {
            let _ = s.conn.execute_batch("ROLLBACK");
            return Err(error);
        }
        if let Err(error) = s.conn.execute_batch("COMMIT") {
            let _ = s.conn.execute_batch("ROLLBACK");
            return Err(error.into());
        }
        Ok(s)
    }

    fn initialize_tx(&mut self) -> Result<()> {
        self.conn.execute_batch(include_str!("schema.sql"))?;
        let fresh = self.is_fresh_database()?;
        self.migrate_tx()?;
        if fresh {
            self.seed_categories_tx()?;
            self.seed_rules_tx()?;
        }
        self.mark_schema_current_tx()
    }

    fn is_fresh_database(&self) -> Result<bool> {
        let application_rows: i64 = self.conn.query_row(
            "SELECT (SELECT COUNT(*) FROM accounts)
                  + (SELECT COUNT(*) FROM statements)
                  + (SELECT COUNT(*) FROM transactions)
                  + (SELECT COUNT(*) FROM categories)
                  + (SELECT COUNT(*) FROM rules)
                  + (SELECT COUNT(*) FROM settings)",
            [],
            |row| row.get(0),
        )?;
        Ok(application_rows == 0)
    }
}
