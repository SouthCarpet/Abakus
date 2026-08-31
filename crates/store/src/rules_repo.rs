use crate::{Result, Store, StoreError};
use rules::{Rule, RuleKind};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)] struct SeedFile { rule: Vec<SeedRule> }
#[derive(Deserialize)] struct SeedRule { key: String, category: String }

fn kind_str(k: RuleKind) -> &'static str { match k { RuleKind::Exact => "exact", RuleKind::Merchant => "merchant", RuleKind::CounterpartyAccount => "counterparty_account", RuleKind::Seed => "seed" } }
fn kind_parse(s: &str) -> RuleKind { match s { "exact" => RuleKind::Exact, "merchant" => RuleKind::Merchant, "counterparty_account" => RuleKind::CounterpartyAccount, _ => RuleKind::Seed } }

impl Store {
    pub(crate) fn seed_rules(&mut self) -> Result<()> {
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
    pub fn list_rules(&self) -> Result<Vec<Rule>> {
        let mut st = self.conn.prepare("SELECT id, match_kind, key, place, category_id FROM rules ORDER BY id")?;
        let rows = st.query_map([], |r| {
            let place: String = r.get(3)?;
            Ok(Rule { id: r.get(0)?, kind: kind_parse(&r.get::<_, String>(1)?), key: r.get(2)?, place: (!place.is_empty()).then_some(place), category_id: r.get(4)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
    /// Reopens any row that was pointing at this rule: nulls `rule_id`, deletes
    /// the rule, then reclassifies every open row (so a suggestion whose rule
    /// just vanished returns to `unassigned` without a caller having to ask).
    pub fn delete_rule(&mut self, id: i64) -> Result<()> {
        self.conn.execute("UPDATE transactions SET rule_id = NULL WHERE rule_id = ?1", [id])?;
        self.conn.execute("DELETE FROM rules WHERE id = ?1", [id])?;
        self.reclassify_open()?;
        Ok(())
    }
    pub fn touch_rule(&mut self, id: i64) -> Result<()> { self.conn.execute("UPDATE rules SET hit_count = hit_count + 1 WHERE id = ?1", [id])?; Ok(()) }

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
