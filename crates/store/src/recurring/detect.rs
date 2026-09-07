//! Evidence loading, grouping and qualifying-run detection
//! (recurring-contract.md §3-4). Read-only: nothing here writes a decision.
use super::key::{self, CurrencyBasis, Identity};
use super::schedule;
use super::types::{Cadence, RecurringDirection};
use crate::{Result, Store};
use chrono::NaiveDate;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone)]
pub(crate) struct Evidence {
    pub transaction_id: i64,
    pub account_id: i64,
    pub account_kind: parser::AccountKind,
    pub account_label: String,
    pub fingerprint: String,
    pub tx_date: NaiveDate,
    pub amount_cents: i64,
    pub orig_amount_cents: Option<i64>,
    pub orig_currency: Option<String>,
    pub merchant_raw: String,
    pub place_norm: Option<String>,
    pub card_last4: Option<String>,
    pub category_id: Option<i64>,
    pub subscription_category: bool,
    pub group_key: String,
    pub direction: RecurringDirection,
    pub currency_basis: CurrencyBasis,
    pub identity: Identity,
}

/// Positive magnitude in the currency the qualifying-run and price-change
/// rules compare in: the original currency when known, EUR otherwise.
pub(crate) fn comparison_amount(e: &Evidence) -> i64 {
    match &e.currency_basis {
        CurrencyBasis::Original(_) => e.orig_amount_cents.unwrap_or(0).unsigned_abs() as i64,
        _ => e.amount_cents.unsigned_abs() as i64,
    }
}

pub(crate) fn comparison_currency(e: &Evidence) -> String {
    match &e.currency_basis {
        CurrencyBasis::Original(c) => c.clone(),
        _ => "EUR".to_string(),
    }
}

const LOAD_SQL: &str = "\
SELECT t.id, t.account_id, a.kind, a.label, t.fingerprint, t.tx_date, t.amount_cents, \
       t.orig_amount_cents, t.orig_currency, t.merchant_raw, t.merchant_norm, t.place_norm, \
       t.counterparty_name, t.counterparty_iban, t.card_last4, t.category_id, c.name, c.archived, p.name \
FROM transactions t \
JOIN accounts a ON a.id = t.account_id \
LEFT JOIN categories c ON c.id = t.category_id \
LEFT JOIN categories p ON p.id = c.parent_id \
WHERE t.status <> 'transfer' AND t.kind <> 'refund' AND t.amount_cents <> 0 AND t.tx_date <= ?1";

/// `(id, account_id, account_kind, account_label, fingerprint, tx_date,
/// amount_cents, orig_amount_cents, orig_currency)`, columns 0-8 of
/// `LOAD_SQL`. Split from `row_to_evidence` purely to keep each function's
/// cyclomatic complexity under the project's Lizard budget, the same
/// reason `query.rs` splits `row_to_tx` into `row_identity`/`row_detail`.
#[allow(clippy::type_complexity)]
fn row_core(r: &rusqlite::Row) -> rusqlite::Result<(i64, i64, parser::AccountKind, String, String, NaiveDate, i64, Option<i64>, Option<String>)> {
    let account_kind_s: String = r.get(2)?;
    let account_kind = if account_kind_s == "business" { parser::AccountKind::Business } else { parser::AccountKind::Personal };
    let tx_date_s: String = r.get(5)?;
    let tx_date = NaiveDate::parse_from_str(&tx_date_s, "%Y-%m-%d").map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e)))?;
    Ok((r.get(0)?, r.get(1)?, account_kind, r.get(3)?, r.get(4)?, tx_date, r.get(6)?, r.get(7)?, r.get(8)?))
}

/// `(merchant_raw, merchant_norm, place_norm, counterparty_name,
/// counterparty_iban, card_last4)`, columns 9-14.
#[allow(clippy::type_complexity)]
fn row_text(r: &rusqlite::Row) -> rusqlite::Result<(String, String, Option<String>, Option<String>, Option<String>, Option<String>)> {
    Ok((r.get(9)?, r.get(10)?, r.get(11)?, r.get(12)?, r.get(13)?, r.get(14)?))
}

/// `(category_id, subscription_category)`, derived from columns 15-18
/// (`category_id`, `c.name`, `c.archived`, `p.name`): the folded ROOT
/// category name (`p.name` when the transaction's category has a parent,
/// else its own name) must equal `predplatne` and the leaf must not be
/// archived.
fn row_category(r: &rusqlite::Row) -> rusqlite::Result<(Option<i64>, bool)> {
    let category_id: Option<i64> = r.get(15)?;
    let category_name: Option<String> = r.get(16)?;
    let category_archived: Option<i64> = r.get(17)?;
    let parent_name: Option<String> = r.get(18)?;
    let root_name = parent_name.or(category_name);
    let subscription_category = category_archived == Some(0) && root_name.as_deref().map(parser::fold::fold).as_deref() == Some("predplatne");
    Ok((category_id, subscription_category))
}

