//! Manual assignment, confirmation and reclassification. Transfer rows are a
//! protected invariant (spec A1): `assign`/`confirm` never touch them.
use crate::{Result, Store};
use rules::RuleKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssignOutcome {
    pub updated: usize,
    pub rules_created: usize,
    pub skipped_transfers: usize,
}

impl Store {
    /// One SQLite transaction for the whole batch: any failure (unknown id,
    /// FK-violating category) rolls every row back instead of leaving a
    /// half-applied assignment.
    pub fn assign(&mut self, ids: &[i64], category_id: i64, apply_to_matching: bool) -> Result<AssignOutcome> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.assign_tx(ids, category_id, apply_to_matching) {
            Ok(outcome) => { self.conn.execute_batch("COMMIT")?; Ok(outcome) }
            Err(e) => { let _ = self.conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    fn assign_tx(&mut self, ids: &[i64], category_id: i64, apply_to_matching: bool) -> Result<AssignOutcome> {
        let mut rules_created = 0;
        let mut updated = 0;
        let mut skipped_transfers = 0;
        let mut merchants = Vec::new();
        for id in ids {
            let (m, p, status): (String, Option<String>, String) = self.conn.query_row(
                "SELECT merchant_norm, place_norm, status FROM transactions WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            if status == "transfer" {
                skipped_transfers += 1;
                continue;
            }
            let before = self.list_rules()?.len();
            let rid = self.insert_rule(RuleKind::Exact, &m, p.as_deref(), category_id)?;
            if !m.is_empty() { self.insert_rule(RuleKind::Merchant, &m, None, category_id)?; }
            rules_created += self.list_rules()?.len() - before;
            self.conn.execute(
                "UPDATE transactions SET status = 'confirmed', category_id = ?2, rule_id = ?3, source = 'exact_rule' WHERE id = ?1 AND status <> 'transfer'",
                rusqlite::params![id, category_id, rid],
            )?;
            updated += 1;
            merchants.push(m);
        }
        if apply_to_matching {
            for m in &merchants {
                self.conn.execute(
                    "UPDATE transactions SET status = 'confirmed', category_id = ?2, source = 'merchant_rule' WHERE merchant_norm = ?1 AND status IN ('suggested', 'unassigned')",
                    rusqlite::params![m, category_id],
                )?;
            }
        }
        Ok(AssignOutcome { updated, rules_created, skipped_transfers })
    }

    pub fn confirm(&mut self, ids: &[i64]) -> Result<usize> {
        let mut n = 0;
        for id in ids {
            let cat: Option<i64> = self.conn.query_row("SELECT category_id FROM transactions WHERE id = ?1 AND status = 'suggested'", [id], |r| r.get(0)).ok().flatten();
            if let Some(c) = cat { self.assign(&[*id], c, false)?; n += 1; }
        }
        Ok(n)
    }

    pub fn reclassify_open(&mut self) -> Result<usize> {
        let ids: Vec<i64> = {
            let mut st = self.conn.prepare("SELECT id FROM transactions WHERE status IN ('suggested', 'unassigned')")?;
            let r = st.query_map([], |r| r.get(0))?;
            r.collect::<std::result::Result<_, _>>()?
        };
        self.classify_ids(&ids)
    }

    /// Every row (any status) whose counterparty IBAN just became an own
    /// account: flip it to `transfer` and drop its category/rule, then let
    /// `reclassify_open` re-run the rest of the open rows. Returns the
    /// number of rows flipped to transfer.
    pub fn reclassify_after_account_change(&mut self) -> Result<usize> {
        let own: Vec<String> = self.list_accounts()?.into_iter().map(|a| a.iban).collect();
        let mut flipped = 0;
        for iban in &own {
            flipped += self.conn.execute(
                "UPDATE transactions SET status = 'transfer', category_id = NULL, rule_id = NULL, source = 'own_account' WHERE counterparty_iban = ?1",
                [iban],
            )?;
        }
        self.reclassify_open()?;
        Ok(flipped)
    }
}
