//! Wire types for the recurring feature (recurring-contract.md §7). Field
//! names and shapes are copied one-for-one from the contract's TypeScript
//! and Rust snippets: the TS `Direction` is named `RecurringDirection` here
//! to avoid conflating it with the unrelated `CategoryKind`.
use crate::query::TxRow;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cadence {
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurringDirection {
    Expense,
    Income,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurringState {
    Active,
    Upcoming,
    Missing,
    Ended,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownReason {
    MissingCoverage,
    AmbiguousMembership,
    NoEvidence,
    UnmatchedHistory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurringScope {
    Group,
    Selected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurringDecisionMode {
    Confirmed,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmountBasis {
    StablePair,
    LatestObservation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecurringQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub account_kind: Option<parser::AccountKind>,
    pub today: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecurringSelection {
    Group { transaction_id: i64 },
    Selected { transaction_ids: Vec<i64> },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecurringDecisionInput {
    Confirmed { cadence: Cadence, anchor_date: NaiveDate },
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRecurringRequest {
    pub decision_id: Option<i64>,
    pub selection: RecurringSelection,
    pub decision: RecurringDecisionInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecurringDecision {
    pub id: i64,
    pub series_key: String,
    pub group_key: String,
    pub account_id: i64,
    pub scope: RecurringScope,
    pub mode: RecurringDecisionMode,
    pub cadence: Option<Cadence>,
    pub anchor_date: Option<NaiveDate>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceChange {
    pub currency: String,
    pub previous_cents: i64,
    pub current_cents: i64,
    pub delta_cents: i64,
    pub effective_from: NaiveDate,
}

/// `decision` here is the row's estimate/confirmed/ignored state, distinct
/// from `RecurringState` (active/upcoming/missing/ended/unknown), which is
/// the schedule health of a confirmed or candidate entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowDecision {
    Estimate,
    Confirmed,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecurringRow {
    pub series_key: String,
    pub group_key: String,
    pub decision_id: Option<i64>,
    pub scope: RecurringScope,
    pub decision: RowDecision,
    pub account_id: i64,
    pub account_label: String,
    pub account_kind: parser::AccountKind,
    pub direction: RecurringDirection,
    pub name: String,
    pub cadence: Option<Cadence>,
    pub anchor_date: Option<NaiveDate>,
    pub subscription_category: bool,
    pub category_id: Option<i64>,
    pub evidence_count: i64,
    pub last_paid: Option<NaiveDate>,
    pub next_due: Option<NaiveDate>,
    pub state: RecurringState,
    pub unknown_reason: Option<UnknownReason>,
    pub grace_until: Option<NaiveDate>,
    pub currency: Option<String>,
    pub original_amount_cents: Option<i64>,
    pub amount_cents: Option<i64>,
    pub amount_basis: Option<AmountBasis>,
    pub monthly_cents: Option<i64>,
    pub annual_cents: Option<i64>,
    pub price_change: Option<PriceChange>,
    pub manual_membership: bool,
    pub foreign_eur_estimate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RecurringAmounts {
    pub monthly_income_cents: i64,
    pub monthly_expense_cents: i64,
    pub monthly_net_cents: i64,
    pub annual_income_cents: i64,
    pub annual_expense_cents: i64,
    pub annual_net_cents: i64,
    pub remaining_income_cents: i64,
    pub remaining_expense_cents: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RecurringExcluded {
    pub missing: i64,
    pub ended: i64,
    pub unknown: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecurringOverview {
    pub as_of: NaiveDate,
    pub history_from: Option<NaiveDate>,
    pub future_period: bool,
    pub unfinished_period: bool,
    pub rows: Vec<RecurringRow>,
    pub confirmed: RecurringAmounts,
    pub estimates: RecurringAmounts,
    pub excluded: RecurringExcluded,
    pub expense_share_basis_points: Option<i64>,
    pub average_expense_cents: Option<i64>,
    pub average_months: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecurringDetailRequest {
    pub series_key: String,
    pub query: RecurringQuery,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecurringDetail {
    pub row: RecurringRow,
    pub transactions: Vec<TxRow>,
    pub matching_transaction_ids: Vec<i64>,
    pub compatible_transactions: Vec<TxRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionRecurringContext {
    pub transaction_id: i64,
    pub group_key: Option<String>,
    pub ambiguous: bool,
    pub decision: Option<RecurringDecision>,
    pub inferred_cadence: Option<Cadence>,
    pub compatible_transactions: Vec<TxRow>,
}