fn row_to_evidence(r: &rusqlite::Row) -> rusqlite::Result<Evidence> {
    let (transaction_id, account_id, account_kind, account_label, fingerprint, tx_date, amount_cents, orig_amount_cents, orig_currency) = row_core(r)?;
    let (merchant_raw, merchant_norm, place_norm, counterparty_name, counterparty_iban, card_last4) = row_text(r)?;
    let (category_id, subscription_category) = row_category(r)?;

    let direction = key::direction_of(amount_cents);
    let currency_basis = CurrencyBasis::resolve(orig_currency.as_deref(), orig_amount_cents);
    let identity = key::resolve_identity(counterparty_iban.as_deref(), counterparty_name.as_deref(), &merchant_norm);
    let group_key = key::group_key(&key::KeyInput {
        account_id,
        direction,
        currency_basis: &currency_basis,
        identity: &identity,
        place_norm: place_norm.as_deref(),
        card_last4: card_last4.as_deref(),
        fingerprint: &fingerprint,
    });

    Ok(Evidence {
        transaction_id,
        account_id,
        account_kind,
        account_label,
        fingerprint,
        tx_date,
        amount_cents,
        orig_amount_cents,
        orig_currency,
        merchant_raw,
        place_norm,
        card_last4,
        category_id,
        subscription_category,
        group_key,
        direction,
        currency_basis,
        identity,
    })
}

impl Store {
    /// Every eligible row through `as_of` (optionally narrowed to one
    /// account kind), oldest first, stable on `(tx_date, fingerprint)`
    /// regardless of SQLite row id or import order.
    pub(crate) fn recurring_load_evidence(&self, account_kind: Option<parser::AccountKind>, as_of: NaiveDate) -> Result<Vec<Evidence>> {
        let sql = match account_kind {
            Some(_) => format!("{LOAD_SQL} AND a.kind = ?2 ORDER BY t.tx_date ASC, t.fingerprint ASC"),
            None => format!("{LOAD_SQL} ORDER BY t.tx_date ASC, t.fingerprint ASC"),
        };
        let mut st = self.conn.prepare(&sql)?;
        let rows = match account_kind {
            Some(k) => st.query_map(rusqlite::params![as_of.to_string(), crate::accounts::kind_str(k)], row_to_evidence)?.collect::<std::result::Result<Vec<_>, _>>()?,
            None => st.query_map(rusqlite::params![as_of.to_string()], row_to_evidence)?.collect::<std::result::Result<Vec<_>, _>>()?,
        };
        Ok(rows)
    }

