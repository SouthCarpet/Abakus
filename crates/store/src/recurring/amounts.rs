//! KPI aggregation (recurring-contract.md §6): confirmed/estimate totals,
//! remaining-month expected charges and the expense-share denominator.
use super::coverage::is_covered;
use super::money::basis_points;
use super::schedule;
use super::types::{Cadence, RecurringAmounts, RecurringDirection, RecurringExcluded, RecurringQuery, RecurringRow, RecurringState, RowDecision};
use crate::{Result, Store};
use chrono::{Datelike, NaiveDate};

/// Confirmed totals, estimate totals, and the exclusion counts shown beside
/// them so a zero total is never mistaken for complete knowledge.
pub(crate) fn aggregate(rows: &[RecurringRow]) -> (RecurringAmounts, RecurringAmounts, RecurringExcluded) {
    let mut confirmed = RecurringAmounts::default();
    let mut estimates = RecurringAmounts::default();
    let mut excluded = RecurringExcluded::default();
    for row in rows {
        match row.state {
            RecurringState::Missing => excluded.missing += 1,
            RecurringState::Ended => excluded.ended += 1,
            RecurringState::Unknown => excluded.unknown += 1,
            RecurringState::Active | RecurringState::Upcoming => add_row(row, &mut confirmed, &mut estimates),
        }
    }
    confirmed.monthly_net_cents = confirmed.monthly_income_cents - confirmed.monthly_expense_cents;
    confirmed.annual_net_cents = confirmed.annual_income_cents - confirmed.annual_expense_cents;
    estimates.monthly_net_cents = estimates.monthly_income_cents - estimates.monthly_expense_cents;
    estimates.annual_net_cents = estimates.annual_income_cents - estimates.annual_expense_cents;
    (confirmed, estimates, excluded)
}

fn add_row(row: &RecurringRow, confirmed: &mut RecurringAmounts, estimates: &mut RecurringAmounts) {
    let bucket = match row.decision {
        RowDecision::Confirmed => &mut *confirmed,
        RowDecision::Estimate => &mut *estimates,
        RowDecision::Ignored => return,
    };
    let (monthly, annual, amount) = (row.monthly_cents.unwrap_or(0), row.annual_cents.unwrap_or(0), row.amount_cents.unwrap_or(0));
    match row.direction {
        RecurringDirection::Income => {
            bucket.monthly_income_cents += monthly;
            bucket.annual_income_cents += annual;
        }
        RecurringDirection::Expense => {
            bucket.monthly_expense_cents += monthly;
            bucket.annual_expense_cents += annual;
        }
    }
    // Remaining-month: an `Upcoming` row's `next_due` is, by construction of
    // the state machine, the sole unmatched occurrence inside
    // `[as_of, month_end(as_of)]`, so its full observed charge (not the
    // monthly-apportioned figure) is exactly the remaining-month amount.
    if row.state == RecurringState::Upcoming {
        match row.direction {
            RecurringDirection::Income => bucket.remaining_income_cents += amount,
            RecurringDirection::Expense => bucket.remaining_expense_cents += amount,
        }
    }
}

impl Store {
    fn accounts_of_kind(&self, kind: Option<parser::AccountKind>) -> Result<Vec<i64>> {
        Ok(self.list_accounts()?.into_iter().filter(|a| kind.is_none() || Some(a.kind) == kind).map(|a| a.id).collect())
    }

    fn earliest_trusted_month_start(&self, account_ids: &[i64]) -> Result<Option<NaiveDate>> {
        if account_ids.is_empty() {
            return Ok(None);
        }
        let placeholders = account_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT MIN(period_start) FROM statements WHERE checksum_status = 'ok' AND account_id IN ({placeholders})");
        let params: Vec<&dyn rusqlite::ToSql> = account_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        let earliest: Option<String> = self.conn.query_row(&sql, params.as_slice(), |r| r.get(0))?;
        Ok(earliest.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()).map(|d| NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d)))
    }

    /// Complete calendar months, wholly inside the finite selected range
    /// (or from the earliest trusted statement for all-history) and
    /// strictly before the `as_of` month, where EVERY account of the
    /// selected kind has trusted coverage for the entire month.
    fn eligible_expense_months(&self, query: &RecurringQuery, as_of: NaiveDate) -> Result<Vec<(NaiveDate, NaiveDate)>> {
        let account_ids = self.accounts_of_kind(query.account_kind)?;
        let Some(earliest) = self.earliest_trusted_month_start(&account_ids)? else { return Ok(Vec::new()) };
        let as_of_month_start = NaiveDate::from_ymd_opt(as_of.year(), as_of.month(), 1).unwrap_or(as_of);
        let range_start = query.from.map(|d| NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d)).unwrap_or(earliest).max(earliest);
        let range_end = query.to.unwrap_or(as_of);
        let ranges: Vec<Vec<super::coverage::Range>> = account_ids.iter().map(|id| self.trusted_coverage(*id)).collect::<Result<_>>()?;
        let mut months = Vec::new();
        let mut cursor = range_start;
        while cursor < as_of_month_start {
            let month_end = schedule::month_end(cursor)?;
            if month_end <= range_end && ranges.iter().all(|r| is_covered(r, cursor, month_end)) {
                months.push((cursor, month_end));
            }
            cursor = schedule::occurrence(cursor, Cadence::Monthly, 1)?;
        }
        Ok(months)
    }

    /// `(expense_share_basis_points, average_expense_cents, average_months)`.
    /// `monthly_confirmed_expense_cents` is the already-computed confirmed
    /// recurring monthly expense total for the numerator.
    pub(crate) fn recurring_expense_share(&self, query: &RecurringQuery, as_of: NaiveDate, monthly_confirmed_expense_cents: i64) -> Result<(Option<i64>, Option<i64>, Vec<String>)> {
        let months = self.eligible_expense_months(query, as_of)?;
        if months.is_empty() {
            return Ok((None, None, Vec::new()));
        }
        let mut total: i128 = 0;
        for (start, end) in &months {
            total += self.summary_filtered(Some(*start), Some(*end), None, query.account_kind)?.expense_cents as i128;
        }
        let count = months.len() as i128;
        let avg = super::money::safe_i64((total + count / 2) / count)?;
        if avg <= 0 {
            return Ok((None, None, Vec::new()));
        }
        let bp = basis_points(monthly_confirmed_expense_cents, avg)?;
        let labels = months.iter().map(|(start, _)| format!("{:04}-{:02}", start.year(), start.month())).collect();
        Ok((bp, Some(avg), labels))
    }
}
