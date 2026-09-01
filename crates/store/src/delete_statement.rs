//! A17/F1: delete one imported statement, all or nothing.
//!
//! The whole cascade runs inside one `BEGIN IMMEDIATE` (same shape as
//! `import_statement`): a failure anywhere rolls back everything, so the
//! database never holds a half-deleted import.
use crate::{Result, Store, StoreError};
use chrono::NaiveDate;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

/// What the confirmation dialog reads before the user decides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatementDeletePreview {
    pub statement_id: i64,
    pub number: i64,
    pub account_label: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub transaction_count: i64,
    /// Rows the user had already confirmed by hand: the part of the loss that
    /// is real work, not just imported data.
    pub confirmed_count: i64,
    pub rules_deleted: i64,
}

/// What the delete actually removed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatementDeleteOutcome {
    pub statement_id: i64,
    pub number: i64,
    pub transactions_deleted: usize,
    pub rules_deleted: usize,
    /// Surviving `suggested`/`unassigned` rows re-run through `classify`
    /// after the deletion. Confirmed and transfer rows are never in this set.
    pub open_rows_reclassified: usize,
}

/// The learned rules a delete of statement `?1` removes: rules this statement
/// produced, that no other statement produced, and that no transaction outside
/// this statement still uses.
///
/// `match_kind <> 'seed'` is belt and braces (seed rules never get a
/// `rule_sources` row), and a rule with no provenance at all can never appear
/// here, so an unknown origin never authorizes a delete.
///
/// The preview and the delete both run THIS query, and the delete runs it
/// before it removes anything, so the count the user confirms is exactly the
/// set that goes. A future narrowing of one side alone cannot drift from the
/// other.
const DOOMED_RULES: &str = "SELECT DISTINCT rs.rule_id FROM rule_sources rs \
     JOIN rules r ON r.id = rs.rule_id \
     WHERE rs.statement_id = ?1 AND r.match_kind <> 'seed' \
       AND NOT EXISTS (SELECT 1 FROM rule_sources o WHERE o.rule_id = rs.rule_id AND o.statement_id <> ?1) \
       AND NOT EXISTS (SELECT 1 FROM transactions t WHERE t.rule_id = rs.rule_id AND t.statement_id <> ?1)";

impl Store {
    pub fn statement_delete_preview(&self, id: i64) -> Result<StatementDeletePreview> {
        let (number, account_label, start, end) = self.statement_head(id)?;
        let count = |sql: &str| -> Result<i64> { Ok(self.conn.query_row(sql, [id], |r| r.get(0))?) };
        Ok(StatementDeletePreview {
            statement_id: id,
            number,
            account_label,
            period_start: start,
            period_end: end,
            transaction_count: count("SELECT COUNT(*) FROM transactions WHERE statement_id = ?1")?,
            confirmed_count: count("SELECT COUNT(*) FROM transactions WHERE statement_id = ?1 AND status = 'confirmed'")?,
            rules_deleted: count(&format!("SELECT COUNT(*) FROM ({DOOMED_RULES})"))?,
        })
    }

    pub fn delete_statement(&mut self, id: i64) -> Result<StatementDeleteOutcome> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.delete_statement_tx(id) {
            Ok(outcome) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(outcome)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    /// Order matters, and each step depends on the one before it:
    ///
    /// 1. read the statement head, so an unknown id fails before anything is touched;
    /// 2. pick the doomed rules WHILE the evidence still exists (the rule_sources
    ///    rows and the transactions of this statement are about to go);
    /// 3. drop this statement's `rule_sources` rows, then its transactions
    ///    (which releases every `transactions.fingerprint`), then the statement
    ///    row itself (which releases `statements.file_hash`, and with it the
    ///    recent-imports and bad-checksum entries). Children before parents:
    ///    the reverse order would hit the foreign keys;
    /// 4. delete the doomed rules. Any surviving row still pointing at one
    ///    would raise the `transactions.rule_id` foreign key and roll the whole
    ///    delete back, which is the loud failure we want, not a silent orphan;
    /// 5. reclassify the surviving OPEN rows (a deleted row can change what a
    ///    refund or a similarity match resolves to). Confirmed and transfer
    ///    rows are out of scope, so a confirmed category never moves;
    /// 6. recompute `hit_count` from the rows that actually point at each rule,
    ///    last, so the touches step 5 just made are corrected too.
    fn delete_statement_tx(&mut self, id: i64) -> Result<StatementDeleteOutcome> {
        let (number, ..) = self.statement_head(id)?;
        let doomed = self.doomed_rule_ids(id)?;
        self.conn.execute("DELETE FROM rule_sources WHERE statement_id = ?1", [id])?;
        let transactions_deleted = self.conn.execute("DELETE FROM transactions WHERE statement_id = ?1", [id])?;
        self.conn.execute("DELETE FROM statements WHERE id = ?1", [id])?;
        for rule_id in &doomed {
            self.conn.execute("DELETE FROM rules WHERE id = ?1", [rule_id])?;
        }
        let open_rows_reclassified = self.reclassify_open()?;
        self.recompute_hit_counts()?;
        Ok(StatementDeleteOutcome { statement_id: id, number, transactions_deleted, rules_deleted: doomed.len(), open_rows_reclassified })
    }

    fn doomed_rule_ids(&self, id: i64) -> Result<Vec<i64>> {
        let mut st = self.conn.prepare(DOOMED_RULES)?;
        let rows = st.query_map([id], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// `touch_rule` counts classification passes, so a reclassification round
    /// inflates it. After a delete the count is set to what it really is: the
    /// number of transactions currently classified by that rule.
    fn recompute_hit_counts(&mut self) -> Result<()> {
        self.conn.execute("UPDATE rules SET hit_count = (SELECT COUNT(*) FROM transactions t WHERE t.rule_id = rules.id)", [])?;
        Ok(())
    }

    fn statement_head(&self, id: i64) -> Result<(i64, String, NaiveDate, NaiveDate)> {
        let row = self
            .conn
            .query_row(
                "SELECT s.number, a.label, s.period_start, s.period_end FROM statements s JOIN accounts a ON a.id = s.account_id WHERE s.id = ?1",
                [id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)),
            )
            .optional()?;
        let (number, label, start, end) = row.ok_or(StoreError::UnknownStatement { id })?;
        Ok((number, label, parse_date(&start)?, parse_date(&end)?))
    }
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| StoreError::Parse(format!("statement date {s}: {e}")))
}
