//! Schema migrations for databases that already exist on a user's machine
//! (`%LOCALAPPDATA%\Abakus\abakus.db`). `schema.sql` is all `IF NOT EXISTS`,
//! so it creates missing tables on an old file by itself; anything that has to
//! MOVE data lives here and runs exactly once, guarded by the `schema_version`
//! setting.
//!
//! Version 1 is any database written before A17/F1 (no `schema_version` row).
//! Version 2 adds `rule_sources`, the provenance of learned rules.
use crate::{Result, Store};

const VERSION_KEY: &str = "schema_version";
pub(crate) const SCHEMA_VERSION: i64 = 2;

impl Store {
    pub(crate) fn migrate(&mut self) -> Result<()> {
        let from = self.schema_version()?;
        if from < 2 {
            self.backfill_rule_sources()?;
        }
        self.set_setting(VERSION_KEY, &SCHEMA_VERSION.to_string())
    }

    pub(crate) fn schema_version(&self) -> Result<i64> {
        Ok(self.setting(VERSION_KEY)?.and_then(|v| v.parse().ok()).unwrap_or(1))
    }

    /// A pre-A17 database has learned rules but no record of where they came
    /// from. The only evidence left is which transaction currently points at a
    /// rule, so that is what the backfill writes.
    ///
    /// The gap is deliberate and one-directional: a learned rule that no row
    /// points at (for example the merchant rule of an assignment whose exact
    /// rule won the classification) gets NO provenance row, so
    /// `delete_statement` never selects it. Unknown provenance never
    /// authorizes a delete.
    fn backfill_rule_sources(&mut self) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO rule_sources (rule_id, transaction_id, statement_id) \
             SELECT t.rule_id, t.id, t.statement_id FROM transactions t \
             JOIN rules r ON r.id = t.rule_id \
             WHERE t.rule_id IS NOT NULL AND r.match_kind <> 'seed'",
            [],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;

    #[test]
    fn a_fresh_database_is_already_at_the_current_version() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.schema_version().unwrap(), super::SCHEMA_VERSION);
    }
}
