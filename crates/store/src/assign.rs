//! Manual assignment, confirmation and reclassification. Transfer rows are a
//! protected invariant (spec A1): `assign`/`confirm` never touch them.
use crate::{Result, Store};
use rules::RuleKind;
use serde::{Deserialize, Serialize};

/// What one assigned row taught: the rules it created or reused, so
/// `apply_to_matching` can sweep its merchant afterwards.
struct Learned {
    merchant: String,
    merchant_rule: Option<i64>,
    created: usize,
}

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
        let mut outcome = AssignOutcome { updated: 0, rules_created: 0, skipped_transfers: 0 };
        let mut learned = Vec::new();
        for id in ids {
            match self.assign_one(*id, category_id)? {
                Some(l) => {
                    outcome.updated += 1;
                    outcome.rules_created += l.created;
                    learned.push(l);
                }
                None => outcome.skipped_transfers += 1,
            }
        }
        if apply_to_matching {
            for l in &learned {
                self.apply_merchant_rule(&l.merchant, l.merchant_rule, category_id)?;
            }
        }
        Ok(outcome)
    }

    /// One row: learn the rules it teaches, confirm it, and record that THIS
    /// transaction is where those rules came from (A17/F1). A transfer row
    /// teaches nothing and is reported as skipped.
    fn assign_one(&mut self, id: i64, category_id: i64) -> Result<Option<Learned>> {
        let (merchant, place, status): (String, Option<String>, String) = self.conn.query_row(
            "SELECT merchant_norm, place_norm, status FROM transactions WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if status == "transfer" {
            return Ok(None);
        }
        let before = self.list_rules()?.len();
        let exact = self.insert_rule(RuleKind::Exact, &merchant, place.as_deref(), category_id)?;
        let merchant_rule = match merchant.is_empty() {
            true => None,
            false => Some(self.insert_rule(RuleKind::Merchant, &merchant, None, category_id)?),
        };
        let created = self.list_rules()?.len() - before;
        self.conn.execute(
            "UPDATE transactions SET status = 'confirmed', category_id = ?2, rule_id = ?3, source = 'exact_rule' WHERE id = ?1 AND status <> 'transfer'",
            rusqlite::params![id, category_id, exact],
        )?;
        self.record_rule_source(exact, id)?;
        if let Some(rule) = merchant_rule {
            self.record_rule_source(rule, id)?;
        }
        Ok(Some(Learned { merchant, merchant_rule, created }))
    }

    /// The rows this assignment sweeps along keep pointing at the merchant
    /// rule that classified them (`COALESCE` leaves `rule_id` alone when the
    /// merchant is empty and no merchant rule exists). Without that pointer a
    /// statement delete could remove a rule that a surviving row still uses.
    /// Confirmed rows are never swept: only `suggested` and `unassigned`.
    ///
    /// A17/F1 gap closed: a swept row also gets a `rule_sources` row of its
    /// own, recorded under ITS statement, not the row that taught the rule.
    /// Without this, a rule the teaching statement's own delete correctly
    /// kept (a surviving row still used it) could become undeletable later:
    /// once the teacher is gone, the swept row's statement has no provenance
    /// of its own to hand `delete_statement` when its turn comes.
    fn apply_merchant_rule(&mut self, merchant: &str, rule_id: Option<i64>, category_id: i64) -> Result<()> {
        let swept: Vec<i64> = {
            let mut st = self.conn.prepare("SELECT id FROM transactions WHERE merchant_norm = ?1 AND status IN ('suggested', 'unassigned')")?;
            let rows = st.query_map([merchant], |r| r.get(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        self.conn.execute(
            "UPDATE transactions SET status = 'confirmed', category_id = ?2, rule_id = COALESCE(?3, rule_id), source = 'merchant_rule' WHERE merchant_norm = ?1 AND status IN ('suggested', 'unassigned')",
            rusqlite::params![merchant, category_id, rule_id],
        )?;
        if let Some(rule) = rule_id {
            for id in swept {
                self.record_rule_source(rule, id)?;
            }
        }
        Ok(())
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
