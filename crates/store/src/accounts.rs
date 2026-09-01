use crate::{Result, Store, StoreError};
use parser::AccountKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct Account { pub id: i64, pub iban: String, pub kind: AccountKind, pub label: String, pub has_password: bool }
pub(crate) fn kind_str(k: AccountKind) -> &'static str { match k { AccountKind::Personal => "personal", AccountKind::Business => "business" } }
fn kind_parse(s: &str) -> AccountKind { if s == "business" { AccountKind::Business } else { AccountKind::Personal } }

impl Store {
    /// Creates the account, or updates the one that already carries this IBAN.
    /// The IBAN itself is the identity: it is what the row is found by, never
    /// something this call rewrites (A17/F2). A kind change on an account that
    /// already has history goes through `update_account`'s guard, so an import
    /// or a settings save can never recast history silently.
    pub fn upsert_account(&mut self, iban: &str, kind: AccountKind, label: &str) -> Result<i64> {
        let iban = parser::iban::normalize(iban);
        if let Some(existing) = self.account_by_iban(&iban)? {
            self.update_account(existing.id, label, kind, false)?;
            return Ok(existing.id);
        }
        self.conn.execute("INSERT INTO accounts (iban, kind, label) VALUES (?1, ?2, ?3)", rusqlite::params![iban, kind_str(kind), label])?;
        Ok(self.conn.last_insert_rowid())
    }

    /// A17/F2: edits an existing account. The label is free to change (every
    /// historical row reads it through a join, so the new label shows
    /// everywhere at once). The kind is refused once the account has imports
    /// unless the caller acknowledges the recast explicitly, and an
    /// acknowledged change runs the same reclassification an added account
    /// runs (spec D4).
    pub fn update_account(&mut self, id: i64, label: &str, kind: AccountKind, acknowledge_kind_change: bool) -> Result<Account> {
        let current = self.account_by_id(id)?.ok_or(StoreError::UnknownAccountId { id })?;
        let kind_changed = current.kind != kind;
        if kind_changed {
            self.check_kind_change(&current, acknowledge_kind_change)?;
        }
        self.conn.execute("UPDATE accounts SET label = ?2, kind = ?3 WHERE id = ?1", rusqlite::params![id, label, kind_str(kind)])?;
        if kind_changed {
            self.reclassify_after_account_change()?;
        }
        self.account_by_id(id)?.ok_or(StoreError::UnknownAccountId { id })
    }

    /// An account with no statement and no transaction has no history to
    /// recast, so its kind is a free edit. Anything else needs the flag.
    fn check_kind_change(&self, account: &Account, acknowledged: bool) -> Result<()> {
        let (statements, transactions) = self.account_history_counts(account.id)?;
        if acknowledged || (statements == 0 && transactions == 0) {
            return Ok(());
        }
        Err(StoreError::AccountKindLocked { iban: account.iban.clone(), statements, transactions })
    }

    /// How much history a kind change would recast: statements and transactions
    /// of this account. The command layer puts both numbers in the warning.
    pub fn account_history_counts(&self, id: i64) -> Result<(i64, i64)> {
        let count = |sql: &str| -> Result<i64> { Ok(self.conn.query_row(sql, [id], |r| r.get(0))?) };
        Ok((count("SELECT COUNT(*) FROM statements WHERE account_id = ?1")?, count("SELECT COUNT(*) FROM transactions WHERE account_id = ?1")?))
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let mut st = self.conn.prepare("SELECT id, iban, kind, label, has_password FROM accounts ORDER BY id")?;
        let rows = st.query_map([], |r| Ok(Account { id: r.get(0)?, iban: r.get(1)?, kind: kind_parse(&r.get::<_, String>(2)?), label: r.get(3)?, has_password: r.get::<_, i64>(4)? != 0 }))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
    pub fn account_by_iban(&self, iban: &str) -> Result<Option<Account>> { Ok(self.list_accounts()?.into_iter().find(|a| a.iban == parser::iban::normalize(iban))) }
    pub fn account_by_id(&self, id: i64) -> Result<Option<Account>> { Ok(self.list_accounts()?.into_iter().find(|a| a.id == id)) }
    pub fn set_has_password(&mut self, id: i64, has: bool) -> Result<()> { self.conn.execute("UPDATE accounts SET has_password = ?2 WHERE id = ?1", rusqlite::params![id, has])?; Ok(()) }
}
