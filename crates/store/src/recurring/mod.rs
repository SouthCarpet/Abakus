//! Recurring transactions: group identity, cadence detection, coverage and
//! persistence (recurring-contract.md). Submodules are kept small and
//! single-purpose to stay under the project's cyclomatic-complexity budget.
mod amounts;
mod coverage;
mod detect;
mod key;
mod matching;
mod money;
mod persist;
mod schedule;
pub mod types;

pub use types::*;

use detect::Evidence;
use persist::DecisionRow;
use std::collections::{BTreeSet, HashMap};

use crate::{Result, Store, StoreError};
use chrono::NaiveDate;

impl Store {
    pub fn recurring_overview(&self, query: &RecurringQuery) -> Result<RecurringOverview> {
        let today = query.today;
        let (as_of, future_period, unfinished_period) = overview_window(query, today);
        if future_period {
            return Ok(empty_overview(as_of));
        }
        let rows = self.build_rows(query, as_of)?;
        let (confirmed, estimates, excluded) = amounts::aggregate(&rows);
        let (expense_share_basis_points, average_expense_cents, average_months) = self.recurring_expense_share(query, as_of, confirmed.monthly_expense_cents)?;
        let history_from = self.recurring_earliest_evidence_date(query.account_kind)?;
        Ok(RecurringOverview { as_of, history_from, future_period, unfinished_period, rows, confirmed, estimates, excluded, expense_share_basis_points, average_expense_cents, average_months })
    }

    pub fn recurring_detail(&self, request: &RecurringDetailRequest) -> Result<RecurringDetail> {
        let today = request.query.today;
        let (as_of, _, _) = overview_window(&request.query, today);
        let all_evidence = self.recurring_load_evidence(None, as_of)?;
        let selected_fps = self.recurring_selected_fingerprints()?;
        let decisions = self.recurring_list_decisions()?;
        let (evidence, decision) = self.series_evidence(&request.series_key, &all_evidence, &selected_fps, &decisions)?;
        let group_key_value = decision.as_ref().map(|d| d.group_key.clone()).or_else(|| evidence.first().map(|e| e.group_key.clone())).unwrap_or_default();
        let row = self.build_row(&group_key_value, &evidence, decision.as_ref(), as_of)?.ok_or_else(|| StoreError::Parse("Opakovaná platba neexistuje.".into()))?;

        let all_tx = self.list_transactions(&crate::query::TxFilter { account_id: Some(row.account_id), ..Default::default() })?;
        let member_ids: std::collections::HashSet<i64> = evidence.iter().map(|e| e.transaction_id).collect();
        let transactions = period_transactions(&all_tx, &member_ids, as_of, request.query.from, request.query.to);
        let matching_transaction_ids: Vec<i64> = evidence.iter().map(|e| e.transaction_id).collect();
        let compatible_transactions = self.compatible_transactions(&row, &all_evidence, &selected_fps, &member_ids, &all_tx)?;
        Ok(RecurringDetail { row, transactions, matching_transaction_ids, compatible_transactions })
    }

    pub fn transaction_recurring_context(&self, transaction_id: i64, as_of: NaiveDate) -> Result<TransactionRecurringContext> {
        let target = persist::load_target(&self.conn, transaction_id)?;
        let all_evidence = self.recurring_load_evidence(None, as_of)?;
        let group_evidence: Vec<Evidence> = all_evidence.iter().filter(|e| e.group_key == target.group_key).cloned().collect();
        let inferred_cadence = detect::infer_cadence(&group_evidence).map(|(c, _)| c);
        let decision = self.recurring_decision_for_transaction(transaction_id, &target.group_key)?;
        let ambiguous = self.is_group_ambiguous(&decision, &group_evidence, as_of)?;
        let selected_fps = self.recurring_selected_fingerprints()?;
        let member_ids: std::collections::HashSet<i64> = group_evidence.iter().map(|e| e.transaction_id).collect();
        let account_tx = self.list_transactions(&crate::query::TxFilter { account_id: Some(target.account_id), ..Default::default() })?;
        let compatible_transactions = self.compatible_by_target(&target, &all_evidence, &selected_fps, &member_ids, &account_tx)?;
        Ok(TransactionRecurringContext {
            transaction_id,
            group_key: Some(target.group_key),
            ambiguous,
            decision: decision.as_ref().map(persist::to_public),
            inferred_cadence,
            compatible_transactions,
        })
    }
}

