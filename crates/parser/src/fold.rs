use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

pub fn fold(s: &str) -> String {
    let stripped: String = s.nfd().filter(|c| !is_combining_mark(*c)).collect();
    stripped.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Diacritics removed, case kept. Used to render labels through fonts (Courier/WinAnsi in the
/// synthetic PDF generator) that cannot encode `č ľ ť ž` and similar.
pub fn strip_marks(s: &str) -> String { s.nfd().filter(|c| !is_combining_mark(*c)).collect() }

#[cfg(test)]
mod tests {
    use super::{fold, strip_marks};
    #[test] fn strips_diacritics_and_case() { assert_eq!(fold("Číslo karty:"), "cislo karty:"); assert_eq!(fold("Držiteľ"), "drzitel"); }
    #[test] fn collapses_spaces() { assert_eq!(fold("  Dátum   sprac. "), "datum sprac."); }
    #[test] fn ascii_input_is_unchanged_but_lowercase() { assert_eq!(fold("Datum sprac."), "datum sprac."); }

    #[test] fn strip_marks_keeps_case() { assert_eq!(strip_marks("Číslo karty: Držiteľ"), "Cislo karty: Drzitel"); }
    #[test] fn strip_marks_ascii_is_unchanged() { assert_eq!(strip_marks("EUR AP nakup POS"), "EUR AP nakup POS"); }
}
