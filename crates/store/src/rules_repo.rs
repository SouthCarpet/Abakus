use crate::{Result, Store, StoreError};
use rules::{Rule, RuleKind, Status};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Deserialize)] struct SeedFile { rule: Vec<SeedRule> }
#[derive(Deserialize)] struct SeedRule { key: String, category: String }

pub(crate) fn kind_str(k: RuleKind) -> &'static str { match k { RuleKind::Exact => "exact", RuleKind::Merchant => "merchant", RuleKind::CounterpartyAccount => "counterparty_account", RuleKind::Seed => "seed" } }
fn kind_parse(s: &str) -> RuleKind { match s { "exact" => RuleKind::Exact, "merchant" => RuleKind::Merchant, "counterparty_account" => RuleKind::CounterpartyAccount, _ => RuleKind::Seed } }

impl Store {
    /// Fresh-only initialization helper. The caller owns BEGIN/COMMIT.
    pub(crate) fn seed_rules_tx(&mut self) -> Result<()> {
        let file: SeedFile = toml::from_str(include_str!("seed_rules.toml")).map_err(|e| StoreError::Parse(e.to_string()))?;
        for r in file.rule {
            let cat = self.category_by_path(&r.category)?.ok_or_else(|| StoreError::Parse(format!("seed rule {} points at unknown category {}", r.key, r.category)))?;
            self.insert_rule(RuleKind::Seed, &r.key, None, cat)?;
        }
        Ok(())
    }
    pub fn insert_rule(&mut self, kind: RuleKind, key: &str, place: Option<&str>, category_id: i64) -> Result<i64> {
        // rules.place is NOT NULL DEFAULT '' (schema.sql note): map None <-> '' here so
        // UNIQUE(match_kind, key, place) actually dedupes place-less rules instead of
        // letting SQLite treat every NULL as distinct.
        let place = place.unwrap_or("");
        self.conn.execute("INSERT INTO rules (match_kind, key, place, category_id) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(match_kind, key, place) DO UPDATE SET category_id = excluded.category_id", rusqlite::params![kind_str(kind), key, place, category_id])?;
        Ok(self.conn.query_row("SELECT id FROM rules WHERE match_kind = ?1 AND key = ?2 AND place = ?3", rusqlite::params![kind_str(kind), key, place], |r| r.get(0))?)
    }
    /// A17/F1: records that `transaction_id` produced `rule_id`. The statement
    /// is read from the transaction itself, so provenance can never point at a
    /// statement the row does not belong to. Re-assigning the same row to the
    /// same rule is a no-op (`OR IGNORE` on the primary key), not a duplicate.
    pub fn record_rule_source(&mut self, rule_id: i64, transaction_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO rule_sources (rule_id, transaction_id, statement_id) \
             SELECT ?1, t.id, t.statement_id FROM transactions t WHERE t.id = ?2",
            rusqlite::params![rule_id, transaction_id],
        )?;
        Ok(())
    }

    pub fn list_rules(&self) -> Result<Vec<Rule>> {
        let mut st = self.conn.prepare("SELECT id, match_kind, key, place, category_id FROM rules ORDER BY id")?;
        let rows = st.query_map([], |r| {
            let place: String = r.get(3)?;
            Ok(Rule { id: r.get(0)?, kind: kind_parse(&r.get::<_, String>(1)?), key: r.get(2)?, place: (!place.is_empty()).then_some(place), category_id: r.get(4)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
    /// Counts current open references and visible open-row changes if `id`
    /// were deleted. `open_classification_changes` compares status and
    /// category only. A fallback rule that keeps both values is not a visible
    /// change even though its `rule_id` or source provenance can differ.
    pub fn rule_delete_preview(&self, id: i64) -> Result<RuleDeletePreview> {
        let rules = self.list_rules()?;
        if !rules.iter().any(|rule| rule.id == id) { return Err(StoreError::UnknownRule { id }); }
        let remaining: Vec<Rule> = rules.into_iter().filter(|rule| rule.id != id).collect();
        let own: HashSet<String> = self.list_accounts()?.into_iter().map(|account| account.iban).collect();
        let cash = self.cash_category_id()?;
        let open = self.open_rule_states()?;
        let mut open_rule_references = 0;
        let mut open_classification_changes = 0;
        for state in open {
            if state.rule_id == Some(id) { open_rule_references += 1; }
            let after = self.classify_with_rules(state.id, &own, &remaining, cash)?;
            if after.status != state.status || after.category_id != state.category_id { open_classification_changes += 1; }
        }
        Ok(RuleDeletePreview { rule_id: id, open_rule_references, open_classification_changes })
    }

    /// Detaches every foreign-key reference, deletes the rule and reclassifies
    /// open rows in one transaction. Confirmed and transfer rows keep their
    /// category, status and source. Their `rule_id` is cleared because its
    /// referenced rule no longer exists.
    pub fn delete_rule(&mut self, id: i64) -> Result<()> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.delete_rule_tx(id) {
            Ok(()) => {
                if let Err(error) = self.conn.execute_batch("COMMIT") {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    return Err(error.into());
                }
                Ok(())
            }
            Err(error) => { let _ = self.conn.execute_batch("ROLLBACK"); Err(error) }
        }
    }

    fn delete_rule_tx(&mut self, id: i64) -> Result<()> {
        let exists = self.conn.prepare("SELECT 1 FROM rules WHERE id = ?1")?.exists([id])?;
        if !exists { return Err(StoreError::UnknownRule { id }); }
        self.conn.execute("UPDATE transactions SET rule_id = NULL WHERE rule_id = ?1", [id])?;
        self.conn.execute("DELETE FROM rules WHERE id = ?1", [id])?;
        self.reclassify_open()?;
        Ok(())
    }
    pub fn touch_rule(&mut self, id: i64) -> Result<()> { self.conn.execute("UPDATE rules SET hit_count = hit_count + 1 WHERE id = ?1", [id])?; Ok(()) }

    /// Section 9 seed provenance: only an actual `transactions.rule_id`
    /// pointing at a `match_kind=seed` rule counts, never a guess from
    /// merchant text or category name. A missing transaction is an error; a
    /// transaction with no seed pointer (learned rule, or none) returns
    /// `Ok(None)`, never an error.
    pub fn seed_rule_for_transaction(&self, transaction_id: i64) -> Result<Option<RuleView>> {
        let exists: bool = self.conn.prepare("SELECT 1 FROM transactions WHERE id = ?1")?.exists([transaction_id])?;
        if !exists { return Err(StoreError::UnknownTransaction { id: transaction_id }); }
        let rule_id: Option<i64> = self
            .conn
            .query_row("SELECT r.id FROM transactions t JOIN rules r ON r.id = t.rule_id WHERE t.id = ?1 AND r.match_kind = 'seed'", [transaction_id], |r| r.get(0))
            .ok();
        Ok(match rule_id {
            Some(id) => self.list_rules_view()?.into_iter().find(|r| r.id == id),
            None => None,
        })
    }

    /// Redirects seed and learned rules. Atomic:
    /// invalid target or a mid-write failure rolls back the rule AND every
    /// row it would have touched. Reclassifies only open (`suggested`,
    /// `unassigned`) rows through the existing rule-precedence classifier, so
    /// confirmed and transfer assignments are preserved and a higher-priority
    /// rule still wins where it applies.
    pub fn update_rule_category(&mut self, rule_id: i64, category_id: i64) -> Result<RuleRedirectOutcome> {
        self.list_rules()?.into_iter().find(|rule| rule.id == rule_id).ok_or(StoreError::UnknownRule { id: rule_id })?;
        self.check_redirect_target(category_id)?;
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.update_rule_category_tx(rule_id, category_id) {
            Ok(updated) => {
                if let Err(error) = self.conn.execute_batch("COMMIT") {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    return Err(error.into());
                }
                Ok(RuleRedirectOutcome { rule_id, category_id, updated })
            }
            Err(e) => { let _ = self.conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    /// A usable non-system active assignment category: a leaf, or a root
    /// with no ACTIVE children (an archived child does not block it).
    fn check_redirect_target(&self, category_id: i64) -> Result<()> {
        let cat = self.category_row(category_id)?.ok_or(StoreError::UnknownCategory { id: category_id })?;
        if cat.system || cat.archived { return Err(StoreError::Parse("Cieľová kategória nie je použiteľná.".into())); }
        if cat.parent_id.is_none() && self.children_of(cat.id)?.iter().any(|c| !c.archived) {
            return Err(StoreError::Parse("Cieľová kategória má aktívne podkategórie.".into()));
        }
        Ok(())
    }

    fn update_rule_category_tx(&mut self, rule_id: i64, category_id: i64) -> Result<usize> {
        let n = self.conn.execute("UPDATE rules SET category_id = ?2 WHERE id = ?1 AND match_kind IN ('seed', 'exact', 'merchant', 'counterparty_account')", rusqlite::params![rule_id, category_id])?;
        if n == 0 { return Err(StoreError::UnknownRule { id: rule_id }); }
        self.reclassify_open()?;
        let updated: i64 = self.conn.query_row("SELECT COUNT(*) FROM transactions WHERE rule_id = ?1 AND status = 'suggested'", [rule_id], |r| r.get(0))?;
        Ok(updated as usize)
    }

    fn open_rule_states(&self) -> Result<Vec<OpenRuleState>> {
        let mut statement = self.conn.prepare("SELECT id, status, category_id, rule_id FROM transactions WHERE status IN ('suggested', 'unassigned')")?;
        let rows = statement.query_map([], |row| {
            let status: String = row.get(1)?;
            Ok(OpenRuleState { id: row.get(0)?, status: crate::import::status_parse(&status), category_id: row.get(2)?, rule_id: row.get(3)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Rules joined with their category (and its parent) for the rules-list screen.
    pub fn list_rules_view(&self) -> Result<Vec<RuleView>> {
        let mut st = self.conn.prepare("SELECT r.id, r.match_kind, r.key, r.place, r.category_id, c.name, p.name, r.hit_count FROM rules r JOIN categories c ON c.id = r.category_id LEFT JOIN categories p ON p.id = c.parent_id ORDER BY r.hit_count DESC, r.id")?;
        let rows = st.query_map([], |row| {
            let place: String = row.get(3)?;
            Ok(RuleView { id: row.get(0)?, kind: kind_parse(&row.get::<_, String>(1)?), key: row.get(2)?, place: (!place.is_empty()).then_some(place), category_id: row.get(4)?, category_name: row.get(5)?, parent_name: row.get(6)?, hit_count: row.get(7)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleView { pub id: i64, pub kind: RuleKind, pub key: String, pub place: Option<String>, pub category_id: i64, pub category_name: String, pub parent_name: Option<String>, pub hit_count: i64 }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RuleRedirectOutcome { pub rule_id: i64, pub category_id: i64, pub updated: usize }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RuleDeletePreview { pub rule_id: i64, pub open_rule_references: usize, pub open_classification_changes: usize }

struct OpenRuleState { id: i64, status: Status, category_id: Option<i64>, rule_id: Option<i64> }
