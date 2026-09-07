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
pub mod rules_repo;
pub mod seed_categories;
pub mod settings;
pub mod summary;

pub use accounts::Account;
pub use assign::AssignOutcome;
pub use backup::BackupOutcome;
pub use categories::{Category, CategoryKind};
pub use delete_account::{AccountDeleteOutcome, AccountDeletePreview};
pub use delete_statement::{StatementDeleteOutcome, StatementDeletePreview};
pub use history::StatementHistoryRow;
pub use import::ImportOutcome;
pub use net_log::NetLogRow;
pub use notes::NOTE_MAX_CHARS;
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
    #[error("Transakcia s id {id} neexistuje.")]
    UnknownTransaction { id: i64 },
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

    /// recurring-contract.md §8: base schema creation, the notes migration,
    /// v4 DDL, the (pending) Spotify repair and the `schema_version` write
    /// all share ONE transaction, for both a brand-new database and an
    /// existing one. Category/rule seeding stays outside it, unchanged from
    /// before: it only ever runs once against an already-committed empty
    /// `categories` table, has its own transaction
    /// (`categories::seed_categories`, owned by lane C), and the contract
    /// does not name it among the steps that must share this transaction.
    ///
    /// Mirrors `delete_account`'s rollback shape: a failure inside the body
    /// OR of `COMMIT` itself rolls back and returns the error, never leaving
    /// the connection with an open transaction on it.
    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let mut s = Store { conn };
        s.conn.execute_batch("BEGIN IMMEDIATE")?;
        let init_result = s.conn.execute_batch(include_str!("schema.sql")).map_err(Into::into).and_then(|()| s.migrate_tx());
        match init_result {
            Ok(()) => match s.conn.execute_batch("COMMIT") {
                Ok(()) => {}
                Err(e) => { let _ = s.conn.execute_batch("ROLLBACK"); return Err(e.into()); }
            },
            Err(e) => { let _ = s.conn.execute_batch("ROLLBACK"); return Err(e); }
        }
        if s.conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get::<_, i64>(0))? == 0 { s.seed_categories()?; s.seed_rules()?; }
        Ok(s)
    }
}
