use crate::fold::fold;

#[derive(Debug, Clone, PartialEq)]
pub struct Block { pub lines: Vec<String> }

fn is_column_header(l: &str) -> bool { let f = fold(l); f.contains("datum sprac") && f.contains("suma") }
fn is_footer(l: &str) -> bool { let f = fold(l); f.starts_with("mena") && f.contains("strana") }

pub fn body_lines(pages: &[Vec<String>]) -> Vec<String> {
    let mut out = Vec::new();
    for page in pages {
        let Some(start) = page.iter().position(|l| is_column_header(l)).map(|i| i + 1) else { continue };
        out.extend(page[start..].iter().take_while(|l| !is_footer(l)).cloned());
    }
    out
}

pub fn is_separator(l: &str) -> bool { let t = l.trim(); t.len() >= 20 && t.chars().all(|c| c == '-') }

pub fn split_blocks(lines: &[String]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    for l in lines {
        if is_separator(l) { flush(&mut cur, &mut blocks); } else if !l.trim().is_empty() { cur.push(l.clone()); }
    }
    flush(&mut cur, &mut blocks);
    blocks
}

fn flush(cur: &mut Vec<String>, blocks: &mut Vec<Block>) {
    if !cur.is_empty() { blocks.push(Block { lines: std::mem::take(cur) }); }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(lines: &[&str]) -> Vec<String> { lines.iter().map(|s| s.to_string()).collect() }

    #[test] fn header_before_column_row_and_footer_from_mena_are_dropped() {
        let p = page(&["Osobný účet  SK44 ...", "IBAN SK44 ...", "Dátum sprac. Popis        Dátum zúčt.     Suma", "01.06.2026 x", "Mena  EUR     Výpis číslo: 6   Strana: 1"]);
        assert_eq!(body_lines(&[p]), vec!["01.06.2026 x".to_string()]);
    }
    #[test] fn a_page_without_the_column_header_contributes_nothing() {
        let legal = page(&["Týmto potvrdzujem, že váš vklad je chránený v zmysle zákona o ochrane vkladov.", "Mena EUR Strana: 3"]);
        let body = page(&["Dátum sprac. Popis  Suma", "a", "Mena EUR Strana: 1"]);
        assert_eq!(body_lines(&[body, legal]), vec!["a".to_string()]);
    }
    #[test] fn body_of_two_pages_is_concatenated() {
        let p1 = page(&["Dátum sprac. Popis  Suma", "a", "Mena EUR Strana: 1"]);
        let p2 = page(&["hdr", "Dátum sprac. Popis  Suma", "b", "Mena EUR Strana: 2"]);
        assert_eq!(body_lines(&[p1, p2]), vec!["a".to_string(), "b".into()]);
    }
    #[test] fn dashed_lines_split_blocks_and_are_not_kept() {
        let lines = page(&["  Posledný výpis 30.05.2026   693.92", "-----------------------------", "01.06.2026 a", "  k: v", "-----------------------------", "02.06.2026 b"]);
        let b = split_blocks(&lines);
        assert_eq!(b.len(), 3); assert_eq!(b[1].lines, vec!["01.06.2026 a".to_string(), "  k: v".into()]);
    }
    #[test] fn separator_needs_at_least_twenty_dashes() { assert!(is_separator("   --------------------")); assert!(!is_separator("--- x ---")); }
    #[test] fn empty_lines_do_not_make_empty_blocks() { assert_eq!(split_blocks(&page(&["--------------------", "", "--------------------"])).len(), 0); }
}
