use super::{ReportDateRange, ReportPeriod};
use crate::{Result, StoreError};
use chrono::{Datelike, Months, NaiveDate};

pub(super) fn resolve_period(
    period: &ReportPeriod,
    today: NaiveDate,
) -> Result<(Option<ReportDateRange>, bool)> {
    let range = match period {
        ReportPeriod::Month { month } => month_range(parse_month(month, "mesiac")?)?,
        ReportPeriod::SixMonths { ending_month } => {
            six_month_range(parse_month(ending_month, "koncový mesiac")?)?
        }
        ReportPeriod::Year { year } => year_range(*year)?,
        ReportPeriod::AllTime => return Ok((None, false)),
    };
    if range.from > today {
        return report_error("Obdobie sa začína v budúcnosti.");
    }
    Ok((Some(range), range.to >= today))
}

fn parse_month(value: &str, field: &str) -> Result<NaiveDate> {
    let bytes = value.as_bytes();
    let exact = bytes.len() == 7
        && bytes[4] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || b.is_ascii_digit());
    if !exact {
        return report_error(&format!("Pole {field} musí mať tvar RRRR-MM."));
    }
    let date = format!("{value}-01");
    let parsed = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| StoreError::Parse(format!("Pole {field} obsahuje neplatný mesiac.")))?;
    if !(1..=9999).contains(&parsed.year()) {
        return report_error(&format!("Pole {field} musí používať rok 0001 až 9999."));
    }
    Ok(parsed)
}

fn month_range(first: NaiveDate) -> Result<ReportDateRange> {
    if first.year() == 9999 && first.month() == 12 {
        let to = NaiveDate::from_ymd_opt(9999, 12, 31)
            .ok_or_else(|| StoreError::Parse("Mesiac nemá platný posledný deň.".into()))?;
        return Ok(ReportDateRange { from: first, to });
    }
    let next = first.checked_add_months(Months::new(1)).ok_or_else(|| {
        StoreError::Parse("Mesiac je mimo podporovaného rozsahu rokov 0001 až 9999.".into())
    })?;
    let to = next
        .pred_opt()
        .ok_or_else(|| StoreError::Parse("Mesiac nemá platný posledný deň.".into()))?;
    Ok(ReportDateRange { from: first, to })
}

fn six_month_range(ending: NaiveDate) -> Result<ReportDateRange> {
    let from = ending
        .checked_sub_months(Months::new(5))
        .ok_or_else(|| StoreError::Parse("Šesťmesačné obdobie prekračuje rok 0001.".into()))?;
    if from.year() < 1 {
        return report_error("Šesťmesačné obdobie prekračuje rok 0001.");
    }
    let to = month_range(ending)?.to;
    Ok(ReportDateRange { from, to })
}

fn year_range(year: i32) -> Result<ReportDateRange> {
    if !(1..=9999).contains(&year) {
        return report_error("Rok musí byť od 0001 do 9999.");
    }
    let from = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| StoreError::Parse("Rok nie je platný.".into()))?;
    let to = NaiveDate::from_ymd_opt(year, 12, 31)
        .ok_or_else(|| StoreError::Parse("Rok nie je platný.".into()))?;
    Ok(ReportDateRange { from, to })
}

pub(super) fn month_keys(range: ReportDateRange) -> Result<Vec<String>> {
    let mut current = range
        .from
        .with_day(1)
        .ok_or_else(|| StoreError::Parse("Začiatok obdobia nie je platný.".into()))?;
    let last = range
        .to
        .with_day(1)
        .ok_or_else(|| StoreError::Parse("Koniec obdobia nie je platný.".into()))?;
    let mut keys = Vec::new();
    loop {
        keys.push(current.format("%Y-%m").to_string());
        if current == last {
            return Ok(keys);
        }
        current = current.checked_add_months(Months::new(1)).ok_or_else(|| {
            StoreError::Parse("Počet mesiacov prekračuje podporovaný rozsah.".into())
        })?;
    }
}

fn report_error<T>(message: &str) -> Result<T> {
    Err(StoreError::Parse(message.into()))
}
