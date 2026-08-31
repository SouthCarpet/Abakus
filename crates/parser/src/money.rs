use crate::Cents;
use chrono::NaiveDate;

pub fn parse_amount(s: &str) -> Option<Cents> {
    let t = s.trim().replace(',', "");
    let (body, neg) = match t.strip_suffix('-') { Some(b) => (b, true), None => (t.as_str(), false) };
    let (whole, frac) = body.split_once('.')?;
    if frac.len() != 2 || whole.is_empty() { return None; }
    let cents = whole.parse::<i64>().ok()? * 100 + frac.parse::<i64>().ok()?;
    Some(if neg { -cents } else { cents })
}

pub fn parse_rate(s: &str) -> Option<i64> {
    let (whole, frac) = s.trim().split_once('.')?;
    let frac6: String = format!("{frac:0<6}").chars().take(6).collect();
    Some(whole.parse::<i64>().ok()? * 1_000_000 + frac6.parse::<i64>().ok()?)
}

pub fn parse_date_long(s: &str) -> Option<NaiveDate> { NaiveDate::parse_from_str(s.trim(), "%d.%m.%Y").ok() }
pub fn parse_date_short(s: &str) -> Option<NaiveDate> { NaiveDate::parse_from_str(s.trim(), "%d.%m.%y").ok() }

pub fn format_cents(c: Cents) -> String {
    let sign = if c < 0 { "-" } else { "" };
    let abs = c.abs();
    let whole = (abs / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) { grouped.push(' '); }
        grouped.push(ch);
    }
    format!("{sign}{grouped},{:02}", abs % 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test] fn trailing_minus_is_negative() { assert_eq!(parse_amount("1.99-"), Some(-199)); }
    #[test] fn thousands_comma_is_ignored() { assert_eq!(parse_amount("1,300.00-"), Some(-130_000)); }
    #[test] fn plain_is_positive() { assert_eq!(parse_amount("693.92"), Some(69_392)); }
    #[test] fn garbage_is_none() { assert_eq!(parse_amount("EUR"), None); }
    #[test] fn rate_to_micros() { assert_eq!(parse_rate("1.1237"), Some(1_123_700)); assert_eq!(parse_rate("1.00000000"), Some(1_000_000)); }
    #[test] fn long_date() { assert_eq!(parse_date_long("01.06.2026"), NaiveDate::from_ymd_opt(2026, 6, 1)); }
    #[test] fn short_date_is_2000_based() { assert_eq!(parse_date_short("29.05.26"), NaiveDate::from_ymd_opt(2026, 5, 29)); }
    #[test] fn format_uses_sk_style() { assert_eq!(format_cents(-123_456), "-1 234,56"); assert_eq!(format_cents(5), "0,05"); }
}
