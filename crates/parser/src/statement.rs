use crate::blocks::{body_lines, split_blocks, Block};
use crate::card::{is_card, parse_card};
use crate::fields::{first_line, is_record_start};
use crate::fold::fold;
use crate::header::parse_header;
use crate::lines::from_text;
use crate::model::{ParseError, Statement, Transaction, TxKind};
use crate::money::{parse_amount, parse_date_long};
use crate::transfer::{is_transfer, parse_transfer};
use crate::Cents;
use chrono::NaiveDate;
use regex::Regex;

/// First entry is the confirmed Tatra banka wording (Michal, 2026-08-30); the rest are fallbacks for other variants.
pub const CLOSING_LABELS: &[&str] = &["zostatok na ucte ku dnu vystavenia vypisu", "konecny zostatok", "novy zostatok", "konecny stav"];

pub fn parse_text(text: &str) -> Result<Statement, ParseError> { parse_pages(&from_text(text)) }

pub fn parse_pages(pages: &[Vec<String>]) -> Result<Statement, ParseError> {
    let header = parse_header(pages.first().ok_or_else(|| ParseError::NotAStatement("empty".into()))?)?;
    let records = split_records(split_blocks(&body_lines(pages)));
    let mut st = Statement { iban: header.iban, account_kind: header.account_kind, number: header.number, period_start: header.date, period_end: header.date, opening_cents: None, closing_cents: None, transactions: Vec::new(), warnings: Vec::new() };
    for block in &records { absorb(&mut st, block); }
    Ok(st)
}

/// A block may hold several records (the opening line shares its block with the first record). Split at every record start after the first line.
fn split_records(blocks: Vec<Block>) -> Vec<Block> {
    let mut out = Vec::new();
    for b in blocks {
        let mut cur: Vec<String> = Vec::new();
        for l in b.lines {
            if is_record_start(&l) && !cur.is_empty() { out.push(Block { lines: std::mem::take(&mut cur) }); }
            cur.push(l);
        }
        if !cur.is_empty() { out.push(Block { lines: cur }); }
    }
    out
}

fn balance_line(line: &str) -> Option<(NaiveDate, Cents)> {
    let c = Regex::new(r"(\d{2}\.\d{2}\.\d{4})\s+([\d,]+\.\d{2}-?)\s*$").unwrap().captures(line)?;
    Some((parse_date_long(&c[1])?, parse_amount(&c[2])?))
}

/// Closing line: label, optional date, amount at the end (`Zostatok na účte ku dňu vystavenia výpisu:   424.24`).
fn trailing_amount(line: &str) -> Option<Cents> {
    let c = Regex::new(r"([\d,]+\.\d{2}-?)\s*$").unwrap().captures(line)?;
    parse_amount(&c[1])
}

fn absorb(st: &mut Statement, block: &Block) {
    let first = &block.lines[0];
    let f = fold(first);
    if f.starts_with("posledny vypis") {
        if let Some((date, cents)) = balance_line(first) { st.period_start = date; st.opening_cents = Some(cents); return; }
    }
    let closing_idx = block.lines.iter().position(|l| { let lf = fold(l); CLOSING_LABELS.iter().any(|c| lf.starts_with(c)) });
    if let Some(idx) = closing_idx {
        if first_line(block).is_none() {
            let line = &block.lines[idx];
            st.closing_cents = trailing_amount(line).or_else(|| block.lines.get(idx + 1).and_then(|l| trailing_amount(l)));
            if st.closing_cents.is_none() { st.warnings.push(format!("closing line not parsed: {}", line.trim())); }
            return;
        }
    }
    match first_line(block) {
        Some(fl) => {
            let t = dispatch(&fl, block);
            if t.kind == TxKind::Other { st.warnings.push(format!("unknown kind: {}", fl.description)); }
            st.transactions.push(t);
        }
        None => st.warnings.push(format!("unparsed block: {}", first.trim())),
    }
}

fn dispatch(fl: &crate::fields::FirstLine, block: &Block) -> Transaction {
    if is_card(&fl.description) { parse_card(fl, block) }
    else if is_transfer(&fl.description) { parse_transfer(fl, block) }
    else { let mut t = Transaction::blank(fl.posted, fl.amount, TxKind::Other, block.lines.join("\n")); t.merchant_raw = fl.description.clone(); t }
}
