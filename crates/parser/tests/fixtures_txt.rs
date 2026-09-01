use chrono::NaiveDate;
use parser::{parse_text, AccountKind, Checksum, TxKind};

fn load(name: &str) -> parser::Statement {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name;
    parse_text(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn d(y: i32, m: u32, day: u32) -> NaiveDate { NaiveDate::from_ymd_opt(y, m, day).unwrap() }

#[test] fn personal_header_and_period() {
    let s = load("personal-2026-06.txt");
    assert_eq!(s.iban, "SK4411000000000012345678"); assert_eq!(s.account_kind, AccountKind::Personal); assert_eq!(s.number, 6);
    assert_eq!(s.period_start, d(2026, 5, 30)); assert_eq!(s.period_end, d(2026, 6, 30)); assert_eq!(s.opening_cents, Some(69_392)); assert_eq!(s.closing_cents, Some(42_424));
}
#[test] fn personal_has_eight_records_in_order() {
    let s = load("personal-2026-06.txt");
    assert_eq!(s.transactions.iter().map(|t| t.kind).collect::<Vec<_>>(), vec![TxKind::Card, TxKind::TransferIn, TxKind::Refund, TxKind::TransferOut, TxKind::Atm, TxKind::StandingOrder, TxKind::CardForeign, TxKind::Other]);
    assert_eq!(s.transactions.iter().map(|t| t.amount_cents).collect::<Vec<_>>(), vec![-199, 1000, 4300, -4000, -18_000, -8000, -1369, -700]);
}
#[test] fn personal_checksum_is_ok_and_the_only_warning_is_the_unknown_fee_kind() {
    let s = load("personal-2026-06.txt");
    assert_eq!(s.checksum(), Checksum::Ok);
    assert_eq!(s.warnings.len(), 1, "{:?}", s.warnings); assert!(s.warnings[0].starts_with("Neznámy typ transakcie"), "{:?}", s.warnings); assert!(s.warnings[0].contains("Poplatok"));
}
#[test] fn other_block_keeps_its_description_as_merchant() {
    assert_eq!(load("personal-2026-06.txt").transactions[7].merchant_raw, "Poplatok za vedenie účtu");
}
#[test] fn standing_order_targets_the_business_account() {
    assert_eq!(load("personal-2026-06.txt").transactions[5].counterparty_iban.as_deref(), Some("SK3711000000000098765432"));
}
#[test] fn business_is_business_with_four_records_and_ok_checksum() {
    let s = load("business-2026-06.txt");
    assert_eq!(s.account_kind, AccountKind::Business); assert_eq!(s.transactions.len(), 4); assert_eq!(s.checksum(), Checksum::Ok);
    assert_eq!(s.transactions[2].counterparty_iban.as_deref(), Some("SK4411000000000012345678")); assert_eq!(s.transactions[3].counterparty_name.as_deref(), Some("Firma Alfa s.r.o."));
}
#[test] fn missing_closing_balance_is_not_verifiable() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/business-2026-06.txt")).unwrap();
    let cut: String = text.lines().filter(|l| !l.contains("Zostatok na účte")).collect::<Vec<_>>().join("\n");
    assert_eq!(parse_text(&cut).unwrap().checksum(), Checksum::NotVerifiable);
}
#[test] fn wrong_closing_balance_reports_the_difference() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/business-2026-06.txt")).unwrap();
    assert_eq!(parse_text(&text.replace("2,088.32", "2,088.00")).unwrap().checksum(), Checksum::OffBy(32));
}
#[test] fn garbage_is_rejected() { assert!(matches!(parse_text("hello\nworld"), Err(parser::ParseError::NotAStatement(_)))); }
#[test] fn missing_opening_line_is_not_verifiable_even_if_sums_agree() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/business-2026-06.txt")).unwrap();
    let cut: String = text.lines().filter(|l| !l.contains("Posledný výpis")).collect::<Vec<_>>().join("\n");
    assert_eq!(parse_text(&cut).unwrap().checksum(), Checksum::NotVerifiable);
}

// Binding amendment A8: layout edge cases (empty statement, wrapped closing line, block split by a page break).
#[test] fn empty_statement_has_no_records_and_an_ok_checksum() {
    let s = load("edge-cases/empty-2026-07.txt");
    assert_eq!(s.transactions.len(), 0);
    assert_eq!(s.opening_cents, Some(25_000)); assert_eq!(s.closing_cents, Some(25_000));
    assert_eq!(s.checksum(), Checksum::Ok);
}
#[test] fn wrapped_closing_line_is_still_parsed() {
    let s = load("edge-cases/wrapped-closing-2026-07.txt");
    assert_eq!(s.closing_cents, Some(18_000));
    assert_eq!(s.checksum(), Checksum::Ok);
}
#[test] fn card_block_split_by_a_page_break_stays_one_transaction() {
    let s = load("edge-cases/split-block-2026-07.txt");
    assert_eq!(s.transactions.len(), 1);
    let t = &s.transactions[0];
    assert_eq!(t.merchant_raw, "ALDI SUED"); assert_eq!(t.place.as_deref(), Some("Neuss")); assert_eq!(t.tx_date, d(2026, 6, 29));
    assert_eq!(s.checksum(), Checksum::Ok);
}

// A12: unknown-account fixture (IBAN not on any registered account), used by
// Task 16's import test and by the Import screen's unknown-account screenshot.
#[test] fn unknown_account_fixture_parses_with_its_own_iban() {
    let s = load("edge-cases/unknown-account-2026-07.txt");
    assert_eq!(s.iban, "SK8911000000000055555555"); assert_eq!(s.account_kind, AccountKind::Personal); assert_eq!(s.number, 1);
    assert_eq!(s.transactions.len(), 0);
    assert_eq!(s.checksum(), Checksum::Ok);
}