/// Exact membership within the query period through `as_of`
/// (recurring-contract.md §7): a full-history query (`from`/`to` both
/// `None`) keeps every member through `as_of`.
fn period_transactions(all_tx: &[crate::query::TxRow], member_ids: &std::collections::HashSet<i64>, as_of: NaiveDate, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Vec<crate::query::TxRow> {
    all_tx.iter().filter(|t| member_ids.contains(&t.id) && in_period(t.tx_date, as_of, from, to)).cloned().collect()
}

fn in_period(tx_date: NaiveDate, as_of: NaiveDate, from: Option<NaiveDate>, to: Option<NaiveDate>) -> bool {
    if tx_date > as_of {
        return false;
    }
    from.is_none_or(|f| tx_date >= f) && to.is_none_or(|t| tx_date <= t)
}

fn overview_window(query: &RecurringQuery, today: NaiveDate) -> (NaiveDate, bool, bool) {
    match (query.from, query.to) {
        (Some(from), Some(to)) => {
            let future_period = from > today;
            let unfinished_period = to >= today;
            (to.min(today), future_period, unfinished_period)
        }
        _ => (today, false, false),
    }
}

fn empty_overview(as_of: NaiveDate) -> RecurringOverview {
    RecurringOverview {
        as_of,
        history_from: None,
        future_period: true,
        unfinished_period: false,
        rows: Vec::new(),
        confirmed: RecurringAmounts::default(),
        estimates: RecurringAmounts::default(),
        excluded: RecurringExcluded::default(),
        expense_share_basis_points: None,
        average_expense_cents: None,
        average_months: Vec::new(),
    }
}

impl Store {
    fn build_rows(&self, query: &RecurringQuery, as_of: NaiveDate) -> Result<Vec<RecurringRow>> {
        let all_evidence = self.recurring_load_evidence(None, as_of)?;
        let selected_fps = self.recurring_selected_fingerprints()?;
        let decisions = self.recurring_list_decisions()?;
        let groups = detect::group_evidence(all_evidence.clone(), &selected_fps);

        let mut rows = self.group_scope_rows(&groups, &decisions, as_of)?;
        rows.extend(self.selected_scope_rows(&all_evidence, &decisions, as_of)?);
        rows.retain(|r| query.account_kind.is_none_or(|k| r.account_kind == k));
        rows.sort_by(|a, b| row_sort_key(a).cmp(&row_sort_key(b)));
        Ok(rows)
    }

    fn group_scope_rows(&self, groups: &std::collections::BTreeMap<String, Vec<Evidence>>, decisions: &[DecisionRow], as_of: NaiveDate) -> Result<Vec<RecurringRow>> {
        let mut group_keys: BTreeSet<String> = groups.keys().cloned().collect();
        for d in decisions.iter().filter(|d| d.scope == RecurringScope::Group) {
            group_keys.insert(d.group_key.clone());
        }
        let empty: Vec<Evidence> = Vec::new();
        let mut rows = Vec::new();
        for gkey in &group_keys {
            let evidence = groups.get(gkey).unwrap_or(&empty);
            let decision = decisions.iter().find(|d| d.scope == RecurringScope::Group && &d.group_key == gkey);
            if let Some(row) = self.build_row(gkey, evidence, decision, as_of)? {
                rows.push(row);
            }
        }
        Ok(rows)
    }

    fn selected_scope_rows(&self, all_evidence: &[Evidence], decisions: &[DecisionRow], as_of: NaiveDate) -> Result<Vec<RecurringRow>> {
        let by_fingerprint: HashMap<&str, &Evidence> = all_evidence.iter().map(|e| (e.fingerprint.as_str(), e)).collect();
        let mut rows = Vec::new();
        for d in decisions.iter().filter(|d| d.scope == RecurringScope::Selected) {
            let members = self.recurring_members_of(d.id)?;
            let mut evidence: Vec<Evidence> = members.iter().filter_map(|fp| by_fingerprint.get(fp.as_str()).map(|e| (*e).clone())).collect();
            evidence.sort_by(|a, b| (a.tx_date, &a.fingerprint).cmp(&(b.tx_date, &b.fingerprint)));
            if let Some(row) = self.build_row(&d.group_key, &evidence, Some(d), as_of)? {
                rows.push(row);
            }
        }
        Ok(rows)
    }

    fn build_row(&self, group_key_value: &str, evidence: &[Evidence], decision: Option<&DecisionRow>, as_of: NaiveDate) -> Result<Option<RecurringRow>> {
        let Some(plan) = row_plan(decision, evidence) else { return Ok(None) };
        let identity = self.row_identity(decision, evidence)?;
        let series_key_value = decision.map(persist::series_key).unwrap_or_else(|| format!("g:{group_key_value}"));
        let subscription_category = evidence.iter().any(|e| e.subscription_category);
        let category_id = uniform_category(evidence);

        let schedule = self.row_schedule(&plan, evidence, identity.account_id, as_of)?;
        let amount = self.row_amount(evidence, &plan)?;
        let price_change = if plan.decision == RowDecision::Ignored || evidence.first().is_some_and(|e| e.currency_basis.is_foreign_unknown()) {
            None
        } else {
            matching::detect_price_change(evidence)
        };

        Ok(Some(RecurringRow {
            series_key: series_key_value,
            group_key: group_key_value.to_string(),
            decision_id: decision.map(|d| d.id),
            scope: decision.map(|d| d.scope).unwrap_or(RecurringScope::Group),
            decision: plan.decision,
            account_id: identity.account_id,
            account_label: identity.account_label,
            account_kind: identity.account_kind,
            direction: identity.direction,
            name: identity.name,
            cadence: plan.cadence,
            anchor_date: plan.anchor_date,
            subscription_category,
            category_id,
            evidence_count: evidence.len() as i64,
            last_paid: schedule.last_paid,
            next_due: schedule.next_due,
            state: schedule.state,
            unknown_reason: schedule.unknown_reason,
            grace_until: schedule.grace_until,
            currency: amount.currency,
            original_amount_cents: amount.original_amount_cents,
            amount_cents: amount.amount_cents,
            amount_basis: amount.amount_basis,
            monthly_cents: amount.monthly_cents,
            annual_cents: amount.annual_cents,
            price_change,
            manual_membership: decision.is_some_and(|d| d.scope == RecurringScope::Selected),
            foreign_eur_estimate: amount.foreign_eur_estimate,
        }))
    }

    fn row_schedule(&self, plan: &RowPlan, evidence: &[Evidence], account_id: i64, as_of: NaiveDate) -> Result<matching::ScheduleOutcome> {
        if plan.decision == RowDecision::Ignored {
            return Ok(matching::ScheduleOutcome { last_paid: evidence.last().map(|e| e.tx_date), next_due: None, state: RecurringState::Unknown, unknown_reason: None, grace_until: None, matched_transaction_ids: Vec::new() });
        }
        let (Some(cadence), Some(anchor)) = (plan.cadence, plan.anchor_date) else {
            return Ok(matching::ScheduleOutcome { last_paid: None, next_due: None, state: RecurringState::Unknown, unknown_reason: Some(UnknownReason::NoEvidence), grace_until: None, matched_transaction_ids: Vec::new() });
        };
        let ranges = self.trusted_coverage(account_id)?;
        matching::derive_schedule(evidence, anchor, cadence, &ranges, as_of)
    }

    fn row_amount(&self, evidence: &[Evidence], plan: &RowPlan) -> Result<RowAmount> {
        if plan.decision == RowDecision::Ignored {
            return Ok(RowAmount::default());
        }
        let Some(cadence) = plan.cadence else { return Ok(RowAmount::default()) };
        let Some((idx, basis)) = matching::stable_or_latest_index(evidence) else { return Ok(RowAmount::default()) };
        let point = &evidence[idx];
        let eur_cents = point.amount_cents.unsigned_abs() as i64;
        let is_foreign = matches!(point.currency_basis, key::CurrencyBasis::Original(_));
        let original_amount_cents = if is_foreign { point.orig_amount_cents.map(|c| c.unsigned_abs() as i64) } else { None };
        let currency = if is_foreign { point.orig_currency.clone() } else { None };
        Ok(RowAmount {
            currency,
            original_amount_cents,
            amount_cents: Some(eur_cents),
            amount_basis: Some(basis),
            monthly_cents: Some(money::monthly_apportionment_cents(eur_cents, cadence)?),
            annual_cents: Some(money::annual_projection_cents(eur_cents, cadence)?),
            foreign_eur_estimate: is_foreign,
        })
    }

    fn row_identity(&self, decision: Option<&DecisionRow>, evidence: &[Evidence]) -> Result<RowIdentity> {
        if let Some(e) = evidence.first() {
            return Ok(RowIdentity {
                direction: e.direction,
                account_id: e.account_id,
                account_kind: e.account_kind,
                account_label: e.account_label.clone(),
                name: e.merchant_raw.clone(),
            });
        }
        if let Some(d) = decision {
            let account = self.account_by_id(d.account_id)?.ok_or(StoreError::UnknownAccountId { id: d.account_id })?;
            return Ok(RowIdentity { direction: d.identity.direction, account_id: d.account_id, account_kind: account.kind, account_label: account.label, name: d.identity.name.clone() });
        }
        Err(StoreError::Parse("recurring: chýba identita pre zobrazenie riadku".into()))
    }

    fn series_evidence(&self, series_key_value: &str, all_evidence: &[Evidence], selected_fps: &std::collections::HashSet<String>, decisions: &[DecisionRow]) -> Result<(Vec<Evidence>, Option<DecisionRow>)> {
        if let Some(id_str) = series_key_value.strip_prefix("s:") {
            let id: i64 = id_str.parse().map_err(|_| StoreError::Parse("neplatný series_key".into()))?;
            let decision = decisions.iter().find(|d| d.id == id).cloned().ok_or_else(|| StoreError::Parse("Opakovaná platba neexistuje.".into()))?;
            let members = self.recurring_members_of(id)?;
            let by_fp: HashMap<&str, &Evidence> = all_evidence.iter().map(|e| (e.fingerprint.as_str(), e)).collect();
            let mut evidence: Vec<Evidence> = members.iter().filter_map(|fp| by_fp.get(fp.as_str()).map(|e| (*e).clone())).collect();
            evidence.sort_by(|a, b| (a.tx_date, &a.fingerprint).cmp(&(b.tx_date, &b.fingerprint)));
            return Ok((evidence, Some(decision)));
        }
        let group_key_value = series_key_value.strip_prefix("g:").unwrap_or(series_key_value);
        let evidence: Vec<Evidence> = all_evidence.iter().filter(|e| e.group_key == group_key_value && !selected_fps.contains(&e.fingerprint)).cloned().collect();
        let decision = decisions.iter().find(|d| d.scope == RecurringScope::Group && d.group_key == group_key_value).cloned();
        Ok((evidence, decision))
    }

    fn recurring_decision_for_transaction(&self, transaction_id: i64, group_key_value: &str) -> Result<Option<DecisionRow>> {
        let fp: Option<String> = self.conn.query_row("SELECT fingerprint FROM transactions WHERE id = ?1", [transaction_id], |r| r.get(0)).ok();
        if let Some(fp) = fp {
            let mut st = self.conn.prepare("SELECT decision_id FROM recurring_members WHERE fingerprint = ?1")?;
            let found: Option<i64> = st.query_map([&fp], |r| r.get::<_, i64>(0))?.next().transpose()?;
            if let Some(id) = found {
                return self.recurring_decision_by_id(id);
            }
        }
        self.recurring_decision_by_group(group_key_value)
    }

    fn is_group_ambiguous(&self, decision: &Option<DecisionRow>, group_evidence: &[Evidence], as_of: NaiveDate) -> Result<bool> {
        let plan = match decision {
            Some(d) if d.mode == RecurringDecisionMode::Confirmed => (d.cadence, d.anchor_date),
            _ => match detect::infer_cadence(group_evidence) {
                Some((c, idx)) => (Some(c), Some(group_evidence[idx].tx_date)),
                None => return Ok(false),
            },
        };
        let (Some(cadence), Some(anchor)) = plan else { return Ok(false) };
        let Some(account_id) = group_evidence.first().map(|e| e.account_id) else { return Ok(false) };
        let ranges = self.trusted_coverage(account_id)?;
        let outcome = matching::derive_schedule(group_evidence, anchor, cadence, &ranges, as_of)?;
        Ok(outcome.unknown_reason == Some(UnknownReason::AmbiguousMembership))
    }

    fn compatible_transactions(
        &self,
        row: &RecurringRow,
        all_evidence: &[Evidence],
        selected_fps: &std::collections::HashSet<String>,
        member_ids: &std::collections::HashSet<i64>,
        account_tx: &[crate::query::TxRow],
    ) -> Result<Vec<crate::query::TxRow>> {
        let candidate_ids: std::collections::HashSet<i64> = all_evidence
            .iter()
            .filter(|e| e.account_id == row.account_id && e.direction == row.direction && !member_ids.contains(&e.transaction_id) && !selected_fps.contains(&e.fingerprint))
            .map(|e| e.transaction_id)
            .collect();
        Ok(account_tx.iter().filter(|t| candidate_ids.contains(&t.id)).cloned().collect())
    }

    fn compatible_by_target(
        &self,
        target: &persist::TargetTx,
        all_evidence: &[Evidence],
        selected_fps: &std::collections::HashSet<String>,
        member_ids: &std::collections::HashSet<i64>,
        account_tx: &[crate::query::TxRow],
    ) -> Result<Vec<crate::query::TxRow>> {
        let candidate_ids: std::collections::HashSet<i64> = all_evidence
            .iter()
            .filter(|e| e.account_id == target.account_id && e.direction == target.identity.direction && !member_ids.contains(&e.transaction_id) && !selected_fps.contains(&e.fingerprint))
            .map(|e| e.transaction_id)
            .collect();
        Ok(account_tx.iter().filter(|t| candidate_ids.contains(&t.id)).cloned().collect())
    }

    fn recurring_earliest_evidence_date(&self, account_kind: Option<parser::AccountKind>) -> Result<Option<NaiveDate>> {
        let sql = match account_kind {
            Some(_) => "SELECT MIN(t.tx_date) FROM transactions t JOIN accounts a ON a.id = t.account_id WHERE t.status <> 'transfer' AND t.kind <> 'refund' AND t.amount_cents <> 0 AND a.kind = ?1",
            None => "SELECT MIN(t.tx_date) FROM transactions t WHERE t.status <> 'transfer' AND t.kind <> 'refund' AND t.amount_cents <> 0",
        };
        let earliest: Option<String> = match account_kind {
            Some(k) => self.conn.query_row(sql, [crate::accounts::kind_str(k)], |r| r.get(0))?,
            None => self.conn.query_row(sql, [], |r| r.get(0))?,
        };
        Ok(earliest.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()))
    }
}

