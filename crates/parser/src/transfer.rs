use crate::blocks::Block;
use crate::fields::{field, line_after, FirstLine};
use crate::fold::fold;
use crate::iban;
use crate::model::{Transaction, TxKind};
use crate::money::parse_date_long;
use regex::Regex;

pub fn is_transfer(description: &str) -> bool { let f = fold(description); f.starts_with("platba ") || f.starts_with("tpp ") }

fn kind_of(first: &FirstLine) -> TxKind {
    if fold(&first.description).starts_with("tpp ") { TxKind::StandingOrder } else if first.amount >= 0 { TxKind::TransferIn } else { TxKind::TransferOut }
}

fn explicit_iban(block: &Block) -> Option<String> {
    let re = Regex::new(r"[A-Z]{2}\d{2}(?: ?[A-Z0-9]{4}){2,7}(?: ?[A-Z0-9]{1,4})?").unwrap();
    let found = block.lines.iter().skip(1).flat_map(|l| re.find_iter(l)).map(|m| iban::normalize(m.as_str())).find(|c| iban::is_valid(c));
    found
}

fn derived_iban(description: &str) -> Option<String> {
    let reference = description.split_whitespace().nth(1)?;
    iban::parse_account_ref(reference).map(|(b, p, n)| iban::from_sk_parts(&b, &p, &n))
}

fn looks_like_iban(s: &str) -> bool { iban::is_valid(s) }

fn counterparty_name(block: &Block) -> Option<String> {
    let label = if field(block, "platitel").is_some() || line_after(block, "platitel").is_some() { "platitel" } else { "prijemca" };
    let same_line = field(block, label).filter(|v| !looks_like_iban(v));
    same_line.or_else(|| {
        let mut idx = block.lines.iter().position(|l| fold(l).contains(label))? + 1;
        while let Some(l) = block.lines.get(idx) { let t = l.trim(); if t.is_empty() || looks_like_iban(t) { idx += 1; continue; } return (!t.contains(':')).then(|| t.to_string()); }
        None
    })
}

pub fn parse_transfer(first: &FirstLine, block: &Block) -> Transaction {
    let mut t = Transaction::blank(first.posted, first.amount, kind_of(first), block.lines.join("\n"));
    t.counterparty_iban = explicit_iban(block).or_else(|| derived_iban(&first.description));
    t.counterparty_name = counterparty_name(block);
    t.reference = field(block, "prijata platba").or_else(|| field(block, "odoslana platba")).or_else(|| field(block, "referencia platitela")).or_else(|| field(block, "detail"));
    t.tx_date = field(block, "valuta").and_then(|v| parse_date_long(&v)).unwrap_or(first.posted);
    t.merchant_raw = t.counterparty_name.clone().unwrap_or_else(|| first.description.clone());
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::Block;
    use crate::fields::first_line;
    use crate::model::TxKind;
    fn parse(lines: &[&str]) -> crate::model::Transaction {
        let blk = Block { lines: lines.iter().map(|s| s.to_string()).collect() };
        parse_transfer(&first_line(&blk).unwrap(), &blk)
    }
    #[test] fn incoming_with_explicit_iban_and_name_lines() {
        let t = parse(&["01.06.2026    Platba 0200/000000-5230000001            10.00", "   Prijatá platba:                         P026060148122371", "   Suma:10.00EUR      Kurz:1.00000000     Valuta:01.06.2026", "   Platiteľ:", "   SK09 0200 0000 0052 3000 0001", "   Peter Vzorový"]);
        assert_eq!(t.kind, TxKind::TransferIn); assert_eq!(t.counterparty_iban.as_deref(), Some("SK0902000000005230000001"));
        assert_eq!(t.counterparty_name.as_deref(), Some("Peter Vzorový")); assert_eq!(t.reference.as_deref(), Some("P026060148122371")); assert_eq!(t.merchant_raw, "Peter Vzorový");
    }
    #[test] fn account_ref_without_iban_line_is_derived() {
        let t = parse(&["05.06.2026    TPP 1100/000000-0098765432            80.00-", "   Platba trvalým príkazom:", "   Suma:80.00EUR   Kurz:1.00000000   Valuta:05.06.2026", "   Príjemca:", "   Jana Vzorová"]);
        assert_eq!(t.kind, TxKind::StandingOrder); assert_eq!(t.counterparty_iban.as_deref(), Some("SK3711000000000098765432")); assert_eq!(t.tx_date, chrono::NaiveDate::from_ymd_opt(2026, 6, 5).unwrap());
    }
    #[test] fn foreign_reference_has_no_iban_and_uses_detail_when_reference_missing() {
        let t = parse(&["02.06.2026    Platba 0026060200022221            40.00-", "   Suma:40.00EUR   Kurz:1.00000000   Valuta:02.06.2026", "   Príjemca:", "   Landesdirektion Sachsen", "   Detail: 2120415617"]);
        assert_eq!(t.kind, TxKind::TransferOut); assert_eq!(t.counterparty_iban, None); assert_eq!(t.counterparty_name.as_deref(), Some("Landesdirektion Sachsen")); assert_eq!(t.reference.as_deref(), Some("2120415617"));
    }
    #[test] fn iban_on_the_label_line_and_name_below() {
        let t = parse(&["11.06.2026    Platba 1100/000000-0012345678         1,300.00-", "   Príjemca:         SK4411000000000012345678", "   Jana Vzorová"]);
        assert_eq!(t.counterparty_iban.as_deref(), Some("SK4411000000000012345678")); assert_eq!(t.counterparty_name.as_deref(), Some("Jana Vzorová"));
    }
    #[test] fn iban_only_gives_no_name() {
        let t = parse(&["03.06.2026    TPP 8180/000000-7000000001         233.42-", "   Príjemca:         SK0281800000007000000001"]);
        assert_eq!(t.counterparty_name, None); assert_eq!(t.merchant_raw, "TPP 8180/000000-7000000001");
    }
}
