use crate::{Result, Store};
use parser::AccountKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct Account { pub id: i64, pub iban: String, pub kind: AccountKind, pub label: String, pub has_password: bool }
fn kind_str(k: AccountKind) -> &'static str { match k { AccountKind::Personal => "personal", AccountKind::Business => "business" } }
fn kind_parse(s: &str) -> AccountKind { if s == "business" { AccountKind::Business } else { AccountKind::Personal } }

impl Store {
    pub fn upsert_account(&mut self, iban: &str, kind: AccountKind, label: &str) -> Result<i64> {
        let iban = parser::iban::normalize(iban);
        self.conn.execute("INSERT INTO accounts (iban, kind, label) VALUES (?1, ?2, ?3) ON CONFLICT(iban) DO UPDATE SET kind = excluded.kind, label = excluded.label", rusqlite::params![iban, kind_str(kind), label])?;
        Ok(self.conn.query_row("SELECT id FROM accounts WHERE iban = ?1", [iban], |r| r.get(0))?)
    }
    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let mut st = self.conn.prepare("SELECT id, iban, kind, label, has_password FROM accounts ORDER BY id")?;
        let rows = st.query_map([], |r| Ok(Account { id: r.get(0)?, iban: r.get(1)?, kind: kind_parse(&r.get::<_, String>(2)?), label: r.get(3)?, has_password: r.get::<_, i64>(4)? != 0 }))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
    pub fn account_by_iban(&self, iban: &str) -> Result<Option<Account>> { Ok(self.list_accounts()?.into_iter().find(|a| a.iban == parser::iban::normalize(iban))) }
    pub fn set_has_password(&mut self, id: i64, has: bool) -> Result<()> { self.conn.execute("UPDATE accounts SET has_password = ?2 WHERE id = ?1", rusqlite::params![id, has])?; Ok(()) }
}
