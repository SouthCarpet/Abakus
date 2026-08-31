use parser::fold::fold;
use regex::Regex;
use std::sync::OnceLock;

/// A token is an id (store number, card reference) when it is all digits, or
/// when it mixes letters with 3+ digits in any shape EXCEPT a single interior
/// digit group with letters on both sides (`Douglas506Neuss` stays,
/// `NQ97D00C4` and `4029357733` go). Review fix 2026-08-30: filter RAW
/// whitespace tokens first, then clean punctuation per kept token - the old
/// zip of cleaned-vs-raw tokens drifted on hyphenated names.
fn is_id_token(t: &str) -> bool {
    let digits = t.chars().filter(char::is_ascii_digit).count();
    if digits == 0 { return false; }
    if t.chars().all(|c| c.is_ascii_digit()) { return true; }
    if digits < 3 { return false; }
    static INTERIOR: OnceLock<Regex> = OnceLock::new();
    !INTERIOR.get_or_init(|| Regex::new(r"^[[:alpha:]]+[0-9]+[[:alpha:]]+$").unwrap()).is_match(t)
}

pub fn normalize(name: &str) -> String {
    name.split_whitespace()
        .filter(|t| !is_id_token(t))
        .map(|t| fold(&t.chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect::<String>()))
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::normalize;
    #[test] fn lowercases_and_strips_diacritics() { assert_eq!(normalize("Reštaurácia U Tomáša"), "restauracia u tomasa"); }
    #[test] fn drops_store_numbers() { assert_eq!(normalize("MCDONALDS 01609"), "mcdonalds"); assert_eq!(normalize("SHELL 0249"), "shell"); }
    #[test] fn drops_mixed_ids_with_three_or_more_digits() { assert_eq!(normalize("AMAZON* NQ97D00C4"), "amazon"); }
    #[test] fn keeps_short_alnum_tokens() { assert_eq!(normalize("Douglas506Neuss"), "douglas506neuss"); assert_eq!(normalize("O2 Slovakia"), "o2 slovakia"); }
    #[test] fn strips_punctuation() { assert_eq!(normalize("Netto Marken-Discount"), "netto marken discount"); assert_eq!(normalize("PAYPAL *NETFLIX COM"), "paypal netflix com"); }
    #[test] fn empty_stays_empty() { assert_eq!(normalize("  "), ""); }
}
