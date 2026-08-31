pub fn normalize(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase() }

fn mod97(digits: &str) -> u32 {
    digits.bytes().fold(0u32, |acc, b| (acc * 10 + u32::from(b - b'0')) % 97)
}

fn to_digits(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_digit() { c.to_string() } else { (c as u32 - 'A' as u32 + 10).to_string() }).collect()
}

pub fn is_valid(s: &str) -> bool {
    let n = normalize(s);
    if n.len() < 15 || !n.chars().all(|c| c.is_ascii_alphanumeric()) { return false; }
    let rearranged = format!("{}{}", &n[4..], &n[..4]);
    mod97(&to_digits(&rearranged)) == 1
}

pub fn from_sk_parts(bank: &str, prefix: &str, number: &str) -> String {
    let bban = format!("{bank:0>4}{prefix:0>6}{number:0>10}");
    let check = 98 - mod97(&to_digits(&format!("{bban}SK00")));
    format!("SK{check:02}{bban}")
}

pub fn parse_account_ref(s: &str) -> Option<(String, String, String)> {
    let (bank, rest) = s.trim().split_once('/')?;
    let (prefix, number) = rest.split_once('-')?;
    let ok = |x: &str, max: usize| !x.is_empty() && x.len() <= max && x.chars().all(|c| c.is_ascii_digit());
    (ok(bank, 4) && ok(prefix, 6) && ok(number, 10)).then(|| (bank.into(), prefix.into(), number.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn wikipedia_example_is_valid() { assert!(is_valid("SK31 1200 0000 1987 4263 7541")); }
    #[test] fn wrong_check_digits_are_invalid() { assert!(!is_valid("SK32 1200 0000 1987 4263 7541")); }
    #[test] fn parts_build_a_valid_iban() { assert_eq!(from_sk_parts("1100", "000000", "0012345678"), "SK4411000000000012345678"); }
    #[test] fn parts_are_zero_padded() { assert_eq!(from_sk_parts("1200", "19", "8742637541"), "SK3112000000198742637541"); }
    #[test] fn account_ref_splits() { assert_eq!(parse_account_ref("0200/000000-5230000001"), Some(("0200".into(), "000000".into(), "5230000001".into()))); }
    #[test] fn foreign_reference_is_not_an_account_ref() { assert_eq!(parse_account_ref("0026060200022221"), None); }
    #[test] fn normalize_strips_spaces_and_lowercase() { assert_eq!(normalize("sk44 1100 0000 0000 1234 5678"), "SK4411000000000012345678"); }
}
