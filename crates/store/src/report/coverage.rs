use super::{ReportCoverage, ReportDateRange};
use crate::{Result, StoreError};
use chrono::{Datelike, Duration, NaiveDate};

#[derive(Debug)]
pub(super) struct StatementRange {
    pub account_id: i64,
    pub from: String,
    pub to: String,
    pub checksum_status: String,
}

pub(super) fn coverage_for(
    account_id: i64,
    statements: &[StatementRange],
    finite: Option<ReportDateRange>,
) -> Result<ReportCoverage> {
    let own: Vec<&StatementRange> = statements
        .iter()
        .filter(|row| row.account_id == account_id && relevant(row, finite))
        .collect();
    let statement_count = u32::try_from(own.len()).map_err(|_| count_overflow())?;
    let unverified_statement_count = count_unverified(&own)?;
    let (mut ranges, invalid_range_count) = valid_ranges(&own, finite)?;
    ranges.sort_by_key(|range| (range.from, range.to));
    let merged = merge_ranges(ranges);
    let known_range = outer_range(&merged);
    let gaps = gaps_for(&merged, finite);
    Ok(ReportCoverage {
        account_id,
        known_range,
        gaps,
        statement_count,
        unverified_statement_count,
        invalid_range_count,
    })
}

fn relevant(row: &StatementRange, finite: Option<ReportDateRange>) -> bool {
    let Some(bound) = finite else { return true };
    let Some(from) = supported_date(&row.from) else {
        return true;
    };
    let Some(to) = supported_date(&row.to) else {
        return true;
    };
    from > to || (to >= bound.from && from <= bound.to)
}

fn count_unverified(rows: &[&StatementRange]) -> Result<u32> {
    u32::try_from(
        rows.iter()
            .filter(|row| row.checksum_status != "ok")
            .count(),
    )
    .map_err(|_| count_overflow())
}

fn valid_ranges(
    rows: &[&StatementRange],
    finite: Option<ReportDateRange>,
) -> Result<(Vec<ReportDateRange>, u32)> {
    let mut valid = Vec::new();
    let mut invalid = 0_u32;
    for row in rows {
        match parse_range(row, finite) {
            ParsedRange::Inside(range) => valid.push(range),
            ParsedRange::Outside => {}
            ParsedRange::Invalid => invalid = invalid.checked_add(1).ok_or_else(count_overflow)?,
        }
    }
    Ok((valid, invalid))
}

enum ParsedRange {
    Inside(ReportDateRange),
    Outside,
    Invalid,
}

fn parse_range(row: &StatementRange, finite: Option<ReportDateRange>) -> ParsedRange {
    let Some(from) = supported_date(&row.from) else {
        return ParsedRange::Invalid;
    };
    let Some(to) = supported_date(&row.to) else {
        return ParsedRange::Invalid;
    };
    if from > to {
        return ParsedRange::Invalid;
    }
    match finite {
        Some(bound) if to < bound.from || from > bound.to => ParsedRange::Outside,
        Some(bound) => ParsedRange::Inside(ReportDateRange {
            from: from.max(bound.from),
            to: to.min(bound.to),
        }),
        None => ParsedRange::Inside(ReportDateRange { from, to }),
    }
}

fn supported_date(value: &str) -> Option<NaiveDate> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    (1..=9999).contains(&date.year()).then_some(date)
}

fn merge_ranges(ranges: Vec<ReportDateRange>) -> Vec<ReportDateRange> {
    let mut merged: Vec<ReportDateRange> = Vec::new();
    for range in ranges {
        if let Some(last) = merged.last_mut() {
            if adjacent_or_overlapping(*last, range) {
                last.to = last.to.max(range.to);
                continue;
            }
        }
        merged.push(range);
    }
    merged
}

fn adjacent_or_overlapping(left: ReportDateRange, right: ReportDateRange) -> bool {
    right.from <= left.to
        || left
            .to
            .checked_add_signed(Duration::days(1))
            .is_some_and(|next| right.from <= next)
}

fn outer_range(ranges: &[ReportDateRange]) -> Option<ReportDateRange> {
    Some(ReportDateRange {
        from: ranges.first()?.from,
        to: ranges.last()?.to,
    })
}

fn gaps_for(merged: &[ReportDateRange], finite: Option<ReportDateRange>) -> Vec<ReportDateRange> {
    let Some(outer) = finite.or_else(|| outer_range(merged)) else {
        return Vec::new();
    };
    let mut gaps = Vec::new();
    let mut cursor = Some(outer.from);
    for range in merged {
        let Some(current) = cursor else { break };
        if current < range.from {
            gaps.push(ReportDateRange {
                from: current,
                to: range.from.pred_opt().unwrap_or(current),
            });
        }
        cursor = range.to.succ_opt().filter(|next| *next > current);
    }
    if let Some(cursor) = cursor.filter(|cursor| *cursor <= outer.to) {
        gaps.push(ReportDateRange {
            from: cursor,
            to: outer.to,
        });
    }
    gaps
}

fn count_overflow() -> StoreError {
    StoreError::Parse("Počet výpisov prekročil podporovaný rozsah.".into())
}
