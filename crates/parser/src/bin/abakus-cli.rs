use parser::{parse_pdf, Checksum, ParseError, TxKind};
use std::io::BufRead;
use std::path::Path;

fn mask(iban: &str) -> String { format!("{}...{}", &iban[..4], &iban[iban.len() - 4..]) }

fn usage() -> ! {
    eprintln!("usage: abakus-cli check <pdf> [--password-stdin | --password <pw>]");
    eprintln!("       abakus-cli lines <pdf> [--page N] [--count M] [--password-stdin | --password <pw>]");
    eprintln!("       abakus-cli geometry <pdf> [--password-stdin | --password <pw>]");
    eprintln!("  --password-stdin reads one line from stdin. Use it for real passwords, so they");
    eprintln!("  never land in shell history or the process list. --password is for the synthetic");
    eprintln!("  test fixtures only.");
    eprintln!("  --page is 1-based (default 1). --count is the number of lines to print (default 25, max 200).");
    std::process::exit(2)
}

/// `--password-stdin` wins when both are given; it reads one line from stdin.
fn password_from_args(args: &[String]) -> Option<String> {
    if args.iter().any(|a| a == "--password-stdin") {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).ok()?;
        return Some(line.trim_end_matches(['\n', '\r']).to_string());
    }
    args.iter().position(|a| a == "--password").and_then(|i| args.get(i + 1)).cloned()
}

fn flag_number(args: &[String], flag: &str) -> Option<usize> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok())
}

/// 1-based on the command line (`--page 1` is the first page), 0-based once past this function.
fn page_index(args: &[String]) -> usize { flag_number(args, "--page").filter(|n| *n >= 1).map(|n| n - 1).unwrap_or(0) }

fn line_count(args: &[String]) -> usize { flag_number(args, "--count").map(|n| n.min(200)).unwrap_or(25) }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(cmd), Some(path)) = (args.get(1), args.get(2)) else { usage() };
    let password = password_from_args(&args);
    if cmd == "lines" { print_masked_head(Path::new(path), password.as_deref(), page_index(&args), line_count(&args)); return; }
    if cmd == "geometry" { print_char_geometry(Path::new(path), password.as_deref()); return; }
    if cmd != "check" { eprintln!("unknown command {cmd}"); std::process::exit(2); }
    match parse_pdf(Path::new(path), password.as_deref()) {
        Ok(st) => report(Path::new(path), password.as_deref(), &st),
        Err(ParseError::Encrypted) => { println!("encrypted: password missing or wrong (pass --password-stdin)"); std::process::exit(1) }
        Err(e) => { println!("error: {e}"); print_tail(Path::new(path), password.as_deref()); std::process::exit(1) }
    }
}

fn report(path: &Path, password: Option<&str>, st: &parser::Statement) {
    println!("account: {:?} {}  statement {}  period {} .. {}", st.account_kind, mask(&st.iban), st.number, st.period_start, st.period_end);
    println!("opening {:?}  closing {:?}", st.opening_cents, st.closing_cents);
    for kind in [TxKind::Card, TxKind::CardForeign, TxKind::Refund, TxKind::Atm, TxKind::TransferIn, TxKind::TransferOut, TxKind::StandingOrder, TxKind::Other] {
        let n = st.transactions.iter().filter(|t| t.kind == kind).count();
        if n > 0 { println!("  {kind:?}: {n}"); }
    }
    match st.checksum() {
        Checksum::Ok => println!("checksum: ok"),
        Checksum::OffBy(c) => println!("checksum: off by {c} cents"),
        Checksum::NotVerifiable => {
            println!("checksum: not verifiable (no closing balance found; labels tried: {:?})", parser::statement::CLOSING_LABELS);
            print_tail(path, password);
        }
    }
    for w in &st.warnings { println!("warning: {w}"); }
}

/// Diagnostic for a statement the header parser refuses, or for a later page/step once `check`
/// fails past the header: `count` lines of page `page` (both defaulted and clamped by
/// `page_index`/`line_count`) as the extractor sees them, with every digit replaced by `#` so
/// the output can be shared without account numbers, amounts or dates.
fn print_masked_head(path: &Path, password: Option<&str>, page: usize, count: usize) {
    match parser::extract_pages(path, password) {
        Ok(pages) => {
            let selected = pages.get(page).map(|p| p.as_slice()).unwrap_or(&[]);
            println!("page {}: {} lines, pages: {}", page + 1, selected.len(), pages.len());
            for (i, l) in selected.iter().take(count).enumerate() {
                let masked: String = l.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).collect();
                println!("{:>2}: {masked}", i + 1);
            }
        }
        Err(ParseError::Encrypted) => { println!("encrypted: password missing or wrong (pass --password-stdin)"); std::process::exit(1) }
        Err(e) => { println!("error: {e}"); std::process::exit(1) }
    }
}

/// Diagnostic for a page whose lines fall apart: the geometry PDFium reports for
/// the first 60 characters of page 1 (loose bounds, tight bounds, origin, font
/// size, font name), digits masked. Shows whether the bounds are degenerate.
fn print_char_geometry(path: &Path, password: Option<&str>) {
    match parser::char_geometry(path, password, 60) {
        Ok(rows) => {
            println!("idx | ch | loose x,y,w,h | tight x,y,w,h | origin x,y | font size scaled/unscaled | font");
            for r in rows { println!("{r}"); }
        }
        Err(ParseError::Encrypted) => { println!("encrypted: password missing or wrong (pass --password-stdin)"); std::process::exit(1) }
        Err(e) => { println!("error: {e}"); std::process::exit(1) }
    }
}

fn print_tail(path: &Path, password: Option<&str>) {
    if let Ok(pages) = parser::extract_pages(path, password) {
        let all: Vec<&String> = pages.iter().flatten().collect();
        println!("last 15 lines:"); for l in all.iter().rev().take(15).rev() { println!("  {l}"); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn a(s: &[&str]) -> Vec<String> { s.iter().map(|s| s.to_string()).collect() }

    #[test] fn page_defaults_to_the_first_page() { assert_eq!(page_index(&a(&[])), 0); }
    #[test] fn page_1_is_index_0() { assert_eq!(page_index(&a(&["--page", "1"])), 0); }
    #[test] fn page_3_is_index_2() { assert_eq!(page_index(&a(&["--page", "3"])), 2); }
    #[test] fn page_0_stays_at_the_default() { assert_eq!(page_index(&a(&["--page", "0"])), 0); }

    #[test] fn count_defaults_to_25() { assert_eq!(line_count(&a(&[])), 25); }
    #[test] fn count_60_is_kept() { assert_eq!(line_count(&a(&["--count", "60"])), 60); }
    #[test] fn count_above_200_is_clamped() { assert_eq!(line_count(&a(&["--count", "5000"])), 200); }
}
