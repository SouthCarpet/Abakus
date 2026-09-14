use super::{affected_row_ids, assignment_rule_identities, RuleIdentity};
use crate::{Result, Store, StoreError};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_UNDO_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
struct AssignmentState {
    status: String,
    category_id: Option<i64>,
    rule_id: Option<i64>,
    source: String,
}

#[derive(Debug)]
struct RowDelta {
    id: i64,
    before: AssignmentState,
    after: AssignmentState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuleState {
    id: i64,
    category_id: i64,
}

#[derive(Debug)]
struct RuleDelta {
    identity: RuleIdentity,
    before: Option<RuleState>,
    after: RuleState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RuleSourceKey {
    rule_id: i64,
    transaction_id: i64,
}

#[derive(Debug)]
pub(crate) struct AssignmentUndo {
    id: String,
    rows: Vec<RowDelta>,
    rules: Vec<RuleDelta>,
    added_rule_sources: Vec<RuleSourceKey>,
    expected_total_changes: u64,
    expected_data_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkAssignOutcome {
    pub updated: usize,
    pub rules_created: usize,
    pub skipped_transfers: usize,
    pub undo_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoAssignmentOutcome {
    pub restored_rows: usize,
}

impl Store {
    pub fn assign_undoable(
        &mut self,
        ids: &[i64],
        category_id: i64,
        apply_to_matching: bool,
    ) -> Result<BulkAssignOutcome> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = self.assign_undoable_tx(ids, category_id, apply_to_matching);
        match result {
            Ok((outcome, undo)) => {
                self.commit_assignment(())?;
                let undo_id = undo.as_ref().map(|record| record.id.clone());
                self.assignment_undo = undo;
                Ok(BulkAssignOutcome {
                    updated: outcome.updated,
                    rules_created: outcome.rules_created,
                    skipped_transfers: outcome.skipped_transfers,
                    undo_id,
                })
            }
            Err(error) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    fn assign_undoable_tx(
        &mut self,
        ids: &[i64],
        category_id: i64,
        apply_to_matching: bool,
    ) -> Result<(super::AssignOutcome, Option<AssignmentUndo>)> {
        let plan = self.prepare_assignment(ids, category_id, apply_to_matching)?;
        if plan.sources.is_empty() {
            let outcome = self.apply_assignment_plan(&plan, super::MatchMode::Merchant)?;
            return Ok((outcome, None));
        }
        let (outcome, undo) = self.capture_assignment_undo(&plan)?;
        Ok((outcome, Some(undo)))
    }

    fn capture_assignment_undo(
        &mut self,
        plan: &super::AssignmentPlan,
    ) -> Result<(super::AssignOutcome, AssignmentUndo)> {
        let row_ids = affected_row_ids(plan);
        let identities = assignment_rule_identities(&plan.sources, &plan.policy);
        let before_rows = self.load_assignment_states(&row_ids)?;
        let before_rules = self.load_rule_states(&identities)?;
        let before_sources = self.load_rule_sources(&row_ids)?;
        let outcome = self.apply_assignment_plan(plan, super::MatchMode::Merchant)?;
        let after_rows = self.load_assignment_states(&row_ids)?;
        let after_rules = self.load_rule_states(&identities)?;
        let after_sources = self.load_rule_sources(&row_ids)?;
        let rules = build_rule_deltas(identities, before_rules, after_rules)?;
        let undo = AssignmentUndo {
            id: format!(
                "assignment-undo-{}",
                NEXT_UNDO_ID.fetch_add(1, Ordering::Relaxed)
            ),
            rows: row_ids
                .iter()
                .zip(before_rows)
                .zip(after_rows)
                .map(|((id, before), after)| RowDelta {
                    id: *id,
                    before,
                    after,
                })
                .collect(),
            rules,
            added_rule_sources: after_sources.difference(&before_sources).copied().collect(),
            expected_total_changes: self.conn.total_changes(),
            expected_data_version: self.data_version()?,
        };
        Ok((outcome, undo))
    }

    pub fn undo_last_assignment(
        &mut self,
        expected_undo_id: &str,
    ) -> Result<Option<UndoAssignmentOutcome>> {
        let Some(current) = self.assignment_undo.as_ref() else {
            return Ok(None);
        };
        if current.id != expected_undo_id {
            return Ok(None);
        }
        let record = self.assignment_undo.take().expect("checked above");
        if let Err(error) = self.conn.execute_batch("BEGIN IMMEDIATE") {
            self.assignment_undo = Some(record);
            return Err(error.into());
        }
        match self.undo_assignment_tx(&record) {
            Ok(false) => {
                self.conn.execute_batch("ROLLBACK")?;
                Ok(None)
            }
            Ok(true) => self.commit_undo(record.rows.len()),
            Err(error) => match self.conn.execute_batch("ROLLBACK") {
                Ok(()) => Err(error),
                Err(rollback_error) => Err(rollback_error.into()),
            },
        }
    }

    fn undo_assignment_tx(&mut self, record: &AssignmentUndo) -> Result<bool> {
        if self.conn.total_changes() != record.expected_total_changes
            || self.data_version()? != record.expected_data_version
            || !self.assignment_post_image_matches(record)?
        {
            return Ok(false);
        }
        for row in &record.rows {
            self.conn.execute("UPDATE transactions SET status = ?2, category_id = ?3, rule_id = ?4, source = ?5 WHERE id = ?1", params![row.id, row.before.status, row.before.category_id, row.before.rule_id, row.before.source])?;
        }
        for source in &record.added_rule_sources {
            self.conn.execute(
                "DELETE FROM rule_sources WHERE rule_id = ?1 AND transaction_id = ?2",
                params![source.rule_id, source.transaction_id],
            )?;
        }
        for rule in &record.rules {
            self.restore_rule(rule)?;
        }
        Ok(true)
    }

    fn commit_undo(&mut self, restored_rows: usize) -> Result<Option<UndoAssignmentOutcome>> {
        if let Err(error) = self.conn.execute_batch("COMMIT") {
            return match self.conn.execute_batch("ROLLBACK") {
                Ok(()) => Err(error.into()),
                Err(rollback_error) => Err(rollback_error.into()),
            };
        }
        Ok(Some(UndoAssignmentOutcome { restored_rows }))
    }

    fn restore_rule(&mut self, rule: &RuleDelta) -> Result<()> {
        match &rule.before {
            Some(before) => {
                self.conn.execute(
                    "UPDATE rules SET category_id = ?2 WHERE id = ?1",
                    params![before.id, before.category_id],
                )?;
            }
            None => {
                self.conn
                    .execute("DELETE FROM rules WHERE id = ?1", [rule.after.id])?;
            }
        }
        Ok(())
    }

    fn assignment_post_image_matches(&self, record: &AssignmentUndo) -> Result<bool> {
        for row in &record.rows {
            if self.load_assignment_state(row.id)? != row.after {
                return Ok(false);
            }
        }
        for rule in &record.rules {
            if self.load_rule_state(&rule.identity)? != Some(rule.after.clone()) {
                return Ok(false);
            }
        }
        for source in &record.added_rule_sources {
            if !self
                .conn
                .prepare("SELECT 1 FROM rule_sources WHERE rule_id = ?1 AND transaction_id = ?2")?
                .exists(params![source.rule_id, source.transaction_id])?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn load_assignment_states(&self, ids: &[i64]) -> Result<Vec<AssignmentState>> {
        ids.iter()
            .map(|id| self.load_assignment_state(*id))
            .collect()
    }

    fn load_assignment_state(&self, id: i64) -> Result<AssignmentState> {
        self.conn
            .query_row(
                "SELECT status, category_id, rule_id, source FROM transactions WHERE id = ?1",
                [id],
                |row| {
                    Ok(AssignmentState {
                        status: row.get(0)?,
                        category_id: row.get(1)?,
                        rule_id: row.get(2)?,
                        source: row.get(3)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    fn load_rule_states(&self, identities: &[RuleIdentity]) -> Result<Vec<Option<RuleState>>> {
        identities
            .iter()
            .map(|identity| self.load_rule_state(identity))
            .collect()
    }

    fn load_rule_state(&self, identity: &RuleIdentity) -> Result<Option<RuleState>> {
        self.conn.query_row("SELECT id, category_id FROM rules WHERE match_kind = ?1 AND key = ?2 AND place = ?3", params![identity.match_kind, identity.key, identity.place], |row| Ok(RuleState { id: row.get(0)?, category_id: row.get(1)? })).optional().map_err(Into::into)
    }

    fn load_rule_sources(&self, transaction_ids: &[i64]) -> Result<HashSet<RuleSourceKey>> {
        let mut result = HashSet::new();
        let mut statement = self.conn.prepare(
            "SELECT rule_id, transaction_id FROM rule_sources WHERE transaction_id = ?1",
        )?;
        for transaction_id in transaction_ids {
            let rows = statement.query_map([transaction_id], |row| {
                Ok(RuleSourceKey {
                    rule_id: row.get(0)?,
                    transaction_id: row.get(1)?,
                })
            })?;
            result.extend(rows.collect::<std::result::Result<Vec<_>, _>>()?);
        }
        Ok(result)
    }

    fn data_version(&self) -> Result<i64> {
        self.conn
            .query_row("PRAGMA main.data_version", [], |row| row.get(0))
            .map_err(Into::into)
    }
}

fn build_rule_deltas(
    identities: Vec<RuleIdentity>,
    before: Vec<Option<RuleState>>,
    after: Vec<Option<RuleState>>,
) -> Result<Vec<RuleDelta>> {
    identities
        .into_iter()
        .zip(before)
        .zip(after)
        .map(|((identity, before), after)| {
            after
                .map(|after| RuleDelta {
                    identity,
                    before,
                    after,
                })
                .ok_or_else(|| StoreError::Db("assigned rule is missing".into()))
        })
        .collect()
}