struct RowIdentity {
    direction: RecurringDirection,
    account_id: i64,
    account_kind: parser::AccountKind,
    account_label: String,
    name: String,
}

#[derive(Default)]
struct RowAmount {
    currency: Option<String>,
    original_amount_cents: Option<i64>,
    amount_cents: Option<i64>,
    amount_basis: Option<AmountBasis>,
    monthly_cents: Option<i64>,
    annual_cents: Option<i64>,
    foreign_eur_estimate: bool,
}

struct RowPlan {
    decision: RowDecision,
    cadence: Option<Cadence>,
    anchor_date: Option<NaiveDate>,
}

/// `None` means this group has neither a stored decision nor a qualifying
/// inferred run: it must not appear as a row at all.
fn row_plan(decision: Option<&DecisionRow>, evidence: &[Evidence]) -> Option<RowPlan> {
    match decision {
        Some(d) if d.mode == RecurringDecisionMode::Ignored => Some(RowPlan { decision: RowDecision::Ignored, cadence: None, anchor_date: None }),
        Some(d) => Some(RowPlan { decision: RowDecision::Confirmed, cadence: d.cadence, anchor_date: d.anchor_date }),
        None => detect::infer_cadence(evidence).map(|(cadence, idx)| RowPlan { decision: RowDecision::Estimate, cadence: Some(cadence), anchor_date: Some(evidence[idx].tx_date) }),
    }
}

fn uniform_category(evidence: &[Evidence]) -> Option<i64> {
    let first = evidence.first()?.category_id?;
    if evidence.iter().all(|e| e.category_id == Some(first)) {
        Some(first)
    } else {
        None
    }
}

/// direction expense/income; decision confirmed/estimate/ignored; name
/// folded; account_id; series_key (recurring-contract.md §7).
fn row_sort_key(r: &RecurringRow) -> (u8, u8, String, i64, String) {
    let direction_rank = match r.direction {
        RecurringDirection::Expense => 0,
        RecurringDirection::Income => 1,
    };
    let decision_rank = match r.decision {
        RowDecision::Confirmed => 0,
        RowDecision::Estimate => 1,
        RowDecision::Ignored => 2,
    };
    (direction_rank, decision_rank, parser::fold::fold(&r.name), r.account_id, r.series_key.clone())
}
