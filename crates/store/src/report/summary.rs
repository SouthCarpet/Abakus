//! The extras of the summary PDFs, read in the same database snapshot as the
//! full report.
use super::capture::capture;
use super::digest::{
    account_totals, days, largest, month_rows, status_counts, LARGEST_TRANSACTION_COUNT,
};
use super::previous::comparison;
use super::{
    ReportCaptureLimits, ReportClock, ReportPeriod, ReportRecurring, ReportRequest, ReportSnapshot,
    SummaryExtras, SummarySnapshot,
};
use crate::recurring::{aggregate_rows, RecurringQuery, RowDecision};
use crate::{Result, Store};
use std::collections::HashSet;

impl Store {
    pub fn report_summary_snapshot(
        &mut self,
        request: &ReportRequest,
        clock: &ReportClock,
    ) -> Result<SummarySnapshot> {
        self.report_summary_snapshot_with_hook(
            request,
            clock,
            ReportCaptureLimits::default(),
            || {},
        )
    }

    #[doc(hidden)]
    pub fn report_summary_snapshot_with_hook<F>(
        &mut self,
        request: &ReportRequest,
        clock: &ReportClock,
        limits: ReportCaptureLimits,
        after_snapshot_established: F,
    ) -> Result<SummarySnapshot>
    where
        F: FnOnce(),
    {
        self.conn.execute_batch("BEGIN")?;
        let result = self.capture_summary(request, clock, limits, after_snapshot_established);
        match result {
            Ok(summary) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(summary)
            }
            Err(error) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    fn capture_summary<F>(
        &self,
        request: &ReportRequest,
        clock: &ReportClock,
        limits: ReportCaptureLimits,
        after_snapshot_established: F,
    ) -> Result<SummarySnapshot>
    where
        F: FnOnce(),
    {
        let snapshot = capture(
            &self.conn,
            request,
            clock,
            limits,
            after_snapshot_established,
        )?;
        let extras = SummaryExtras {
            days: month_days(request, &snapshot)?,
            month_rows: month_rows(&snapshot.months)?,
            account_totals: account_totals(&snapshot.preview.accounts, &snapshot.transactions)?,
            status_counts: status_counts(&snapshot.transactions)?,
            largest: largest(&snapshot.transactions, LARGEST_TRANSACTION_COUNT),
            comparison: comparison(&self.conn, request, &snapshot, limits)?,
            recurring: self.report_recurring(clock, &snapshot)?,
        };
        Ok(SummarySnapshot { snapshot, extras })
    }

    /// Prehľad's recurring overview for the report window, narrowed to the
    /// report accounts. Totals come from the same function over those rows.
    fn report_recurring(
        &self,
        clock: &ReportClock,
        snapshot: &ReportSnapshot,
    ) -> Result<ReportRecurring> {
        let range = snapshot.preview.range;
        let overview = self.recurring_overview(&RecurringQuery {
            from: range.map(|item| item.from),
            to: range.map(|item| item.to),
            account_kind: None,
            today: clock.today,
        })?;
        let ids = snapshot
            .preview
            .accounts
            .iter()
            .map(|account| account.id)
            .collect::<HashSet<_>>();
        let rows = overview
            .rows
            .into_iter()
            .filter(|row| ids.contains(&row.account_id))
            .collect::<Vec<_>>();
        let (confirmed, estimates, excluded) = aggregate_rows(&rows)?;
        Ok(ReportRecurring {
            as_of: overview.as_of,
            confirmed,
            estimates,
            excluded,
            rows: rows.into_iter().filter(is_shown).collect(),
        })
    }
}

fn is_shown(row: &crate::recurring::RecurringRow) -> bool {
    use crate::recurring::RecurringState::{Active, Upcoming};
    row.decision != RowDecision::Ignored && matches!(row.state, Active | Upcoming)
}

/// Daily entries exist only for a single-month period.
fn month_days(request: &ReportRequest, snapshot: &ReportSnapshot) -> Result<Vec<super::ReportDay>> {
    match (&request.period, snapshot.preview.range) {
        (ReportPeriod::Month { .. }, Some(range)) => days(&snapshot.transactions, range),
        _ => Ok(Vec::new()),
    }
}
