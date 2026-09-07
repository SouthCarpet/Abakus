//! Statement coverage (recurring-contract.md §5): only ranges from
//! statements with `checksum_status='ok'` can ever prove the ABSENCE of a
//! transaction. Off-by/unverifiable statements remain visible evidence
//! elsewhere but never close a coverage gap here.
use crate::{Result, Store};
use chrono::{Duration, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Range {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// Overlapping, nested, duplicate and adjacent (touching, no calendar-day
/// gap) ranges collapse into one, so a real one-day gap between two
/// statements is the only thing that can still break coverage.
pub(crate) fn merge_ranges(mut ranges: Vec<Range>) -> Vec<Range> {
    ranges.retain(|r| r.start <= r.end);
    ranges.sort_by_key(|r| r.start);
    let mut merged: Vec<Range> = Vec::new();
    for r in ranges {
        match merged.last_mut() {
            Some(last) if r.start <= last.end + Duration::days(1) => {
                if r.end > last.end {
                    last.end = r.end;
                }
            }
            _ => merged.push(r),
        }
    }
    merged
}

pub(crate) fn is_covered(ranges: &[Range], start: NaiveDate, end: NaiveDate) -> bool {
    if start > end {
        return true; // an empty window is trivially covered
    }
    ranges.iter().any(|r| r.start <= start && r.end >= end)
}

impl Store {
    /// Trusted (`checksum_status='ok'`) coverage ranges for one account,
    /// merged. Off-by/unverifiable statements are excluded entirely: they
    /// are real evidence of an IMPORT, never proof of an absence.
    pub(crate) fn trusted_coverage(&self, account_id: i64) -> Result<Vec<Range>> {
        let mut st = self.conn.prepare("SELECT period_start, period_end FROM statements WHERE account_id = ?1 AND checksum_status = 'ok'")?;
        let rows = st.query_map([account_id], |r| {
            let start: String = r.get(0)?;
            let end: String = r.get(1)?;
            Ok((start, end))
        })?;
        let mut ranges = Vec::new();
        for row in rows {
            let (start, end) = row?;
            let (Ok(start), Ok(end)) = (NaiveDate::parse_from_str(&start, "%Y-%m-%d"), NaiveDate::parse_from_str(&end, "%Y-%m-%d")) else { continue };
            ranges.push(Range { start, end });
        }
        Ok(merge_ranges(ranges))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate { NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap() }
    fn r(a: &str, b: &str) -> Range { Range { start: d(a), end: d(b) } }

    #[test]
    fn overlapping_nested_and_duplicate_ranges_merge_into_one() {
        let merged = merge_ranges(vec![r("2026-01-01", "2026-01-31"), r("2026-01-15", "2026-01-20"), r("2026-01-01", "2026-01-31")]);
        assert_eq!(merged, vec![r("2026-01-01", "2026-01-31")]);
    }

    #[test]
    fn adjacent_ranges_merge_with_no_gap() {
        let merged = merge_ranges(vec![r("2026-01-01", "2026-01-31"), r("2026-02-01", "2026-02-28")]);
        assert_eq!(merged, vec![r("2026-01-01", "2026-02-28")]);
    }

    #[test]
    fn a_real_one_day_gap_stays_a_gap() {
        let merged = merge_ranges(vec![r("2026-01-01", "2026-01-30"), r("2026-02-01", "2026-02-28")]);
        assert_eq!(merged, vec![r("2026-01-01", "2026-01-30"), r("2026-02-01", "2026-02-28")]);
        assert!(!is_covered(&merged, d("2026-01-25"), d("2026-02-05")));
    }

    #[test]
    fn a_reversed_or_invalid_range_never_grants_coverage() {
        let merged = merge_ranges(vec![Range { start: d("2026-02-01"), end: d("2026-01-01") }]);
        assert!(merged.is_empty());
    }

    #[test]
    fn no_statements_means_no_coverage_anywhere() {
        assert!(!is_covered(&[], d("2026-01-01"), d("2026-01-31")));
    }
}
