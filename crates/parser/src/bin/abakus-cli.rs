use parser::{parse_pdf, Checksum, ParseError, TxKind};
use std::io::BufRead;
use std::path::Path;

fn mask(iban: &str) -> String { format!("{}...{}", &iban[..4], &iban[iban.len() - 4..]) }

fn usage() -> ! {
    eprintln!("usage: abakus-cli check <pdf> [--password-stdin | --password <pw>]");
    eprintln!("  --password-stdin reads one line from stdin. Use it for real passwords, so they");
    eprintln!("  never land in shell history or the process list. --password is for the synthetic");
    eprintln!("  test fixtures only.");
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(cmd), Some(path)) = (args.get(1), args.get(2)) else { usage() };
    let password = password_from_args(&args);
    if cmd == "lines" { print_masked_head(Path::new(path), password.as_deref()); return; }
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

/// Diagnostic for a statement the header parser refuses: the first 25 lines of
/// page 1 as the extractor sees them, with every digit replaced by `#` so the
/// output can be shared without account numbers, amounts or dates.
fn print_masked_head(path: &Path, password: Option<&str>) {
    match parser::extract_pages(path, password) {
        Ok(pages) => {
            let page1 = pages.first().map(|p| p.as_slice()).unwrap_or(&[]);
            println!("page 1: {} lines, pages: {}", page1.len(), pages.len());
            for (i, l) in page1.iter().take(25).enumerate() {
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
