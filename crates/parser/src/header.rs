use crate::fold::fold;
use crate::model::{AccountKind, Header, ParseError};
use crate::{iban, money};
use regex::Regex;

pub fn parse_header(page1: &[String]) -> Result<Header, ParseError> {
    let joined = page1.iter().take(12).cloned().collect::<Vec<_>>().join("\n");
    let folded = fold(&joined);
    // The trailing optional group only widens the last block within the same line;
    // `\s` here would also match the join newline and swallow the next line's
    // leading capital (e.g. "Dialog" -> "D"), so it is space/tab-only, unlike the
    // `\s` used between full 4-char groups earlier in the pattern.
    let iban_re = Regex::new(r"IBAN\s+([A-Z]{2}\d{2}(?:\s?[A-Z0-9]{4}){2,7}(?:[ \t]?[A-Z0-9]{1,4})?)").unwrap();
    let iban = iban_re.captures(&joined).map(|c| iban::normalize(&c[1])).ok_or_else(|| ParseError::NotAStatement("no IBAN line".into()))?;
    let account_kind = if folded.contains("podnikatelsky ucet") { AccountKind::Business } else if folded.contains("osobny ucet") { AccountKind::Personal } else { return Err(ParseError::NotAStatement("no account kind".into())) };
    let number = Regex::new(r"vypis cislo:\s*(\d+)").unwrap().captures(&folded).and_then(|c| c[1].parse().ok()).ok_or_else(|| ParseError::NotAStatement("no statement number".into()))?;
    let date = Regex::new(r"datum\s+(\d{2}\.\d{2}\.\d{4})").unwrap().captures(&folded).and_then(|c| money::parse_date_long(&c[1])).ok_or_else(|| ParseError::NotAStatement("no statement date".into()))?;
    Ok(Header { iban, account_kind, number, date })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AccountKind;
    fn p(lines: &[&str]) -> Vec<String> { lines.iter().map(|s| s.to_string()).collect() }
    const PAGE: &[&str] = &[
        "Osobný účet     SK44 1100 0000 0000 1234 5678          Mena  EUR       BIC (SWIFT)   TATRSKBX",
        "IBAN SK44 1100 0000 0000 1234 5678",
        "Dialog  0800 00 1100              ID:   00               Výpis číslo:        6",
        "Osobný účet     SK44 1100 0000 0000 1234 5678     Majiteľ Jana Vzorová      Dátum 30.06.2026",
        "Dátum sprac.  Popis          Dátum zúčt.       Suma",
    ];
    #[test] fn reads_iban_kind_number_and_date() {
        let h = parse_header(&p(PAGE)).unwrap();
        assert_eq!(h.iban, "SK4411000000000012345678"); assert_eq!(h.account_kind, AccountKind::Personal); assert_eq!(h.number, 6);
        assert_eq!(h.date, chrono::NaiveDate::from_ymd_opt(2026, 6, 30).unwrap());
    }
    #[test] fn business_header_is_business() {
        let mut page = p(PAGE); page[0] = page[0].replace("Osobný účet", "Podnikateľský účet"); page[3] = page[3].replace("Osobný účet", "Podnikateľský účet");
        assert_eq!(parse_header(&page).unwrap().account_kind, AccountKind::Business);
    }
    #[test] fn missing_iban_is_not_a_statement() {
        assert!(matches!(parse_header(&p(&["hello", "world"])), Err(ParseError::NotAStatement(_))));
    }

    /// The 2026-06-30 statement's own header shape, run through `group_lines` first (the
    /// diagnostic geometry, `lines::fixtures`): "Osobný účet ... Dátum 30.06.2026" plus
    /// "IBAN SK97 ...". The statement number line is not part of that diagnostic, so it is added
    /// as plain text here; only the IBAN, account kind and date come from the grouped geometry.
    #[test] fn new_generator_line_pair_after_grouping_parses_iban_kind_and_date() {
        use crate::lines::{fixtures::{header_line1, header_line2}, group_lines};
        let mut chars = header_line1();
        chars.extend(header_line2());
        let grouped = group_lines(&chars);
        let mut page: Vec<String> = grouped.into_iter().map(|l| l.text).collect();
        page.push("Výpis číslo: 6".to_string());
        let h = parse_header(&page).unwrap();
        assert_eq!(h.iban, "SK9711000000002935301887");
        assert_eq!(h.account_kind, AccountKind::Personal);
        assert_eq!(h.date, chrono::NaiveDate::from_ymd_opt(2026, 6, 30).unwrap());
    }
}
