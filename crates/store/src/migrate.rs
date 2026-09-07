//! Schema migrations for databases that already exist on a user's machine
//! (`%LOCALAPPDATA%\Abakus\abakus.db`). `schema.sql` is all `IF NOT EXISTS`,
//! so it creates missing tables on an old file by itself; anything that has to
//! MOVE or ADD data on an EXISTING table lives here and runs exactly once,
//! guarded by the `schema_version` setting.
//!
//! Version 1 is any database written before A17/F1 (no `schema_version` row).
//! Version 2 adds `rule_sources`, the provenance of learned rules.
//! Version 3 (0.1.2) adds `transactions.note`.
use crate::{Result, Store};

const VERSION_KEY: &str = "schema_version";
pub(crate) const SCHEMA_VERSION: i64 = 3;

impl Store {
    /// One transaction for every step below: a database that fails partway
    /// through (disk full, another process holding the file) must come back
    /// up still at its OLD version, not stuck between two schemas.
    pub(crate) fn migrate(&mut self) -> Result<()> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.migrate_tx() {
            Ok(()) => { self.conn.execute_batch("COMMIT")?; Ok(()) }
            Err(e) => { let _ = self.conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    fn migrate_tx(&mut self) -> Result<()> {
        let from = self.schema_version()?;
        if from < 2 {
            self.backfill_rule_sources()?;
        }
        if from < 3 {
            self.add_notes_column()?;
        }
        self.set_setting(VERSION_KEY, &SCHEMA_VERSION.to_string())
    }

    pub(crate) fn schema_version(&self) -> Result<i64> {
        Ok(self.setting(VERSION_KEY)?.and_then(|v| v.parse().ok()).unwrap_or(1))
    }

    /// Idempotent regardless of the version guard above it: a brand-new
    /// database reaches `schema_version` 3 in the SAME call that creates the
    /// column via `schema.sql`, so by the time this runs the column may
    /// already exist. `pragma_table_info` is the portable way to ask SQLite
    /// "does this column exist" without relying on `ADD COLUMN IF NOT
    /// EXISTS`, which not every bundled SQLite version accepts.
    fn add_notes_column(&mut self) -> Result<()> {
        let has_note: bool = self
            .conn
            .prepare("SELECT 1 FROM pragma_table_info('transactions') WHERE name = 'note'")?
            .exists([])?;
        if !has_note {
            self.conn.execute_batch("ALTER TABLE transactions ADD COLUMN note TEXT NOT NULL DEFAULT ''")?;
        }
        Ok(())
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
