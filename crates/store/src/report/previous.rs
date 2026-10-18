//! Comparison with the immediately preceding period of the same length.
use super::capture::{read_statements, read_transactions};
use super::coverage::coverage_for;
use super::digest::{category_deltas, delta, previous_range};
use super::totals::calculate;
use super::{
    ReportAccount, ReportCaptureLimits, ReportComparison, ReportDateRange, ReportPeriod,
    ReportPrevious, ReportRequest, ReportSnapshot,
};
use crate::Result;
use rusqlite::Connection;

pub(super) fn comparison(
    conn: &Connection,
    request: &ReportRequest,
    snapshot: &ReportSnapshot,
    limits: ReportCaptureLimits,
) -> Result<ReportComparison> {
    let Some(current) = finite_range(request, snapshot) else {
        return Ok(ReportComparison::AllTime);
    };
    let Some(range) = previous_range(current) else {
        return Ok(ReportComparison::OutOfRange);
    };
    let accounts = &snapshot.preview.accounts;
    let rows = read_transactions(conn, accounts, Some(range), limits)?;
    let (totals, _months, categories) = calculate(&rows, Some(range))?;
    let (without_statements, with_gaps) = coverage_counts(conn, accounts, range)?;
    Ok(ReportComparison::Ready(Box::new(ReportPrevious {
        range,
        income: delta(snapshot.totals.income_cents, totals.income_cents)?,
        expense: delta(snapshot.totals.expense_cents, totals.expense_cents)?,
        net: delta(snapshot.totals.net_cents, totals.net_cents)?,
        categories: category_deltas(&snapshot.categories, &categories)?,
        accounts_without_statements: without_statements,
        accounts_with_gaps: with_gaps,
    })))
}

fn finite_range(request: &ReportRequest, snapshot: &ReportSnapshot) -> Option<ReportDateRange> {
    if request.period == ReportPeriod::AllTime {
        return None;
    }
    snapshot.preview.range
}

fn coverage_counts(
    conn: &Connection,
    accounts: &[ReportAccount],
    range: ReportDateRange,
) -> Result<(u32, u32)> {
    let statements = read_statements(conn, accounts)?;
    let mut without_statements = 0_u32;
    let mut with_gaps = 0_u32;
    for account in accounts {
        let coverage = coverage_for(account.id, &statements, Some(range))?;
        without_statements += u32::from(coverage.statement_count == 0);
        with_gaps += u32::from(!coverage.gaps.is_empty());
    }
    Ok((without_statements, with_gaps))
}