    /// Fingerprints already claimed by an explicit `selected`-scope
    /// decision: removed from automatic group evidence before inference,
    /// per recurring-contract.md §3.
    pub(crate) fn recurring_selected_fingerprints(&self) -> Result<HashSet<String>> {
        let mut st = self.conn.prepare("SELECT m.fingerprint FROM recurring_members m JOIN recurring_decisions d ON d.id = m.decision_id WHERE d.scope = 'selected'")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

/// Groups already-loaded, already `as_of`-scoped evidence by `group_key`,
/// in ascending `tx_date` order within each group (the load query is
/// already sorted, so a single pass preserves it). Fingerprints already
/// claimed by a `selected` decision never enter any group's automatic pool.
pub(crate) fn group_evidence(evidence: Vec<Evidence>, exclude_selected: &HashSet<String>) -> BTreeMap<String, Vec<Evidence>> {
    let mut groups: BTreeMap<String, Vec<Evidence>> = BTreeMap::new();
    for e in evidence {
        if exclude_selected.contains(&e.fingerprint) {
            continue;
        }
        groups.entry(e.group_key.clone()).or_default().push(e);
    }
    groups
}

struct RunSpec {
    cadence: Cadence,
    min_len: usize,
    min_days: i64,
    max_days: i64,
    month_diff: i64,
}

const RUN_SPECS: [RunSpec; 3] = [
    RunSpec { cadence: Cadence::Monthly, min_len: 3, min_days: 28, max_days: 33, month_diff: 1 },
    RunSpec { cadence: Cadence::Quarterly, min_len: 2, min_days: 85, max_days: 97, month_diff: 3 },
    RunSpec { cadence: Cadence::Yearly, min_len: 2, min_days: 355, max_days: 375, month_diff: 12 },
];

fn adjacent_pair_qualifies(a: &Evidence, b: &Evidence, spec: &RunSpec) -> bool {
    let gap = (b.tx_date - a.tx_date).num_days();
    if gap < spec.min_days || gap > spec.max_days {
        return false;
    }
    schedule::calendar_months_between(a.tx_date, b.tx_date) == spec.month_diff
}

fn run_amount_ok(window: &[Evidence]) -> bool {
    let amounts: Vec<i64> = window.iter().map(comparison_amount).collect();
    let Some(&min) = amounts.iter().min() else { return false };
    let Some(&max) = amounts.iter().max() else { return false };
    min > 0 && 100 * (max - min) <= 10 * min
}

/// The earliest index `i` such that `evidence[i..i+spec.min_len]` are all
/// pairwise adjacency-qualifying (no extra transaction of this SAME group
/// can sit between them, since the window is contiguous in the already
/// group-scoped, date-sorted slice) and within the 10% amount band.
fn earliest_run_start(evidence: &[Evidence], spec: &RunSpec) -> Option<usize> {
    if evidence.len() < spec.min_len {
        return None;
    }
    (0..=(evidence.len() - spec.min_len)).find(|&start| {
        let window = &evidence[start..start + spec.min_len];
        window.windows(2).all(|pair| adjacent_pair_qualifies(&pair[0], &pair[1], spec)) && run_amount_ok(window)
    })
}

/// `(cadence, anchor_index)` of the earliest qualifying run, monthly
/// preferred over quarterly over yearly when more than one would qualify.
/// Foreign rows with an incomplete original amount/currency
/// (`foreign_unknown`) are never auto-detected.
pub(crate) fn infer_cadence(evidence: &[Evidence]) -> Option<(Cadence, usize)> {
    if evidence.first().is_some_and(|e| e.currency_basis.is_foreign_unknown()) {
        return None;
    }
    RUN_SPECS.iter().find_map(|spec| earliest_run_start(evidence, spec).map(|start| (spec.cadence, start)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recurring::key::{CurrencyBasis, Identity};

    fn ev(date: &str, amount: i64) -> Evidence {
        Evidence {
            transaction_id: 0,
            account_id: 1,
            account_kind: parser::AccountKind::Personal,
            account_label: "Osobný".into(),
            fingerprint: date.to_string(),
            tx_date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            amount_cents: -amount,
            orig_amount_cents: None,
            orig_currency: None,
            merchant_raw: "MERCHANT".into(),
            place_norm: None,
            card_last4: None,
            category_id: None,
            subscription_category: false,
            group_key: "g".into(),
            direction: RecurringDirection::Expense,
            currency_basis: CurrencyBasis::Eur,
            identity: Identity::Merchant("merchant".into()),
        }
    }

    #[test]
    fn three_monthly_observations_thirty_one_days_apart_qualify_monthly() {
        let evidence = vec![ev("2026-01-31", 1200), ev("2026-02-28", 1200), ev("2026-03-31", 1200)];
        assert_eq!(infer_cadence(&evidence), Some((Cadence::Monthly, 0)));
    }

    #[test]
    fn two_observations_alone_give_no_monthly_candidate() {
        let evidence = vec![ev("2026-01-31", 1200), ev("2026-02-28", 1200)];
        assert_eq!(infer_cadence(&evidence), None);
    }

    #[test]
    fn weekly_purchases_are_never_thinned_into_a_monthly_run() {
        let evidence = vec![ev("2026-01-05", 5000), ev("2026-01-12", 5000), ev("2026-01-19", 5000), ev("2026-01-26", 5000), ev("2026-02-02", 5000), ev("2026-02-09", 5000)];
        assert_eq!(infer_cadence(&evidence), None, "weekly gaps never satisfy the monthly 28-33 day window");
    }

    #[test]
    fn a_price_change_greater_than_ten_percent_does_not_erase_an_already_qualifying_run() {
        let evidence = vec![ev("2026-01-31", 1000), ev("2026-02-28", 1000), ev("2026-03-31", 1000), ev("2026-04-30", 1200)];
        assert_eq!(infer_cadence(&evidence), Some((Cadence::Monthly, 0)), "the run itself already qualified before the price jump");
    }

    #[test]
    fn quarterly_before_yearly_and_the_day_gap_table_boundaries() {
        let evidence = vec![ev("2026-01-31", 3000), ev("2026-04-30", 3000)];
        assert_eq!(infer_cadence(&evidence), Some((Cadence::Quarterly, 0)));
        let too_short = vec![ev("2026-01-31", 3000), ev("2026-04-25", 3000)]; // 84 days: just outside 85-97
        assert_eq!(infer_cadence(&too_short), None);
    }

    #[test]
    fn amount_variation_over_ten_percent_fails_the_run() {
        let ok = vec![ev("2026-01-31", 1000), ev("2026-02-28", 1000), ev("2026-03-31", 1100)]; // exactly 10%
        assert_eq!(infer_cadence(&ok), Some((Cadence::Monthly, 0)));
        let bad = vec![ev("2026-01-31", 1000), ev("2026-02-28", 1000), ev("2026-03-31", 1101)]; // just over 10%
        assert_eq!(infer_cadence(&bad), None);
    }
}
