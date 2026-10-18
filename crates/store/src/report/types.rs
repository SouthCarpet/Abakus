use chrono::{DateTime, FixedOffset, NaiveDate};
use parser::AccountKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReportPeriod {
    Month { month: String },
    SixMonths { ending_month: String },
    Year { year: i32 },
    AllTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReportScope {
    All,
    Kind { account_kind: AccountKind },
    Account { account_id: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportRequest {
    pub period: ReportPeriod,
    pub scope: ReportScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDateRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportAccount {
    pub id: i64,
    pub label: String,
    pub kind: AccountKind,
    pub iban_suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportPreview {
    pub range: Option<ReportDateRange>,
    pub scope_label: String,
    pub accounts: Vec<ReportAccount>,
    pub transaction_count: u32,
    pub latest_transaction_date: Option<NaiveDate>,
    pub captured_at: String,
    pub unfinished_period: bool,
    pub accounts_without_statements: u32,
    pub accounts_with_gaps: u32,
    pub unverified_statement_count: u32,
    pub invalid_statement_range_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportClock {
    pub today: NaiveDate,
    pub captured_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportTotals {
    pub income_cents: i64,
    pub expense_cents: i64,
    pub net_cents: i64,
    pub transfer_out_cents: i64,
    pub transfer_in_cents: i64,
    pub suggested_count: u32,
    pub unassigned_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportMonth {
    pub month: String,
    pub income_cents: i64,
    pub expense_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportCategoryTotal {
    pub category_id: Option<i64>,
    pub label: String,
    pub cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportCoverage {
    pub account_id: i64,
    pub known_range: Option<ReportDateRange>,
    pub gaps: Vec<ReportDateRange>,
    pub statement_count: u32,
    pub unverified_statement_count: u32,
    pub invalid_range_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReportTransaction {
    pub transaction: crate::TxRow,
    pub account_label: String,
    pub category_kind: Option<crate::CategoryKind>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReportSnapshot {
    pub preview: ReportPreview,
    pub totals: ReportTotals,
    pub months: Vec<ReportMonth>,
    pub categories: Vec<ReportCategoryTotal>,
    pub coverage: Vec<ReportCoverage>,
    pub transactions: Vec<ReportTransaction>,
}

/// One calendar day of a single-month report (daily chart of the summary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportDay {
    pub date: NaiveDate,
    pub income_cents: i64,
    pub expense_cents: i64,
}

/// One row of the monthly table of the summary. `cumulative_net_cents` is the
/// running sum of `net_cents` from the first month of the period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportMonthRow {
    pub month: String,
    pub income_cents: i64,
    pub expense_cents: i64,
    pub net_cents: i64,
    pub cumulative_net_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportAccountTotal {
    pub account_id: i64,
    pub label: String,
    pub income_cents: i64,
    pub expense_cents: i64,
    pub net_cents: i64,
    pub transaction_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportStatusCounts {
    pub confirmed: u32,
    pub suggested: u32,
    pub unassigned: u32,
    pub transfer: u32,
}

/// Current against previous value. `percent_tenths` is tenths of a percent
/// (`458` is `45,8 %`) against `abs(previous)`; `None` when previous is zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportDelta {
    pub current_cents: i64,
    pub previous_cents: i64,
    pub delta_cents: i64,
    pub percent_tenths: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportCategoryDelta {
    pub category_id: Option<i64>,
    pub label: String,
    pub delta: ReportDelta,
}

/// The immediately preceding period with the same number of days.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPrevious {
    pub range: ReportDateRange,
    pub income: ReportDelta,
    pub expense: ReportDelta,
    pub net: ReportDelta,
    pub categories: Vec<ReportCategoryDelta>,
    pub accounts_without_statements: u32,
    pub accounts_with_gaps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportComparison {
    /// Celé obdobie has no comparison.
    AllTime,
    /// The preceding range would start before year 0001.
    OutOfRange,
    Ready(Box<ReportPrevious>),
}

/// Recurring payments of the report accounts, as the Prehľad shows them.
/// Every amount is an estimate from imports.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportRecurring {
    pub as_of: NaiveDate,
    pub confirmed: crate::recurring::RecurringAmounts,
    pub estimates: crate::recurring::RecurringAmounts,
    pub excluded: crate::recurring::RecurringExcluded,
    /// Active and upcoming rows that are not ignored, in Prehľad order.
    pub rows: Vec<crate::recurring::RecurringRow>,
}

/// Data the summaries need on top of the full report snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SummaryExtras {
    /// One entry per day, only for a single-month period; empty otherwise.
    pub days: Vec<ReportDay>,
    pub month_rows: Vec<ReportMonthRow>,
    pub account_totals: Vec<ReportAccountTotal>,
    pub status_counts: ReportStatusCounts,
    pub largest: Vec<ReportTransaction>,
    pub comparison: ReportComparison,
    pub recurring: ReportRecurring,
}

/// The report snapshot and its extras, read in one database snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SummarySnapshot {
    pub snapshot: ReportSnapshot,
    pub extras: SummaryExtras,
}
