use crate::blocks::is_column_header;
use crate::fold::fold;
use crate::model::{AccountKind, Header, ParseError};
use crate::{iban, money};
use regex::Regex;

/// How many of page 1's own lines carry header fields. Client and bank address blocks (Michal's
/// real statement, 2026-09-09) push the statement-number line to index 21, past the old fixed
/// 12; the column header (`blocks::is_column_header`) marks the end of the header, so scope
/// stops there. Never fewer than the original 12, in case the column header sits earlier than
/// that on some layout. No column header on page 1 at all falls back to the first 40 lines.
fn header_scope(page1: &[String]) -> usize {
    match page1.iter().position(|l| is_column_header(l)) {
        Some(idx) => idx.max(12),
        None => 40,
    }
}

pub fn parse_header(page1: &[String]) -> Result<Header, ParseError> {
    let joined = page1.iter().take(header_scope(page1)).cloned().collect::<Vec<_>>().join("\n");
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

    /// Michal's real page 1 (masked `abakus-cli lines` dump, 2026-09-09 continuation): the
    /// client and bank address blocks push "Výpis číslo" to line 22 and the column header to
    /// line 25, past the old `take(12)`. Synthetic values only.
    fn real_shape_page1() -> Vec<String> {
        use crate::lines::{fixtures::{header_line1, header_line2}, group_lines};
        let mut chars = header_line1();
        chars.extend(header_line2());
        let mut page: Vec<String> = group_lines(&chars).into_iter().map(|l| l.text).collect(); // 1, 2
        page.push("--------------------------------------------------------------------------------------------------".into()); // 3
        page.push("Číslo klienta:  GKN1234567".into()); // 4
        page.push("Majiteľ účtu:   Fuchs Michal".into()); // 5
        page.push("Ulica 1".into()); // 6
        page.push("Mesto".into()); // 7
        page.push("Slovenská republika".into()); // 8
        page.push("Názov účtu:   Osobný účet".into()); // 9
        page.push("E-mail:   fuchs@example.test".into()); // 10
        page.push("123456".into()); // 11
        page.push("Fuchs Michal".into()); // 12
        page.push("Ulica 1".into()); // 13
        page.push("Mesto".into()); // 14
        page.push("Slovenská republika".into()); // 15
        page.push("Tatra banka, a.s.".into()); // 16
        page.push("Hodžovo nám. 3".into()); // 17
        page.push("811 06 Bratislava".into()); // 18
        page.push("IČO:   00686930".into()); // 19
        page.push("DIČ:   2020408522".into()); // 20
        page.push("Obchodný register Okresného súdu Bratislava I".into()); // 21
        page.push("Dialog Nitra 8        ID:  00        Výpis číslo:      6".into()); // 22
        page.push("Osobný účet  2935301887  Majiteľ Fuchs Michal   Dátum 30.06.2026".into()); // 23
        page.push("--------------------------------------------------------------------------------------------------".into()); // 24
        page.push("Dátum sprac. Popis   Dátum zúčt.   Suma".into()); // 25
        page
    }

    #[test] fn header_scope_reaches_the_statement_number_past_the_client_and_bank_blocks() {
        let h = parse_header(&real_shape_page1()).unwrap();
        assert_eq!(h.iban, "SK9711000000002935301887");
        assert_eq!(h.account_kind, AccountKind::Personal);
        assert_eq!(h.number, 6);
        assert_eq!(h.date, chrono::NaiveDate::from_ymd_opt(2026, 6, 30).unwrap());
    }

    #[test] fn header_scope_falls_back_to_40_lines_when_page1_has_no_column_header() {
        let mut page = p(&[PAGE[0], PAGE[1]]);
        for i in 0..15 { page.push(format!("Výplň riadok {i}")); } // lines 3..17
        page.push("Dialog  Výpis číslo:  6".into()); // line 18, past the old take(12)
        page.push("Dátum 30.06.2026".into());
        let h = parse_header(&page).unwrap();
        assert_eq!(h.number, 6);
    }
}
