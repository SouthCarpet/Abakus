//! Calendar-month occurrence arithmetic (recurring-contract.md §4): every
//! occurrence is computed directly from the anchor date, never by
//! repeatedly adding a month to the previous (possibly clamped) occurrence,
//! so a clamped day never silently drifts forward cycle over cycle.
use super::types::Cadence;
use crate::{Result, StoreError};
use chrono::{Datelike, NaiveDate};

pub(crate) fn cadence_months(c: Cadence) -> i64 {
    match c {
        Cadence::Monthly => 1,
        Cadence::Quarterly => 3,
        Cadence::Yearly => 12,
    }
}

fn out_of_range() -> StoreError { StoreError::Parse("recurring: dátum mimo podporovaného rozsahu 0001-9999".into()) }

fn days_in_month(year: i32, month: u32) -> Result<u32> {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_of_next = NaiveDate::from_ymd_opt(ny, nm, 1).ok_or_else(out_of_range)?;
    Ok(first_of_next.pred_opt().ok_or_else(out_of_range)?.day())
}

fn is_month_end(d: NaiveDate) -> Result<bool> { Ok(d.day() == days_in_month(d.year(), d.month())?) }

pub(crate) fn month_end(d: NaiveDate) -> Result<NaiveDate> {
    NaiveDate::from_ymd_opt(d.year(), d.month(), days_in_month(d.year(), d.month())?).ok_or_else(out_of_range)
}

/// The Nth occurrence (`n = 0` is the anchor itself) of `cadence` starting
/// at `anchor`. A month-end anchor stays at month end every cycle
/// (including leap-year February 29); any other anchor day clamps to the
/// target month's final day without changing the original anchor for
/// later, unclamped months.
pub(crate) fn occurrence(anchor: NaiveDate, cadence: Cadence, n: i64) -> Result<NaiveDate> {
    let months = n.checked_mul(cadence_months(cadence)).ok_or_else(out_of_range)?;
    let total = (anchor.year() as i64) * 12 + (anchor.month() as i64 - 1) + months;
    let ny = total.div_euclid(12);
    let nm = (total.rem_euclid(12) + 1) as u32;
    let ny = i32::try_from(ny).map_err(|_| out_of_range())?;
    if !(1..=9999).contains(&ny) {
        return Err(out_of_range());
    }
    let last_day = days_in_month(ny, nm)?;
    let day = if is_month_end(anchor)? { last_day } else { anchor.day().min(last_day) };
    NaiveDate::from_ymd_opt(ny, nm, day).ok_or_else(out_of_range)
}

/// Whole calendar months between two dates' (year, month), ignoring day of
/// month: used for the day-gap table's "calendar month difference" column.
pub(crate) fn calendar_months_between(earlier: NaiveDate, later: NaiveDate) -> i64 {
    (later.year() as i64 * 12 + later.month() as i64) - (earlier.year() as i64 * 12 + earlier.month() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recurring::types::Cadence;

    #[test]
    fn monthly_anchor_31_lands_on_month_end_when_shorter() {
        let anchor = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(occurrence(anchor, Cadence::Monthly, 1).unwrap(), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        assert_eq!(occurrence(anchor, Cadence::Monthly, 2).unwrap(), NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(), "clamped Feb must not permanently change the anchor day");
    }

    #[test]
    fn a_manual_jan_30_anchor_gives_feb_28_then_mar_30_not_mar_28() {
        let anchor = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        assert_eq!(occurrence(anchor, Cadence::Monthly, 1).unwrap(), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        assert_eq!(occurrence(anchor, Cadence::Monthly, 2).unwrap(), NaiveDate::from_ymd_opt(2026, 3, 30).unwrap(), "the un-clamped anchor day 30 must return once March has 31 days");
    }

    #[test]
    fn yearly_feb_29_anchor_returns_feb_28_in_a_non_leap_year_and_feb_29_in_the_next_leap_year() {
        let anchor = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        assert_eq!(occurrence(anchor, Cadence::Yearly, 1).unwrap(), NaiveDate::from_ymd_opt(2025, 2, 28).unwrap());
        assert_eq!(occurrence(anchor, Cadence::Yearly, 2).unwrap(), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        assert_eq!(occurrence(anchor, Cadence::Yearly, 4).unwrap(), NaiveDate::from_ymd_opt(2028, 2, 29).unwrap(), "2028 is a leap year, so the month-end anchor recovers Feb 29");
    }

    #[test]
    fn quarterly_and_yearly_month_step() {
        let anchor = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(occurrence(anchor, Cadence::Quarterly, 1).unwrap(), NaiveDate::from_ymd_opt(2026, 4, 30).unwrap());
        assert_eq!(occurrence(anchor, Cadence::Yearly, 1).unwrap(), NaiveDate::from_ymd_opt(2027, 1, 31).unwrap());
    }

    #[test]
    fn calendar_month_difference_ignores_day_of_month() {
        let a = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        let b = NaiveDate::from_ymd_opt(2026, 2, 28).unwrap();
        assert_eq!(calendar_months_between(a, b), 1);
        let c = NaiveDate::from_ymd_opt(2026, 4, 30).unwrap();
        assert_eq!(calendar_months_between(a, c), 3);
    }
}
