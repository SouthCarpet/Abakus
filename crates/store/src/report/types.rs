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
