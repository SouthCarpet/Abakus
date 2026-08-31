use crate::blocks::Block;
use crate::fold::fold;
use crate::money::{parse_amount, parse_date_long};
use crate::Cents;
use chrono::NaiveDate;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq)]
pub struct FirstLine { pub posted: NaiveDate, pub description: String, pub right_date: Option<NaiveDate>, pub amount: Cents }

fn first_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(\d{2}\.\d{2}\.\d{4})\s+(.+?)\s{2,}(?:(\d{2}\.\d{2}\.\d{4})\s+)?([\d,]+\.\d{2}-?)\s*$").unwrap())
}

pub fn is_record_start(line: &str) -> bool { first_re().is_match(line) }

pub fn first_line(block: &Block) -> Option<FirstLine> {
    let c = first_re().captures(block.lines.first()?)?;
    Some(FirstLine { posted: parse_date_long(&c[1])?, description: c[2].trim().to_string(), right_date: c.get(3).and_then(|d| parse_date_long(d.as_str())), amount: parse_amount(&c[4])? })
}

/// Split a line into segments on runs of two or more spaces (the statement's column gaps).
fn segments(line: &str) -> Vec<String> {
    Regex::new(r"\s{2,}").unwrap().split(line.trim()).filter(|p| !p.is_empty()).map(str::to_string).collect()
}

/// True when a segment starts a `Label:` pair (letters before the colon).
/// `12:13:57` is a time value, not a label.
fn is_labelled(seg: &str) -> bool {
    seg.split_once(':').is_some_and(|(h, _)| !h.is_empty() && h.chars().any(char::is_alphabetic))
}

/// Values of the segment whose folded text before ':' EQUALS `label` (or is
/// the label plus a `/variant`, e.g. `miesto platby/vyberu`). Substring
/// matches are rejected on purpose (review blocker 2026-08-30): `platitel`
/// must not match `Referencia banky platiteľa`, `prijemca` must not match
/// `Banka príjemcu`.
fn labelled_values(block: &Block, label: &str) -> Option<(usize, Vec<String>)> {
    for (i, line) in block.lines.iter().enumerate() {
        let segs = segments(line);
        for (j, seg) in segs.iter().enumerate() {
            let Some((head, tail)) = seg.split_once(':') else { continue };
            let fh = fold(head);
            if fh != label && !fh.starts_with(&format!("{label}/")) { continue; }
            let mut vals: Vec<String> = Vec::new();
            if !tail.trim().is_empty() { vals.push(tail.trim().to_string()); }
            for later in &segs[j + 1..] {
                if is_labelled(later) { break; }
                vals.push(later.clone());
            }
            return Some((i, vals));
        }
    }
    None
}

pub fn pieces(block: &Block, label: &str) -> Vec<String> { labelled_values(block, label).map(|(_, v)| v).unwrap_or_default() }
pub fn field(block: &Block, label: &str) -> Option<String> { pieces(block, label).into_iter().next() }
pub fn line_after(block: &Block, label: &str) -> Option<String> {
    let (i, _) = labelled_values(block, label)?;
    block.lines.get(i + 1).map(|l| l.trim().to_string()).filter(|l| !l.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::Block;
    fn b(lines: &[&str]) -> Block { Block { lines: lines.iter().map(|s| s.to_string()).collect() } }
    #[test] fn first_line_with_amount_only() {
        let f = first_line(&b(&["01.06.2026    EUR AP nákup POS                       1.99-"])).unwrap();
        assert_eq!(f.description, "EUR AP nákup POS"); assert_eq!(f.amount, -199); assert_eq!(f.right_date, None);
    }
    #[test] fn first_line_with_right_date() {
        let f = first_line(&b(&["02.06.2026    EUR NÁVRAT POS          29.05.2026        43.00"])).unwrap();
        assert_eq!(f.right_date, chrono::NaiveDate::from_ymd_opt(2026, 5, 29)); assert_eq!(f.amount, 4300);
    }
    #[test] fn non_record_line_is_none() { assert!(first_line(&b(&["   Posledný výpis  30.05.2026      693.92"])).is_none()); }
    #[test] fn field_takes_first_piece_after_colon() {
        let blk = b(&["x", "   Číslo karty:      440000******1111      Držiteľ: JANA VZOROVÁ"]);
        assert_eq!(field(&blk, "cislo karty").as_deref(), Some("440000******1111")); assert_eq!(field(&blk, "drzitel").as_deref(), Some("JANA VZOROVÁ"));
    }
    #[test] fn field_without_spaces_after_colon() {
        let blk = b(&["x", "   Suma:10.00EUR      Kurz:1.00000000     Valuta:01.06.2026"]);
        assert_eq!(field(&blk, "valuta").as_deref(), Some("01.06.2026")); assert_eq!(field(&blk, "kurz").as_deref(), Some("1.00000000"));
    }
    #[test] fn pieces_returns_all_columns() {
        assert_eq!(pieces(&b(&["   Miesto platby:    Neuss                 ALDI SUED"]), "miesto platby"), vec!["Neuss".to_string(), "ALDI SUED".into()]);
    }
    #[test] fn line_after_label() { assert_eq!(line_after(&b(&["   Platiteľ:", "   Peter Vzorový"]), "platitel").as_deref(), Some("Peter Vzorový")); }
    #[test] fn platitel_does_not_match_referencia_banky_platitela() {
        let blk = b(&["x", "   Referencia banky platiteľa:             2699990000000001", "   Platiteľ:", "   Peter Vzorový"]);
        assert_eq!(field(&blk, "platitel"), None); assert_eq!(line_after(&blk, "platitel").as_deref(), Some("Peter Vzorový"));
        assert_eq!(field(&blk, "referencia banky platitela").as_deref(), Some("2699990000000001"));
    }
    #[test] fn prijemca_does_not_match_banka_prijemcu() {
        let blk = b(&["x", "   Banka príjemcu:   MARKDEF1860", "   Príjemca:", "   Landesdirektion Sachsen"]);
        assert_eq!(field(&blk, "prijemca"), None); assert_eq!(line_after(&blk, "prijemca").as_deref(), Some("Landesdirektion Sachsen"));
    }
    #[test] fn time_values_with_colons_are_not_labels() {
        let blk = b(&["x", "   Dátum:  29.05.26  Čas:  12:13:57        Suma:          1.99- EUR"]);
        assert_eq!(field(&blk, "cas").as_deref(), Some("12:13:57")); assert_eq!(field(&blk, "datum").as_deref(), Some("29.05.26"));
    }
    #[test] fn slash_variant_label_matches_prefix() {
        assert_eq!(pieces(&b(&["   Miesto platby/výberu: London            OF"]), "miesto platby"), vec!["London".to_string(), "OF".into()]);
    }
}
