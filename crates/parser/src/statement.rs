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
    let records = split_summary(merge_continuations(split_records(split_blocks(&body_lines(pages)))));
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

/// True for the label lines a closing-balance/account-summary block ends with. Used both to
/// keep such a block out of `merge_continuations` and to find where `split_summary` cuts one
/// off a transaction it got glued to (Michal's real page 9, 2026-09-09 continuation: the
/// `SPOLU`/`CR:` subtotal, the overdraft line and the savings-goal line follow the last
/// transaction's detail lines with no separator in between).
fn is_summary_line(l: &str) -> bool {
    let f = fold(l);
    f.starts_with("spolu") || f.starts_with("cr:") || f.starts_with("dt:") || CLOSING_LABELS.iter().any(|c| f.starts_with(c)) || f.starts_with("nepovolene precerpanie") || f.starts_with("sporenie k uctu")
}

/// A block whose first line is not a record start, not the opening line, and not a summary line
/// is the tail of the previous record's detail lines, cut off by a page-break separator
/// (Michal's real page 2) or the shorter dashes inside a fee breakdown (real page 9); it is
/// appended back onto the previous record. A block with no previous record to join keeps
/// today's "Blok sa nepodarilo spracovať" warning.
fn merge_continuations(blocks: Vec<Block>) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for b in blocks {
        let first = &b.lines[0];
        let is_continuation = !is_record_start(first) && !fold(first).starts_with("posledny vypis") && !is_summary_line(first);
        match (is_continuation, out.last_mut()) {
            (true, Some(prev)) => prev.lines.extend(b.lines),
            _ => out.push(b),
        }
    }
    out
}

