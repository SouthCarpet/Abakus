//! `recurring_decisions`/`recurring_members` persistence
//! (recurring-contract.md §7-8). Group-scope membership is never stored: it
//! is whatever currently matches `group_key`. Only `selected`-scope
//! membership is stored, by fingerprint (no transaction FK), so deleting or
//! reimporting a statement can never delete a decision or its membership.
use super::key::{group_key, resolve_identity, CurrencyBasis};
use super::types::{Cadence, RecurringDecision, RecurringDecisionInput, RecurringDecisionMode, RecurringScope, RecurringSelection, SaveRecurringRequest};
use crate::{Result, Store, StoreError};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

fn cadence_str(c: Cadence) -> &'static str {
    match c {
        Cadence::Monthly => "monthly",
        Cadence::Quarterly => "quarterly",
        Cadence::Yearly => "yearly",
    }
}
fn cadence_parse(s: &str) -> Cadence {
    match s {
        "quarterly" => Cadence::Quarterly,
        "yearly" => Cadence::Yearly,
        _ => Cadence::Monthly,
    }
}
fn scope_str(s: RecurringScope) -> &'static str {
    match s {
        RecurringScope::Group => "group",
        RecurringScope::Selected => "selected",
    }
}
fn scope_parse(s: &str) -> RecurringScope { if s == "selected" { RecurringScope::Selected } else { RecurringScope::Group } }
fn mode_str(m: RecurringDecisionMode) -> &'static str {
    match m {
        RecurringDecisionMode::Confirmed => "confirmed",
        RecurringDecisionMode::Ignored => "ignored",
    }
}
fn mode_parse(s: &str) -> RecurringDecisionMode { if s == "ignored" { RecurringDecisionMode::Ignored } else { RecurringDecisionMode::Confirmed } }

/// Snapshot of display identity (recurring-contract.md §8): enough to show
/// a saved row with no surviving observations. Not money, not inference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct IdentitySnapshot {
    pub direction: super::types::RecurringDirection,
    pub currency: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DecisionRow {
    pub id: i64,
    pub account_id: i64,
    pub group_key: String,
    pub scope: RecurringScope,
    pub mode: RecurringDecisionMode,
    pub cadence: Option<Cadence>,
    pub anchor_date: Option<NaiveDate>,
    pub identity: IdentitySnapshot,
    pub updated_at: String,
}

pub(crate) fn series_key(row: &DecisionRow) -> String {
    match row.scope {
        RecurringScope::Group => format!("g:{}", row.group_key),
        RecurringScope::Selected => format!("s:{}", row.id),
    }
}

fn row_to_decision(r: &rusqlite::Row) -> rusqlite::Result<DecisionRow> {
    let scope: String = r.get(3)?;
    let mode: String = r.get(4)?;
    let cadence: Option<String> = r.get(5)?;
    let anchor: Option<String> = r.get(6)?;
    let identity_json: String = r.get(7)?;
    let identity: IdentitySnapshot = serde_json::from_str(&identity_json).unwrap_or(IdentitySnapshot { direction: super::types::RecurringDirection::Expense, currency: None, name: String::new() });
    Ok(DecisionRow {
        id: r.get(0)?,
        account_id: r.get(1)?,
        group_key: r.get(2)?,
        scope: scope_parse(&scope),
        mode: mode_parse(&mode),
        cadence: cadence.as_deref().map(cadence_parse),
        anchor_date: anchor.as_deref().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        identity,
        updated_at: r.get(8)?,
    })
}

const SELECT_DECISION: &str = "SELECT id, account_id, group_key, scope, mode, cadence, anchor_date, identity_json, updated_at FROM recurring_decisions";

