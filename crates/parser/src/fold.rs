use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

pub fn fold(s: &str) -> String {
    let stripped: String = s.nfd().filter(|c| !is_combining_mark(*c)).collect();
    stripped.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::fold;
    #[test] fn strips_diacritics_and_case() { assert_eq!(fold("Číslo karty:"), "cislo karty:"); assert_eq!(fold("Držiteľ"), "drzitel"); }
    #[test] fn collapses_spaces() { assert_eq!(fold("  Dátum   sprac. "), "datum sprac."); }
    #[test] fn ascii_input_is_unchanged_but_lowercase() { assert_eq!(fold("Datum sprac."), "datum sprac."); }
}