/// Splits a block at its first summary line (`is_summary_line`) so a closing-balance/summary
/// tail glued onto the last transaction becomes its own block, read by `try_closing` exactly as
/// an already-separated closing block is. A block whose own first line is already a summary
/// line (the usual, well-separated case) is left alone.
fn split_summary(blocks: Vec<Block>) -> Vec<Block> {
    let mut out = Vec::new();
    for b in blocks {
        match b.lines.iter().position(|l| is_summary_line(l)).filter(|i| *i > 0) {
            Some(idx) => {
                let (head, tail) = b.lines.split_at(idx);
                out.push(Block { lines: head.to_vec() });
                out.push(Block { lines: tail.to_vec() });
            }
            None => out.push(b),
        }
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

/// A fee record (`Poplatok za účet`, `Poplatky za transakcie`): kind stays `Other` (a dedicated
/// `Fee` kind is deferred, see `KNOWN_ISSUES.md`), but it is a recognized shape, so it does not
/// warn as an unknown transaction type.
fn is_fee(description: &str) -> bool { fold(description).starts_with("poplat") }

fn absorb(st: &mut Statement, block: &Block) {
    if try_opening(st, block) { return; }
    if try_closing(st, block) { return; }
    match first_line(block) {
        Some(fl) => push_transaction(st, &fl, block),
        None => st.warnings.push(format!("Blok sa nepodarilo spracovať: {}", block.lines[0].trim())),
    }
}

fn try_opening(st: &mut Statement, block: &Block) -> bool {
    let first = &block.lines[0];
    if !fold(first).starts_with("posledny vypis") { return false; }
    let Some((date, cents)) = balance_line(first) else { return false };
    st.period_start = date;
    st.opening_cents = Some(cents);
    true
}

fn try_closing(st: &mut Statement, block: &Block) -> bool {
    let Some(idx) = block.lines.iter().position(|l| { let lf = fold(l); CLOSING_LABELS.iter().any(|c| lf.starts_with(c)) }) else { return false };
    if first_line(block).is_some() { return false; }
    let line = &block.lines[idx];
    st.closing_cents = trailing_amount(line).or_else(|| block.lines.get(idx + 1).and_then(|l| trailing_amount(l)));
    // A17/F8: the warning text is Slovak because this parser has one consumer
    // (this app's UI); the interpolated line/description text only the parser
    // holds at this point, so it stays here rather than a code the UI re-translates.
    if st.closing_cents.is_none() { st.warnings.push(format!("Konečný zostatok sa nedá prečítať: {}", line.trim())); }
    true
}

fn push_transaction(st: &mut Statement, fl: &crate::fields::FirstLine, block: &Block) {
    let t = dispatch(fl, block);
    if t.kind == TxKind::Other && !is_fee(&fl.description) { st.warnings.push(format!("Neznámy typ transakcie: {}", fl.description)); }
    st.transactions.push(t);
}

fn dispatch(fl: &crate::fields::FirstLine, block: &Block) -> Transaction {
    if is_card(&fl.description) { parse_card(fl, block) }
    else if is_transfer(&fl.description) { parse_transfer(fl, block) }
    else { let mut t = Transaction::blank(fl.posted, fl.amount, TxKind::Other, block.lines.join("\n")); t.merchant_raw = fl.description.clone(); t }
}

/// A17/F8: all three warning shapes read in Slovak, not English.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AccountKind, Checksum};

    fn blank_statement() -> Statement {
        let d = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        Statement { iban: "SK00".into(), account_kind: AccountKind::Personal, number: 1, period_start: d, period_end: d, opening_cents: None, closing_cents: None, transactions: Vec::new(), warnings: Vec::new() }
    }

    fn b(lines: &[&str]) -> Block { Block { lines: lines.iter().map(|s| s.to_string()).collect() } }

    #[test]
    fn unparsed_block_warning_is_slovak() {
        let mut st = blank_statement();
        absorb(&mut st, &b(&["totally unrecognized garbage line"]));
        assert_eq!(st.warnings.len(), 1, "{:?}", st.warnings);
        assert!(st.warnings[0].starts_with("Blok sa nepodarilo spracovať"), "{:?}", st.warnings);
    }

    #[test]
    fn closing_line_not_parsed_warning_is_slovak() {
        let mut st = blank_statement();
        absorb(&mut st, &b(&["Konecny zostatok bez sumy"]));
        assert_eq!(st.warnings.len(), 1, "{:?}", st.warnings);
        assert!(st.warnings[0].starts_with("Konečný zostatok sa nedá prečítať"), "{:?}", st.warnings);
    }

    #[test]
    fn unknown_kind_warning_is_slovak() {
        let mut st = blank_statement();
        absorb(&mut st, &b(&["01.06.2026    Nejaky neznamy poplatok                       1.99-"]));
        assert_eq!(st.warnings.len(), 1, "{:?}", st.warnings);
        assert!(st.warnings[0].starts_with("Neznámy typ transakcie: Nejaky neznamy poplatok"), "{:?}", st.warnings);
    }

    fn page(lines: &[&str]) -> Vec<String> { lines.iter().map(|s| s.to_string()).collect() }

    /// Page A (Michal's real page 2 shape, 2026-09-09 continuation): ends right after a card
    /// record's first line; its detail lines open the next page.
    fn page_a() -> Vec<String> {
        page(&[
            "Osobný účet     SK44 1100 0000 0000 1234 5678          Mena  EUR       BIC (SWIFT)   TATRSKBX",
            "IBAN SK44 1100 0000 0000 1234 5678",
            "Dialog  0800 00 1100              ID:   00               Výpis číslo:        7",
            "Osobný účet     SK44 1100 0000 0000 1234 5678     Majiteľ Fuchs Michal      Dátum 30.06.2026",
            "Dátum sprac.  Popis                                     Dátum zúčt.       Suma",
            "--------------------------------------------------------------------------------------------------",
            "              Posledný výpis  30.05.2026                                                     500.00",
            "--------------------------------------------------------------------------------------------------",
            "01.06.2026    EUR AP nákup POS                                                                 10.00-",
        ])
    }

    /// Page B: the page-1 card's detail lines, an incoming transfer, a fee record whose
    /// breakdown block is cut by shorter dashes (Michal's real page 9), two more card records
    /// (one E-COMM) and the closing summary glued onto the last transaction with no separator.
    fn page_b() -> Vec<String> {
        page(&[
            "Osobný účet     SK44 1100 0000 0000 1234 5678     Majiteľ Fuchs Michal      Dátum 30.06.2026",
            "--------------------------------------------------------------------------------------------------",
            "Dátum sprac.  Popis                                     Dátum zúčt.       Suma",
            "--------------------------------------------------------------------------------------------------",
            "              Číslo karty:      440000******1111      Držiteľ: FUCHS MICHAL",
            "              Miesto platby:    Neuss                 ALDI SUED",
            "              Dátum:  29.05.26  Čas:  12:13:57        Suma:          10.00- EUR",
            "--------------------------------------------------------------------------------------------------",
            "02.06.2026    Platba 0200/000000-5230000001                                                  20.00",
            "              Prijatá platba:                         P099990000000001",
            "              Suma:20.00EUR      Kurz:1.00000000     Valuta:02.06.2026",
            "              Platiteľ:",
            "              SK09 0200 0000 0052 3000 0001",
            "              Peter Vzorový",
            "--------------------------------------------------------------------------------------------------",
            "03.06.2026    Poplatky za transakcie                                                          3.00-",
            "              Transakcia                         Počet  Cena za trans.              Spolu",
            "              Výber z bankomatu v zahraničí        1        1.00                 1.00 EUR",
            "              Platba kartou                       10      v balíku                0.00 EUR",
            "------------------------------------------------------------------",
            "              Poplatok za transakcie nad rámec balíka                         2.00 EUR",
            "--------------------------------------------------------------------------------------------------",
            "04.06.2026    EUR AP nákup E-COMM                                                              5.00-",
            "              Miesto platby:    Internet             SHOP ONLINE",
            "              Dátum:  03.06.26  Čas:  10:00:00        Suma:           5.00- EUR",
            "--------------------------------------------------------------------------------------------------",
            "05.06.2026    EUR VYB.HOTOV. BANKOMAT                                                          12.00-",
            "              Číslo karty:      440000******2222      Držiteľ: FUCHS MICHAL",
            "              Miesto výberu:    JUECHEN              Sparkasse Neuss",
            "              Dátum:  05.06.26  Čas:  07:08:05        Suma:          12.00- EUR",
            "              SPOLU                        DB:        5                    -25.00",
            "              CR:            1                    20.00",
            "              Zostatok na účte ku dňu vystavenia výpisu:                           490.00",
            "Mena    EUR                                          Výpis číslo:        7        Strana:        2",
        ])
    }

    /// Discriminating test (must fail today): a page-break-interrupted card record, a fee
    /// breakdown cut by shorter dashes, an E-COMM purchase and a closing summary glued onto the
    /// last transaction, all from Michal's real statement shapes (2026-09-09 continuation).
    #[test] fn continuations_and_glued_summary_parse_clean_with_no_warnings() {
        let st = parse_pages(&[page_a(), page_b()]).unwrap();
        assert_eq!(st.warnings, Vec::<String>::new(), "{:?}", st.warnings);
        assert_eq!(st.transactions.len(), 5, "{:?}", st.transactions);
        assert_eq!(st.transactions.iter().map(|t| t.kind).collect::<Vec<_>>(), vec![TxKind::Card, TxKind::TransferIn, TxKind::Other, TxKind::Card, TxKind::Atm]);
        assert_eq!(st.transactions[0].merchant_raw, "ALDI SUED");
        assert_eq!(st.transactions[2].merchant_raw, "Poplatky za transakcie");
        assert_eq!(st.transactions[3].merchant_raw, "SHOP ONLINE");
        assert_eq!(st.opening_cents, Some(50_000));
        assert_eq!(st.closing_cents, Some(49_000));
        assert_eq!(st.checksum(), Checksum::Ok);
    }
}