impl Store {
    pub(crate) fn recurring_list_decisions(&self) -> Result<Vec<DecisionRow>> {
        let mut st = self.conn.prepare(SELECT_DECISION)?;
        let rows = st.query_map([], row_to_decision)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub(crate) fn recurring_decision_by_group(&self, group_key: &str) -> Result<Option<DecisionRow>> {
        let mut st = self.conn.prepare(&format!("{SELECT_DECISION} WHERE group_key = ?1 AND scope = 'group'"))?;
        let row = st.query_map([group_key], row_to_decision)?.next().transpose()?;
        Ok(row)
    }

    pub(crate) fn recurring_decision_by_id(&self, id: i64) -> Result<Option<DecisionRow>> {
        let mut st = self.conn.prepare(&format!("{SELECT_DECISION} WHERE id = ?1"))?;
        let row = st.query_map([id], row_to_decision)?.next().transpose()?;
        Ok(row)
    }

    pub(crate) fn recurring_members_of(&self, decision_id: i64) -> Result<Vec<String>> {
        let mut st = self.conn.prepare("SELECT fingerprint FROM recurring_members WHERE decision_id = ?1")?;
        let rows = st.query_map([decision_id], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn save_recurring(&mut self, request: &SaveRecurringRequest) -> Result<RecurringDecision> {
        let tx = self.conn.transaction()?;
        let outcome = save_recurring_tx(&tx, request);
        match outcome {
            Ok(row) => {
                tx.commit()?;
                Ok(row)
            }
            Err(e) => Err(e),
        }
    }

    /// A zero-effect reset (unknown id) is a clear error, not a reported
    /// success.
    pub fn reset_recurring(&mut self, decision_id: i64) -> Result<()> {
        let n = self.conn.execute("DELETE FROM recurring_decisions WHERE id = ?1", [decision_id])?;
        if n == 0 {
            return Err(StoreError::Parse(format!("Rozhodnutie s id {decision_id} neexistuje.")));
        }
        Ok(())
    }
}

pub(crate) struct TargetTx {
    pub account_id: i64,
    pub fingerprint: String,
    pub group_key: String,
    pub identity: IdentitySnapshot,
    pub identity_kind: super::key::Identity,
    pub currency_basis: CurrencyBasis,
}

/// `(id, account_id, fingerprint, amount_cents, orig_amount_cents,
/// orig_currency, merchant_norm)`, columns 0-6 of `load_target`'s query.
#[allow(clippy::type_complexity)]
fn target_core(r: &rusqlite::Row) -> rusqlite::Result<(i64, i64, String, i64, Option<i64>, Option<String>, String)> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
}

/// `(place_norm, card_last4, counterparty_name, counterparty_iban,
/// merchant_raw, status, kind)`, columns 7-13.
#[allow(clippy::type_complexity)]
fn target_rest(r: &rusqlite::Row) -> rusqlite::Result<(Option<String>, Option<String>, Option<String>, Option<String>, String, String, String)> {
    Ok((r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?, r.get(12)?, r.get(13)?))
}

fn currency_of(basis: &CurrencyBasis) -> Option<String> {
    match basis {
        CurrencyBasis::Original(c) => Some(c.clone()),
        _ => None,
    }
}

pub(crate) fn load_target(conn: &rusqlite::Connection, transaction_id: i64) -> Result<TargetTx> {
    let row = conn
        .query_row(
            "SELECT t.id, t.account_id, t.fingerprint, t.amount_cents, t.orig_amount_cents, t.orig_currency, t.merchant_norm, t.place_norm, t.card_last4, t.counterparty_name, t.counterparty_iban, t.merchant_raw, t.status, t.kind \
             FROM transactions t WHERE t.id = ?1",
            [transaction_id],
            |r| Ok((target_core(r)?, target_rest(r)?)),
        )
        .map_err(|_| StoreError::UnknownTransaction { id: transaction_id })?;
    let ((id, account_id, fingerprint, amount_cents, orig_amount_cents, orig_currency, merchant_norm), (place_norm, card_last4, counterparty_name, counterparty_iban, merchant_raw, status, kind)) = row;
    if status == "transfer" || kind == "refund" || amount_cents == 0 {
        return Err(StoreError::Parse(format!("Transakcia {id} nie je oprávnená pre opakovanú platbu (prevod, refundácia alebo nulová suma).")));
    }
    let direction = super::key::direction_of(amount_cents);
    let currency_basis = CurrencyBasis::resolve(orig_currency.as_deref(), orig_amount_cents);
    let identity_kind = resolve_identity(&kind, counterparty_iban.as_deref(), counterparty_name.as_deref(), &merchant_norm);
    let key = group_key(&super::key::KeyInput { account_id, direction, currency_basis: &currency_basis, identity: &identity_kind, place_norm: place_norm.as_deref(), card_last4: card_last4.as_deref(), fingerprint: &fingerprint });
    let currency = currency_of(&currency_basis);
    Ok(TargetTx {
        account_id,
        fingerprint,
        group_key: key,
        identity: IdentitySnapshot { direction, currency, name: merchant_raw },
        identity_kind,
        currency_basis,
    })
}

/// Manual members share the exact nonblank group. Blank rows are compatible
/// only with other blank rows on the same account, direction and currency.
pub(crate) fn compatible(a: &TargetTx, b: &TargetTx) -> bool {
    let same_basis = a.account_id == b.account_id
        && a.identity.direction == b.identity.direction
        && a.currency_basis == b.currency_basis;
    same_basis
        && (a.group_key == b.group_key
            || matches!((&a.identity_kind, &b.identity_kind), (super::key::Identity::Blank, super::key::Identity::Blank)))
}

pub(crate) fn compatible_evidence(target: &TargetTx, evidence: &super::detect::Evidence) -> bool {
    let same_basis = target.account_id == evidence.account_id
        && target.identity.direction == evidence.direction
        && target.currency_basis == evidence.currency_basis;
    same_basis
        && (target.group_key == evidence.group_key
            || matches!((&target.identity_kind, &evidence.identity), (super::key::Identity::Blank, super::key::Identity::Blank)))
}

fn validate_selection_targets(targets: &[TargetTx]) -> Result<()> {
    let Some(first) = targets.first() else {
        return Err(StoreError::Parse("Výber neobsahuje žiadnu transakciu.".into()));
    };
    if targets.iter().any(|t| !compatible(first, t)) {
        return Err(StoreError::Parse("Vybrané transakcie nepatria do rovnakého účtu, smeru a meny.".into()));
    }
    Ok(())
}

fn other_selected_fingerprints(tx: &rusqlite::Transaction, exclude_decision_id: Option<i64>) -> Result<std::collections::HashSet<String>> {
    let mut st = tx.prepare("SELECT m.fingerprint FROM recurring_members m JOIN recurring_decisions d ON d.id = m.decision_id WHERE d.scope = 'selected' AND (?1 IS NULL OR d.id <> ?1)")?;
    let rows = st.query_map([exclude_decision_id], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

fn decision_input_parts(input: &RecurringDecisionInput) -> Result<(RecurringDecisionMode, Option<Cadence>, Option<NaiveDate>)> {
    match input {
        RecurringDecisionInput::Confirmed { cadence, anchor_date } => {
            if anchor_date.year() < 1 || anchor_date.year() > 9999 {
                return Err(StoreError::Parse("Dátum ukotvenia je mimo podporovaného rozsahu.".into()));
            }
            Ok((RecurringDecisionMode::Confirmed, Some(*cadence), Some(*anchor_date)))
        }
        RecurringDecisionInput::Ignored => Ok((RecurringDecisionMode::Ignored, None, None)),
    }
}

fn existing_for_update(tx: &rusqlite::Transaction, decision_id: Option<i64>) -> Result<Option<DecisionRow>> {
    let Some(id) = decision_id else { return Ok(None) };
    let mut st = tx.prepare(&format!("{SELECT_DECISION} WHERE id = ?1"))?;
    let row = st.query_map([id], row_to_decision)?.next().transpose()?;
    row.ok_or_else(|| StoreError::Parse(format!("Rozhodnutie s id {id} neexistuje.")))
        .map(Some)
}

fn load_selection_targets(tx: &rusqlite::Transaction, selection: &RecurringSelection) -> Result<Vec<TargetTx>> {
    match selection {
        RecurringSelection::Group { transaction_id } => Ok(vec![load_target(tx, *transaction_id)?]),
        RecurringSelection::Selected { transaction_ids } => {
            if transaction_ids.is_empty() {
                return Err(StoreError::Parse("Výber neobsahuje žiadnu transakciu.".into()));
            }
            transaction_ids.iter().map(|id| load_target(tx, *id)).collect()
        }
    }
}

fn scope_of(selection: &RecurringSelection) -> RecurringScope {
    match selection {
        RecurringSelection::Group { .. } => RecurringScope::Group,
        RecurringSelection::Selected { .. } => RecurringScope::Selected,
    }
}

fn existing_selected_reference(tx: &rusqlite::Transaction, existing: &DecisionRow) -> Result<Option<TargetTx>> {
    if existing.scope != RecurringScope::Selected {
        return Ok(None);
    }
    let id: Option<i64> = tx
        .query_row(
            "SELECT t.id FROM recurring_members m JOIN transactions t ON t.fingerprint = m.fingerprint WHERE m.decision_id = ?1 ORDER BY m.fingerprint LIMIT 1",
            [existing.id],
            |row| row.get(0),
        )
        .ok();
    id.map(|id| load_target(tx, id)).transpose()
}

fn check_existing_compatible(tx: &rusqlite::Transaction, existing: &Option<DecisionRow>, targets: &[TargetTx]) -> Result<()> {
    let Some(existing) = existing else { return Ok(()) };
    if existing.account_id != targets[0].account_id {
        return Err(StoreError::Parse("Úprava nesmie zmeniť účet existujúceho rozhodnutia.".into()));
    }
    let compatible_group = match existing_selected_reference(tx, existing)? {
        Some(reference) => targets.iter().all(|target| compatible(&reference, target)),
        None => targets.iter().all(|target| target.group_key == existing.group_key),
    };
    if !compatible_group {
        return Err(StoreError::Parse("Úprava nesmie zmeniť skupinovú identitu existujúceho rozhodnutia.".into()));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_decision(
    tx: &rusqlite::Transaction,
    scope: RecurringScope,
    decision_id: Option<i64>,
    group_key_value: &str,
    account_id: i64,
    mode: RecurringDecisionMode,
    cadence: Option<Cadence>,
    anchor_date: Option<NaiveDate>,
    identity_json: &str,
    targets: &[TargetTx],
) -> Result<i64> {
    validate_member_collisions(tx, scope, decision_id, targets)?;
    let id = decision_row_id(tx, scope, decision_id, group_key_value, account_id, mode, cadence, anchor_date, identity_json)?;
    replace_members(tx, id, scope, targets)?;
    Ok(id)
}

fn validate_member_collisions(tx: &rusqlite::Transaction, scope: RecurringScope, decision_id: Option<i64>, targets: &[TargetTx]) -> Result<()> {
    if scope == RecurringScope::Selected {
        let others = other_selected_fingerprints(tx, decision_id)?;
        if targets.iter().any(|t| others.contains(&t.fingerprint)) {
            return Err(StoreError::Parse("Transakcia je už súčasťou iného ručného výberu.".into()));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn decision_row_id(
    tx: &rusqlite::Transaction,
    scope: RecurringScope,
    decision_id: Option<i64>,
    group_key_value: &str,
    account_id: i64,
    mode: RecurringDecisionMode,
    cadence: Option<Cadence>,
    anchor_date: Option<NaiveDate>,
    identity_json: &str,
) -> Result<i64> {
    let id = match decision_id {
        Some(id) => update_decision(tx, id, scope, mode, cadence, anchor_date, identity_json)?,
        None if scope == RecurringScope::Group => {
            upsert_group_decision(tx, group_key_value, account_id, mode, cadence, anchor_date, identity_json)?
        }
        None => insert_selected_decision(tx, group_key_value, account_id, mode, cadence, anchor_date, identity_json)?,
    };
    Ok(id)
}

fn replace_members(tx: &rusqlite::Transaction, id: i64, scope: RecurringScope, targets: &[TargetTx]) -> Result<()> {
    tx.execute("DELETE FROM recurring_members WHERE decision_id = ?1", [id])?;
    if scope == RecurringScope::Selected {
        for target in targets {
            tx.execute(
                "INSERT INTO recurring_members (decision_id, fingerprint) VALUES (?1, ?2)",
                rusqlite::params![id, target.fingerprint],
            )?;
        }
    }
    Ok(())
}

fn save_recurring_tx(tx: &rusqlite::Transaction, request: &SaveRecurringRequest) -> Result<RecurringDecision> {
    let existing = existing_for_update(tx, request.decision_id)?;
    let targets = load_selection_targets(tx, &request.selection)?;
    validate_selection_targets(&targets)?;
    let account_id = targets[0].account_id;
    let scope = scope_of(&request.selection);
    if scope == RecurringScope::Group && matches!(&targets[0].identity_kind, super::key::Identity::Blank) {
        return Err(StoreError::Parse("Transakcia bez obchodníka vyžaduje ručný výber.".into()));
    }
    check_existing_compatible(tx, &existing, &targets)?;

    let (mode, cadence, anchor_date) = decision_input_parts(&request.decision)?;
    let group_key_value = existing.as_ref().map(|row| row.group_key.clone()).unwrap_or_else(|| targets[0].group_key.clone());
    let identity_json = serde_json::to_string(&targets[0].identity).map_err(|e| StoreError::Parse(e.to_string()))?;
    let id = write_decision(tx, scope, request.decision_id, &group_key_value, account_id, mode, cadence, anchor_date, &identity_json, &targets)?;

    let row = tx
        .query_row(&format!("{SELECT_DECISION} WHERE id = ?1"), [id], row_to_decision)
        .map_err(|_| StoreError::Db("rozhodnutie zmizlo po uložení".into()))?;
    Ok(to_public(&row))
}

fn upsert_group_decision(tx: &rusqlite::Transaction, group_key: &str, account_id: i64, mode: RecurringDecisionMode, cadence: Option<Cadence>, anchor_date: Option<NaiveDate>, identity_json: &str) -> Result<i64> {
    tx.execute(
        "INSERT INTO recurring_decisions (account_id, group_key, scope, mode, cadence, anchor_date, identity_json, updated_at) VALUES (?1, ?2, 'group', ?3, ?4, ?5, ?6, datetime('now')) \
         ON CONFLICT(group_key) WHERE scope='group' DO UPDATE SET mode = excluded.mode, cadence = excluded.cadence, anchor_date = excluded.anchor_date, identity_json = excluded.identity_json, updated_at = datetime('now')",
        rusqlite::params![account_id, group_key, mode_str(mode), cadence.map(cadence_str), anchor_date.map(|d| d.to_string()), identity_json],
    )?;
    Ok(tx.query_row("SELECT id FROM recurring_decisions WHERE group_key = ?1 AND scope = 'group'", [group_key], |r| r.get(0))?)
}

#[allow(clippy::too_many_arguments)]
fn insert_selected_decision(
    tx: &rusqlite::Transaction,
    group_key: &str,
    account_id: i64,
    mode: RecurringDecisionMode,
    cadence: Option<Cadence>,
    anchor_date: Option<NaiveDate>,
    identity_json: &str,
) -> Result<i64> {
    tx.execute(
        "INSERT INTO recurring_decisions (account_id, group_key, scope, mode, cadence, anchor_date, identity_json, updated_at) VALUES (?1, ?2, 'selected', ?3, ?4, ?5, ?6, datetime('now'))",
        rusqlite::params![account_id, group_key, mode_str(mode), cadence.map(cadence_str), anchor_date.map(|d| d.to_string()), identity_json],
    )?;
    Ok(tx.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
fn update_decision(
    tx: &rusqlite::Transaction,
    id: i64,
    scope: RecurringScope,
    mode: RecurringDecisionMode,
    cadence: Option<Cadence>,
    anchor_date: Option<NaiveDate>,
    identity_json: &str,
) -> Result<i64> {
    let n = tx.execute(
        "UPDATE recurring_decisions SET scope = ?2, mode = ?3, cadence = ?4, anchor_date = ?5, identity_json = ?6, updated_at = datetime('now') WHERE id = ?1",
        rusqlite::params![id, scope_str(scope), mode_str(mode), cadence.map(cadence_str), anchor_date.map(|d| d.to_string()), identity_json],
    )?;
    if n != 1 {
        return Err(StoreError::Parse(format!("Rozhodnutie s id {id} neexistuje.")));
    }
    Ok(id)
}

pub(crate) fn to_public(row: &DecisionRow) -> RecurringDecision {
    RecurringDecision {
        id: row.id,
        series_key: series_key(row),
        group_key: row.group_key.clone(),
        account_id: row.account_id,
        scope: row.scope,
        mode: row.mode,
        cadence: row.cadence,
        anchor_date: row.anchor_date,
        updated_at: row.updated_at.clone(),
    }
}
