//! Pure derivations for the summary PDFs. Every money figure comes from
//! `totals::contribution`, the same rule the full report uses.
use super::totals::{contribution, narrow};
use super::{
    ReportAccount, ReportAccountTotal, ReportCategoryDelta, ReportCategoryTotal, ReportDateRange,
    ReportDay, ReportDelta, ReportMonth, ReportMonthRow, ReportStatusCounts, ReportTransaction,
};
use crate::{Result, StoreError};
use chrono::{Datelike, Duration, NaiveDate};
use rules::Status;
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// The summary lists this many largest transactions.
pub const LARGEST_TRANSACTION_COUNT: usize = 10;

/// `numerator / abs(denominator)` in tenths, rounded half away from zero.
/// `None` when the denominator is zero.
pub fn ratio_tenths(numerator: i128, denominator: i128) -> Option<i64> {
    if denominator == 0 {
        return None;
    }
    let scaled = numerator.checked_mul(1000)?;
    let divisor = denominator.checked_abs()?;
    let magnitude = (scaled.checked_abs()?.checked_mul(2)?.checked_add(divisor)?) / (2 * divisor);
    let signed = if scaled < 0 { -magnitude } else { magnitude };
    i64::try_from(signed).ok()
}

/// The immediately preceding range with the same number of days.
/// `None` when it would start before year 0001.
pub fn previous_range(current: ReportDateRange) -> Option<ReportDateRange> {
    let days = (current.to - current.from).num_days().checked_add(1)?;
    let to = current.from.pred_opt()?;
    let from = to.checked_sub_signed(Duration::try_days(days - 1)?)?;
    (from.year() >= 1).then_some(ReportDateRange { from, to })
}

pub(super) fn delta(current: i64, previous: i64) -> Result<ReportDelta> {
    let difference = i128::from(current) - i128::from(previous);
    Ok(ReportDelta {
        current_cents: current,
        previous_cents: previous,
        delta_cents: narrow(difference)?,
        percent_tenths: ratio_tenths(difference, i128::from(previous)),
    })
}

pub(super) fn category_deltas(
    current: &[ReportCategoryTotal],
    previous: &[ReportCategoryTotal],
) -> Result<Vec<ReportCategoryDelta>> {
    let mut merged: BTreeMap<Option<i64>, (String, i64, i64)> = BTreeMap::new();
    for item in previous {
        merged.insert(item.category_id, (item.label.clone(), 0, item.cents));
    }
    for item in current {
        let entry = merged
            .entry(item.category_id)
            .or_insert_with(|| (item.label.clone(), 0, 0));
        entry.0.clone_from(&item.label);
        entry.1 = item.cents;
    }
    let mut rows = merged
        .into_iter()
        .map(|(category_id, (label, now, before))| {
            Ok(ReportCategoryDelta {
                category_id,
                label,
                delta: delta(now, before)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    rows.sort_by(compare_delta);
    Ok(rows)
}

fn compare_delta(left: &ReportCategoryDelta, right: &ReportCategoryDelta) -> Ordering {
    right
        .delta
        .delta_cents
        .unsigned_abs()
        .cmp(&left.delta.delta_cents.unsigned_abs())
        .then_with(|| {
            left.category_id
                .unwrap_or(i64::MAX)
                .cmp(&right.category_id.unwrap_or(i64::MAX))
        })
        .then_with(|| left.label.cmp(&right.label))
}

pub(super) fn days(rows: &[ReportTransaction], range: ReportDateRange) -> Result<Vec<ReportDay>> {
    let mut sums: BTreeMap<NaiveDate, (i128, i128)> = range
        .from
        .iter_days()
        .take_while(|date| *date <= range.to)
        .map(|date| (date, (0, 0)))
        .collect();
    for row in rows {
        let part = contribution(row);
        if let Some(entry) = sums.get_mut(&row.transaction.tx_date) {
            entry.0 += part.income;
            entry.1 += part.expense;
        }
    }
    sums.into_iter()
        .map(|(date, (income, expense))| {
            Ok(ReportDay {
                date,
                income_cents: narrow(income)?,
                expense_cents: narrow(expense)?,
            })
        })
        .collect()
}

pub(super) fn month_rows(months: &[ReportMonth]) -> Result<Vec<ReportMonthRow>> {
    let mut running = 0_i128;
    months
        .iter()
        .map(|month| {
            let net = i128::from(month.income_cents) - i128::from(month.expense_cents);
            running += net;
            Ok(ReportMonthRow {
                month: month.month.clone(),
                income_cents: month.income_cents,
                expense_cents: month.expense_cents,
                net_cents: narrow(net)?,
                cumulative_net_cents: narrow(running)?,
            })
        })
        .collect()
}

pub(super) fn account_totals(
    accounts: &[ReportAccount],
    rows: &[ReportTransaction],
) -> Result<Vec<ReportAccountTotal>> {
    let mut sums: BTreeMap<i64, (i128, i128, u32)> = BTreeMap::new();
    for row in rows {
        let part = contribution(row);
        let entry = sums.entry(row.transaction.account_id).or_default();
        entry.0 += part.income;
        entry.1 += part.expense;
        entry.2 = entry.2.checked_add(1).ok_or_else(count_overflow)?;
    }
    accounts
        .iter()
        .map(|account| {
            let (income, expense, count) = sums.get(&account.id).copied().unwrap_or_default();
            Ok(ReportAccountTotal {
                account_id: account.id,
                label: account.label.clone(),
                income_cents: narrow(income)?,
                expense_cents: narrow(expense)?,
                net_cents: narrow(income - expense)?,
                transaction_count: count,
            })
        })
        .collect()
}

pub(super) fn status_counts(rows: &[ReportTransaction]) -> Result<ReportStatusCounts> {
    let mut counts = ReportStatusCounts {
        confirmed: 0,
        suggested: 0,
        unassigned: 0,
        transfer: 0,
    };
    for row in rows {
        let slot = match row.transaction.status {
            Status::Confirmed => &mut counts.confirmed,
            Status::Suggested => &mut counts.suggested,
            Status::Unassigned => &mut counts.unassigned,
            Status::Transfer => &mut counts.transfer,
        };
        *slot = slot.checked_add(1).ok_or_else(count_overflow)?;
    }
    Ok(counts)
}

/// Largest by absolute amount. A transfer is neither income nor expense, so
/// it is not listed.
pub(super) fn largest(rows: &[ReportTransaction], limit: usize) -> Vec<ReportTransaction> {
    let mut candidates = rows
        .iter()
        .filter(|row| row.transaction.status != Status::Transfer)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| compare_largest(left, right));
    candidates.into_iter().take(limit).cloned().collect()
}

fn compare_largest(left: &ReportTransaction, right: &ReportTransaction) -> Ordering {
    let (left, right) = (&left.transaction, &right.transaction);
    right
        .amount_cents
        .unsigned_abs()
        .cmp(&left.amount_cents.unsigned_abs())
        .then_with(|| right.tx_date.cmp(&left.tx_date))
        .then_with(|| right.id.cmp(&left.id))
}

fn count_overflow() -> StoreError {
    StoreError::Parse("Počet transakcií prekročil podporovaný rozsah.".into())
}
