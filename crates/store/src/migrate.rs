//! Schema migrations for databases that already exist on a user's machine
//! (`%LOCALAPPDATA%\Abakus\abakus.db`). `schema.sql` is all `IF NOT EXISTS`,
//! so it creates missing tables on an old file by itself; anything that has to
//! MOVE or ADD data on an EXISTING table lives here and runs exactly once,
//! guarded by the `schema_version` setting.
//!
//! Version 1 is any database written before A17/F1 (no `schema_version` row).
//! Version 2 adds `rule_sources`, the provenance of learned rules.
//! Version 3 (0.1.2) adds `transactions.note`.
//! Version 4 (0.1.2 recurring/category release) adds `recurring_decisions`
//! and `recurring_members`. This step runs through the same versioned path
//! for a brand-new database too (`Store::init` never creates these tables
//! via `schema.sql`'s unconditional `CREATE TABLE IF NOT EXISTS`), so a
//! fresh install and an upgraded one reach v4 by the identical code path.
//!
//! `Store::init` wraps `schema.sql` (base table creation) and this whole
//! function in ONE `BEGIN IMMEDIATE`/`COMMIT`: a failure anywhere from the
//! first `CREATE TABLE IF NOT EXISTS` through the final `schema_version`
//! write leaves a database that was already on disk completely unchanged,
//! and never leaves a brand-new file with some v4 objects but no version
//! marker.
use crate::{Result, Store, StoreError};

const VERSION_KEY: &str = "schema_version";
pub(crate) const SCHEMA_VERSION: i64 = 4;

const V4_DDL: &str = "\
CREATE TABLE recurring_decisions (
 id INTEGER PRIMARY KEY,
 account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 group_key TEXT NOT NULL,
 scope TEXT NOT NULL CHECK(scope IN ('group','selected')),
 mode TEXT NOT NULL CHECK(mode IN ('confirmed','ignored')),
 cadence TEXT CHECK(cadence IN ('monthly','quarterly','yearly')),
 anchor_date TEXT,
 identity_json TEXT NOT NULL DEFAULT '{}',
 updated_at TEXT NOT NULL DEFAULT (datetime('now')),
 CHECK((mode='confirmed' AND cadence IS NOT NULL AND anchor_date IS NOT NULL)
    OR (mode='ignored' AND cadence IS NULL AND anchor_date IS NULL))
);
CREATE UNIQUE INDEX recurring_group_unique ON recurring_decisions(group_key) WHERE scope='group';
CREATE TABLE recurring_members (
 decision_id INTEGER NOT NULL REFERENCES recurring_decisions(id) ON DELETE CASCADE,
 fingerprint TEXT NOT NULL UNIQUE,
 PRIMARY KEY(decision_id,fingerprint)
);
";

impl Store {
    /// Called once, from inside `Store::init`'s own transaction: this
    /// function must never open or close a transaction itself (nested
    /// `BEGIN` is a SQLite error), per the migration contract's "do not
    /// start nested transactions in seed or repair helpers".
    pub(crate) fn migrate_tx(&mut self) -> Result<()> {
        let from = self.schema_version()?;
        if from > SCHEMA_VERSION {
            // A database written by a NEWER Abakus than this binary: refuse
            // to touch it rather than silently downgrading its marker or
            // running migration steps meant for an older shape.
            return Err(StoreError::Parse(format!(
                "databáza má novšiu verziu schémy ({from}) než táto aplikácia podporuje ({SCHEMA_VERSION}). Aktualizujte Abakus."
            )));
        }
        if from < 2 {
            self.backfill_rule_sources()?;
        }
        if from < 3 {
            self.add_notes_column()?;
        }
        if from < 4 {
            self.conn.execute_batch(V4_DDL)?;
            self.repair_spotify_seed_tx()?;
        }
        Ok(())
    }

    /// The schema marker is the last initialization write. Fresh seeding or
    /// any migration failure therefore cannot advertise a completed schema.
    pub(crate) fn mark_schema_current_tx(&mut self) -> Result<()> {
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
