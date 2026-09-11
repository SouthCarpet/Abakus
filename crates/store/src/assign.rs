//! Manual assignment, confirmation and reclassification. Transfer rows are a
//! protected invariant (spec A1): `assign`/`confirm` never touch them.
use crate::{Result, Store, StoreError};
use rules::RuleKind;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum MatchKey {
    MerchantPlace {
        merchant: String,
        place: Option<String>,
    },
    CounterpartyAccount(String),
}

#[derive(Clone, Copy)]
enum MatchMode {
    Merchant,
    ExactIdentity,
}

#[derive(Clone)]
struct StoredTransaction {
    id: i64,
    kind: String,
    merchant: String,
    place: Option<String>,
    counterparty_iban: Option<String>,
    category_id: Option<i64>,
    status: String,
}

impl StoredTransaction {
    fn match_key(&self, mode: MatchMode) -> Option<MatchKey> {
        if matches!(mode, MatchMode::Merchant) {
            return (!self.merchant.is_empty()).then(|| MatchKey::MerchantPlace {
                merchant: self.merchant.clone(),
                place: None,
            });
        }
        if uses_counterparty_account(&self.kind) {
            return self
                .counterparty_iban
                .as_deref()
                .map(str::trim)
                .filter(|iban| !iban.is_empty())
                .map(|iban| MatchKey::CounterpartyAccount(iban.to_string()));
        }
        (!self.merchant.is_empty()).then(|| MatchKey::MerchantPlace {
            merchant: self.merchant.clone(),
            place: self.place.clone(),
        })
    }
}

struct AssignmentSource {
    row: StoredTransaction,
    category_id: i64,
}

struct MatchingTarget {
    id: i64,
    key: MatchKey,
    category_id: i64,
}

#[derive(Clone, Copy)]
struct MatchingRule {
    id: i64,
    source: &'static str,
}

struct Learned {
    key: Option<MatchKey>,
    matching_rule: Option<MatchingRule>,
    source_rules: Vec<i64>,
    primary_rule: Option<i64>,
    source: &'static str,
    created: usize,
}

#[derive(Default)]
struct BatchPolicy {
    identities: HashMap<MatchKey, Option<i64>>,
    merchants: HashMap<String, Option<i64>>,
}

impl BatchPolicy {
    fn from_sources(sources: &[AssignmentSource], mode: MatchMode) -> Self {
        let mut policy = Self::default();
        for source in sources {
            if let Some(key) = source.row.match_key(mode) {
                merge_category(&mut policy.identities, key, source.category_id);
            }
            if !uses_counterparty_account(&source.row.kind) && !source.row.merchant.is_empty() {
                merge_category(
                    &mut policy.merchants,
                    source.row.merchant.clone(),
                    source.category_id,
                );
            }
        }
        policy
    }

    fn identity_category(&self, key: &MatchKey) -> Option<i64> {
        self.identities.get(key).copied().flatten()
    }

    fn merchant_has_category(&self, merchant: &str, category_id: i64) -> bool {
        self.merchants.get(merchant).copied().flatten() == Some(category_id)
    }
}

fn merge_category<K: std::hash::Hash + Eq>(
    categories: &mut HashMap<K, Option<i64>>,
    key: K,
    category_id: i64,
) {
    match categories.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(Some(category_id));
        }
        Entry::Occupied(mut entry) if *entry.get() != Some(category_id) => {
            entry.insert(None);
        }
        Entry::Occupied(_) => {}
    }
}

