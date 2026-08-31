use crate::accounts::Account;
use crate::{Result, Store, StoreError};
use parser::{Checksum, Statement, Transaction};
use rules::{classify, Context, Facts, Status};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportOutcome { pub statement_id: i64, pub inserted: usize, pub duplicates: usize, pub checksum: Checksum, pub warnings: Vec<String>, pub already_imported: bool }

pub fn fingerprint(iban: &str, t: &Transaction) -> String {
    let norm_block = t.raw_block.split_whitespace().collect::<Vec<_>>().join(" ");
    hex::encode(Sha256::digest(format!("{iban}|{}|{}|{norm_block}", t.posted_date, t.amount_cents)))
}

fn checksum_cols(c: Checksum) -> (&'static str, Option<i64>) { match c { Checksum::Ok => ("ok", None), Checksum::OffBy(d) => ("off", Some(d)), Checksum::NotVerifiable => ("not_verifiable", None) } }

impl Store {
    /// Statement insert, transaction loop and classification run in one SQLite
    /// transaction (amendment A1): any failure rolls the whole import back
    /// instead of leaving a half-imported statement.
    pub fn import_statement(&mut self, st: &Statement, file_hash: &str) -> Result<ImportOutcome> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.import_statement_tx(st, file_hash) {
            Ok(outcome) => { self.conn.execute_batch("COMMIT")?; Ok(outcome) }
            Err(e) => { let _ = self.conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    fn import_statement_tx(&mut self, st: &Statement, file_hash: &str) -> Result<ImportOutcome> {
        let account = self.account_by_iban(&st.iban)?.ok_or_else(|| StoreError::UnknownAccount { iban: st.iban.clone(), kind: st.account_kind })?;
        if let Some(statement_id) = self.existing_statement_id(file_hash)? {
            return Ok(ImportOutcome { statement_id, inserted: 0, duplicates: st.transactions.len(), checksum: st.checksum(), warnings: st.warnings.clone(), already_imported: true });
        }
        let statement_id = self.insert_statement(&account, st, file_hash)?;
        let (inserted, duplicates, new_ids) = self.insert_transactions(statement_id, account.id, &st.iban, &st.transactions)?;
        self.classify_ids(&new_ids)?;
        Ok(ImportOutcome { statement_id, inserted, duplicates, checksum: st.checksum(), warnings: st.warnings.clone(), already_imported: false })
    }

    fn existing_statement_id(&self, file_hash: &str) -> Result<Option<i64>> {
        Ok(self.conn.query_row("SELECT id FROM statements WHERE file_hash = ?1", [file_hash], |r| r.get::<_, i64>(0)).ok())
    }

    fn insert_statement(&mut self, account: &Account, st: &Statement, file_hash: &str) -> Result<i64> {
        let (status, off) = checksum_cols(st.checksum());
        self.conn.execute("INSERT INTO statements (account_id, number, period_start, period_end, opening_cents, closing_cents, checksum_status, checksum_off_by, file_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![account.id, st.number as i64, st.period_start.to_string(), st.period_end.to_string(), st.opening_cents, st.closing_cents, status, off, file_hash])?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Statements currently imported (used by the a2 dedup test to confirm a
    /// re-export under a new file hash creates a second row).
    pub fn statement_count(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM statements", [], |r| r.get(0))?)
    }

    fn insert_transactions(&mut self, statement_id: i64, account_id: i64, iban: &str, txs: &[Transaction]) -> Result<(usize, usize, Vec<i64>)> {
        let mut inserted = 0;
        let mut duplicates = 0;
        let mut new_ids = Vec::new();
        for t in txs {
            match self.insert_transaction(statement_id, account_id, iban, t)? {
                Some(id) => { inserted += 1; new_ids.push(id); }
                None => duplicates += 1,
            }
        }
        Ok((inserted, duplicates, new_ids))
    }

    fn insert_transaction(&mut self, statement_id: i64, account_id: i64, iban: &str, t: &Transaction) -> Result<Option<i64>> {
        let fp = fingerprint(iban, t);
        let n = self.conn.execute("INSERT OR IGNORE INTO transactions (statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, orig_amount_cents, orig_currency, rate_micros, merchant_raw, merchant_norm, place, place_norm, counterparty_name, counterparty_iban, reference, card_last4, raw_block, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, 'unassigned')",
            rusqlite::params![statement_id, account_id, fp, t.posted_date.to_string(), t.tx_date.to_string(), serde_json::to_value(t.kind).unwrap().as_str().unwrap(), t.amount_cents, t.orig_amount_cents, t.orig_currency, t.rate_micros, t.merchant_raw, rules::normalize(&t.merchant_raw), t.place, t.place.as_deref().map(rules::normalize), t.counterparty_name, t.counterparty_iban, t.reference, t.card_last4, t.raw_block])?;
        Ok((n == 1).then(|| self.conn.last_insert_rowid()))
    }

    /// Runs `rules::classify` for the given rows and writes status, category, rule and source.
    pub fn classify_ids(&mut self, ids: &[i64]) -> Result<usize> {
        let own: HashSet<String> = self.list_accounts()?.into_iter().map(|a| a.iban).collect();
        let rule_list = self.list_rules()?;
        let cash = self.cash_category_id()?;
        let mut changed = 0;
        for id in ids {
            let a = self.classify_one(*id, &own, &rule_list, cash)?;
            self.conn.execute("UPDATE transactions SET status = ?2, category_id = ?3, rule_id = ?4, source = ?5 WHERE id = ?1", rusqlite::params![id, status_str(a.status), a.category_id, a.rule_id, serde_json::to_value(a.source).unwrap().as_str().unwrap()])?;
            if let Some(r) = a.rule_id { self.touch_rule(r)?; }
            changed += 1;
        }
        Ok(changed)
    }

    fn classify_one(&self, id: i64, own: &HashSet<String>, rule_list: &[rules::Rule], cash: Option<i64>) -> Result<rules::Assignment> {
        let (kind, merchant_norm, place_norm, iban): (String, String, Option<String>, Option<String>) = self.conn.query_row("SELECT kind, merchant_norm, place_norm, counterparty_iban FROM transactions WHERE id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
        let lookup = |m: &str| self.last_confirmed_category(m);
        Ok(classify(&Facts { kind: serde_json::from_value(serde_json::Value::String(kind)).unwrap(), merchant_norm: &merchant_norm, place_norm: place_norm.as_deref(), counterparty_iban: iban.as_deref() }, &Context { own_ibans: own, rules: rule_list, cash_category: cash, refund_lookup: &lookup }))
    }

    fn last_confirmed_category(&self, merchant_norm: &str) -> Option<i64> {
        self.conn.query_row("SELECT category_id FROM transactions WHERE merchant_norm = ?1 AND status = 'confirmed' AND amount_cents < 0 ORDER BY tx_date DESC LIMIT 1", [merchant_norm], |r| r.get(0)).ok()
    }
}

pub(crate) fn status_str(s: Status) -> &'static str { match s { Status::Transfer => "transfer", Status::Confirmed => "confirmed", Status::Suggested => "suggested", Status::Unassigned => "unassigned" } }
pub(crate) fn status_parse(s: &str) -> Status { match s { "transfer" => Status::Transfer, "confirmed" => Status::Confirmed, "suggested" => Status::Suggested, _ => Status::Unassigned } }