fn uses_counterparty_account(kind: &str) -> bool {
    matches!(kind, "transfer_in" | "transfer_out" | "standing_order")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssignOutcome {
    pub updated: usize,
    pub rules_created: usize,
    pub skipped_transfers: usize,
}

impl Store {
    /// Assigns every unique selected row in one SQLite transaction. Optional
    /// matching uses a stable snapshot and includes only an exact merchant +
    /// place or an exact counterparty account for bank transaction kinds.
    pub fn assign(
        &mut self,
        ids: &[i64],
        category_id: i64,
        apply_to_matching: bool,
    ) -> Result<AssignOutcome> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = self.assign_tx(ids, category_id, apply_to_matching);
        self.finish_assignment_transaction(result)
    }

    fn assign_tx(
        &mut self,
        ids: &[i64],
        category_id: i64,
        apply_to_matching: bool,
    ) -> Result<AssignOutcome> {
        let selected = self.load_selected(ids)?;
        let skipped_transfers = selected
            .iter()
            .filter(|row| row.status == "transfer")
            .count();
        let sources = selected
            .into_iter()
            .filter(|row| row.status != "transfer")
            .map(|row| AssignmentSource { row, category_id })
            .collect::<Vec<_>>();
        self.apply_assignment_sources(
            sources,
            skipped_transfers,
            apply_to_matching,
            MatchMode::Merchant,
        )
    }

    /// Confirms selected suggestions atomically. Unknown ids fail the full
    /// batch. Confirmed and unassigned selected rows are no-ops. Transfer ids
    /// are protected and reported once after input deduplication.
    pub fn confirm(&mut self, ids: &[i64], apply_to_matching: bool) -> Result<AssignOutcome> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = self.confirm_tx(ids, apply_to_matching);
        self.finish_assignment_transaction(result)
    }

    fn confirm_tx(&mut self, ids: &[i64], apply_to_matching: bool) -> Result<AssignOutcome> {
        let selected = self.load_selected(ids)?;
        let skipped_transfers = selected
            .iter()
            .filter(|row| row.status == "transfer")
            .count();
        let sources = selected
            .into_iter()
            .filter(|row| row.status == "suggested")
            .filter_map(|row| {
                row.category_id
                    .map(|category_id| AssignmentSource { row, category_id })
            })
            .collect::<Vec<_>>();
        self.apply_assignment_sources(
            sources,
            skipped_transfers,
            apply_to_matching,
            MatchMode::ExactIdentity,
        )
    }

    fn finish_assignment_transaction<T>(&mut self, result: Result<T>) -> Result<T> {
        match result {
            Ok(value) => self.commit_assignment(value),
            Err(error) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    fn commit_assignment<T>(&mut self, value: T) -> Result<T> {
        if let Err(error) = self.conn.execute_batch("COMMIT") {
            let _ = self.conn.execute_batch("ROLLBACK");
            return Err(error.into());
        }
        Ok(value)
    }

    fn load_selected(&self, ids: &[i64]) -> Result<Vec<StoredTransaction>> {
        let mut unique = HashSet::new();
        ids.iter()
            .copied()
            .filter(|id| unique.insert(*id))
            .map(|id| self.load_transaction(id))
            .collect()
    }

    fn load_transaction(&self, id: i64) -> Result<StoredTransaction> {
        self.conn
            .query_row(
                "SELECT id, kind, merchant_norm, place_norm, counterparty_iban, category_id, status FROM transactions WHERE id = ?1",
                [id],
                read_stored_transaction,
            )
            .optional()?
            .ok_or(StoreError::UnknownTransaction { id })
    }

    fn apply_assignment_sources(
        &mut self,
        sources: Vec<AssignmentSource>,
        skipped_transfers: usize,
        apply_to_matching: bool,
        mode: MatchMode,
    ) -> Result<AssignOutcome> {
        let policy = BatchPolicy::from_sources(&sources, mode);
        let selected_ids = sources
            .iter()
            .map(|source| source.row.id)
            .collect::<HashSet<_>>();
        let targets = self.matching_targets(&policy, &selected_ids, apply_to_matching, mode)?;
        let mut outcome = AssignOutcome {
            updated: 0,
            rules_created: 0,
            skipped_transfers,
        };
        let mut matching_rules = HashMap::new();
        for source in &sources {
            let learned = self.assign_one(source, &policy, mode)?;
            outcome.updated += 1;
            outcome.rules_created += learned.created;
            if let (Some(key), Some(rule)) = (learned.key, learned.matching_rule) {
                matching_rules.insert((key, source.category_id), rule);
            }
        }
        outcome.updated += self.apply_matching_targets(&targets, &matching_rules)?;
        Ok(outcome)
    }

    fn matching_targets(
        &self,
        policy: &BatchPolicy,
        selected_ids: &HashSet<i64>,
        apply_to_matching: bool,
        mode: MatchMode,
    ) -> Result<Vec<MatchingTarget>> {
        if !apply_to_matching {
            return Ok(Vec::new());
        }
        let mut statement = self.conn.prepare(
            "SELECT id, kind, merchant_norm, place_norm, counterparty_iban, category_id, status \
             FROM transactions WHERE status IN ('suggested', 'unassigned')",
        )?;
        let rows = statement.query_map([], read_stored_transaction)?;
        let mut targets = Vec::new();
        for row in rows {
            let row = row?;
            if let Some(target) = matching_target(row, policy, selected_ids, mode) {
                targets.push(target);
            }
        }
        Ok(targets)
    }

    fn assign_one(
        &mut self,
        source: &AssignmentSource,
        policy: &BatchPolicy,
        mode: MatchMode,
    ) -> Result<Learned> {
        let learned = self.learn_rules_for(source, policy, mode)?;
        self.confirm_selected(source.row.id, source.category_id, &learned)?;
        Ok(learned)
    }

    fn learn_rules_for(
        &mut self,
        source: &AssignmentSource,
        policy: &BatchPolicy,
        mode: MatchMode,
    ) -> Result<Learned> {
        if matches!(mode, MatchMode::Merchant) {
            return self.learn_assignment_rules(source, policy);
        }
        let Some(key) = source.row.match_key(mode) else {
            return Ok(Learned::none());
        };
        if policy.identity_category(&key) != Some(source.category_id) {
            return Ok(Learned::none());
        }
        match &key {
            MatchKey::CounterpartyAccount(iban) => {
                self.learn_account_rule(key.clone(), iban, source.category_id)
            }
            MatchKey::MerchantPlace { merchant, place } => self.learn_merchant_rules(
                key.clone(),
                merchant,
                place.as_deref(),
                source.category_id,
                policy,
            ),
        }
    }

    fn learn_assignment_rules(
        &mut self,
        source: &AssignmentSource,
        policy: &BatchPolicy,
    ) -> Result<Learned> {
        if source.row.merchant.is_empty() {
            return Ok(Learned::none());
        }
        let key = MatchKey::MerchantPlace {
            merchant: source.row.merchant.clone(),
            place: None,
        };
        if policy.identity_category(&key) != Some(source.category_id) {
            return Ok(Learned::none());
        }
        let (exact, exact_created) = self.insert_learned_rule(
            RuleKind::Exact,
            &source.row.merchant,
            source.row.place.as_deref(),
            source.category_id,
        )?;
        let (merchant, merchant_created) = self.insert_learned_rule(
            RuleKind::Merchant,
            &source.row.merchant,
            None,
            source.category_id,
        )?;
        Ok(Learned {
            key: Some(key),
            matching_rule: Some(MatchingRule {
                id: merchant,
                source: "merchant_rule",
            }),
            source_rules: vec![exact, merchant],
            primary_rule: Some(exact),
            source: "exact_rule",
            created: exact_created + merchant_created,
        })
    }

    fn learn_account_rule(
        &mut self,
        key: MatchKey,
        iban: &str,
        category_id: i64,
    ) -> Result<Learned> {
        let (rule, created) =
            self.insert_learned_rule(RuleKind::CounterpartyAccount, iban, None, category_id)?;
        Ok(Learned {
            key: Some(key),
            matching_rule: Some(MatchingRule {
                id: rule,
                source: "account_rule",
            }),
            source_rules: vec![rule],
            primary_rule: Some(rule),
            source: "account_rule",
            created,
        })
    }

    fn learn_merchant_rules(
        &mut self,
        key: MatchKey,
        merchant: &str,
        place: Option<&str>,
        category_id: i64,
        policy: &BatchPolicy,
    ) -> Result<Learned> {
        let (exact, exact_created) =
            self.insert_learned_rule(RuleKind::Exact, merchant, place, category_id)?;
        let merchant_rule = self.learn_broad_merchant_rule(merchant, category_id, policy)?;
        let mut source_rules = vec![exact];
        let mut created = exact_created;
        if let Some((rule, was_created)) = merchant_rule {
            source_rules.push(rule);
            created += was_created;
        }
        Ok(Learned {
            key: Some(key),
            matching_rule: Some(MatchingRule {
                id: exact,
                source: "exact_rule",
            }),
            source_rules,
            primary_rule: Some(exact),
            source: "exact_rule",
            created,
        })
    }

    fn learn_broad_merchant_rule(
        &mut self,
        merchant: &str,
        category_id: i64,
        policy: &BatchPolicy,
    ) -> Result<Option<(i64, usize)>> {
        if !policy.merchant_has_category(merchant, category_id) {
            return Ok(None);
        }
        self.insert_learned_rule(RuleKind::Merchant, merchant, None, category_id)
            .map(Some)
    }

    fn insert_learned_rule(
        &mut self,
        kind: RuleKind,
        key: &str,
        place: Option<&str>,
        category_id: i64,
    ) -> Result<(i64, usize)> {
        let place_value = place.unwrap_or("");
        let existed = self
            .conn
            .prepare("SELECT 1 FROM rules WHERE match_kind = ?1 AND key = ?2 AND place = ?3")?
            .exists(params![crate::rules_repo::kind_str(kind), key, place_value])?;
        let rule = self.insert_rule(kind, key, place, category_id)?;
        Ok((rule, usize::from(!existed)))
    }

    fn confirm_selected(&mut self, id: i64, category_id: i64, learned: &Learned) -> Result<()> {
        self.conn.execute(
            "UPDATE transactions SET status = 'confirmed', category_id = ?2, rule_id = ?3, source = ?4 \
             WHERE id = ?1 AND status <> 'transfer'",
            params![id, category_id, learned.primary_rule, learned.source],
        )?;
        for rule in &learned.source_rules {
            self.record_rule_source(*rule, id)?;
        }
        Ok(())
    }

    fn apply_matching_targets(
        &mut self,
        targets: &[MatchingTarget],
        matching_rules: &HashMap<(MatchKey, i64), MatchingRule>,
    ) -> Result<usize> {
        let mut updated = 0;
        for target in targets {
            if let Some(rule) = matching_rules.get(&(target.key.clone(), target.category_id)) {
                updated += self.confirm_matching_target(target, *rule)?;
            }
        }
        Ok(updated)
    }

    fn confirm_matching_target(
        &mut self,
        target: &MatchingTarget,
        rule: MatchingRule,
    ) -> Result<usize> {
        let updated = self.conn.execute(
            "UPDATE transactions SET status = 'confirmed', category_id = ?2, rule_id = ?3, source = ?4 \
             WHERE id = ?1 AND status IN ('suggested', 'unassigned') \
               AND (category_id IS NULL OR category_id = ?2)",
            params![target.id, target.category_id, rule.id, rule.source],
        )?;
        if updated == 1 {
            self.record_rule_source(rule.id, target.id)?;
        }
        Ok(updated)
    }

    pub fn reclassify_open(&mut self) -> Result<usize> {
        let ids: Vec<i64> = {
            let mut st = self.conn.prepare(
                "SELECT id FROM transactions WHERE status IN ('suggested', 'unassigned')",
            )?;
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

impl Learned {
    fn none() -> Self {
        Self {
            key: None,
            matching_rule: None,
            source_rules: Vec::new(),
            primary_rule: None,
            source: "none",
            created: 0,
        }
    }
}

fn matching_target(
    row: StoredTransaction,
    policy: &BatchPolicy,
    selected_ids: &HashSet<i64>,
    mode: MatchMode,
) -> Option<MatchingTarget> {
    if selected_ids.contains(&row.id) {
        return None;
    }
    let key = row.match_key(mode)?;
    let category_id = policy.identity_category(&key)?;
    if row
        .category_id
        .is_some_and(|existing| existing != category_id)
    {
        return None;
    }
    Some(MatchingTarget {
        id: row.id,
        key,
        category_id,
    })
}

fn read_stored_transaction(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredTransaction> {
    Ok(StoredTransaction {
        id: row.get(0)?,
        kind: row.get(1)?,
        merchant: row.get(2)?,
        place: row.get(3)?,
        counterparty_iban: row.get(4)?,
        category_id: row.get(5)?,
        status: row.get(6)?,
    })
}
