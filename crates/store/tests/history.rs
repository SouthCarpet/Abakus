//! 0.1.2: `statement_history`, scoping/ordering/null-handling/checksum.
use parser::{parse_text, AccountKind, Checksum};
use store::Store;

const FIRST_PERSONAL: &str = "SK4411000000000012345678";
const SECOND_PERSONAL: &str = "SK3112000000198742637541";
const BUSINESS_IBAN: &str = "SK3711000000000098765432";

fn fixture(name: &str) -> parser::Statement {
    parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap()).unwrap()
}

/// One card purchase, for whichever IBAN/period the caller passes. Mirrors
/// `summary_scope.rs`'s helper of the same shape.
fn small_personal_statement(iban_spaced: &str, number: u32, header_date: &str, posted: &str) -> parser::Statement {
    let text = format!(
"Osobný účet     {iban_spaced}          Mena  EUR                          BIC (SWIFT)   TATRSKBX
IBAN {iban_spaced}
Číslo klienta:  1234567
Majiteľ účtu:   JANA VZOROVÁ
Tatra banka, a.s., Hodžovo nám. 3
811 06 Bratislava
Dialog  0800 00 1100              ID:   00                                    Výpis číslo:        {number}
Osobný účet     {iban_spaced}     Majiteľ Jana Vzorová                  Dátum {header_date}
Dátum sprac.  Popis                                     Dátum zúčt.                              Suma
--------------------------------------------------------------------------------------------------
              Posledný výpis  {header_date}                                                       500.00
{posted}    EUR AP nákup POS                                                                  20.00-
              Miesto platby:    Neuss                 BAUHAUS
              Dátum:  {posted}  Čas:  12:00:00        Suma:          20.00- EUR
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                                       480.00
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        {number}        Strana:        1
");
    parse_text(&text).unwrap()
}

/// No "Posledný výpis" and no closing line: `opening_cents`/`closing_cents`
/// stay `None`, and `period_start`/`period_end` both fall back to the header
/// date. A real Tatra banka export never omits these two lines, but the
/// parser already treats a missing one as unverifiable rather than as an
/// error (`Statement::checksum`), so `statement_history` must hand back a
/// usable row for it too, not panic on the null case.
fn statement_with_no_opening_or_closing(iban_spaced: &str) -> parser::Statement {
    let text = format!(
"Osobný účet     {iban_spaced}          Mena  EUR                          BIC (SWIFT)   TATRSKBX
IBAN {iban_spaced}
Číslo klienta:  1234567
Majiteľ účtu:   JANA VZOROVÁ
Tatra banka, a.s., Hodžovo nám. 3
811 06 Bratislava
Dialog  0800 00 1100              ID:   00                                    Výpis číslo:        99
Osobný účet     {iban_spaced}     Majiteľ Jana Vzorová                  Dátum 15.08.2026
Dátum sprac.  Popis                                     Dátum zúčt.                              Suma
--------------------------------------------------------------------------------------------------
01.08.2026    EUR AP nákup POS                                                                  5.00-
              Miesto platby:    Neuss                 TESTMERCHANT
              Dátum:  01.08.26  Čas:  12:00:00        Suma:          5.00- EUR
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        99        Strana:        1
");
    parse_text(&text).unwrap()
}

/// FIRST_PERSONAL gets TWO statements (June, then a later July one) so the
/// period_start/period_end sort within one account is exercised. SECOND_PERSONAL
/// gets the opening/closing-less statement (null handling). BUSINESS gets one
/// ordinary statement.
fn four_statements_three_accounts() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account(FIRST_PERSONAL, AccountKind::Personal, "Osobný 1").unwrap();
    s.upsert_account(SECOND_PERSONAL, AccountKind::Personal, "Osobný 2").unwrap();
    s.upsert_account(BUSINESS_IBAN, AccountKind::Business, "Firemný").unwrap();
    // Imported deliberately out of period order, to prove the result is SORTED, not insertion order.
    s.import_statement(&small_personal_statement("SK44 1100 0000 0000 1234 5678", 7, "31.07.2026", "01.07.2026"), "p1-july").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "p1-june").unwrap();
    s.import_statement(&statement_with_no_opening_or_closing("SK31 1200 0000 1987 4263 7541"), "p2-null").unwrap();
    s.import_statement(&fixture("business-2026-06.txt"), "b1").unwrap();
    s
}

fn account_id(s: &Store, iban: &str) -> i64 { s.account_by_iban(iban).unwrap().unwrap().id }

#[test]
fn no_filters_returns_every_statement_sorted_by_account_then_period_then_id() {
    let s = four_statements_three_accounts();

    let rows = s.statement_history(None, None).unwrap();

    assert_eq!(rows.len(), 4);
    let first = account_id(&s, FIRST_PERSONAL);
    let second = account_id(&s, SECOND_PERSONAL);
    let business = account_id(&s, BUSINESS_IBAN);
    // account_id ascending first...
    assert!(rows[0].account_id == first && rows[1].account_id == first, "both FIRST_PERSONAL rows must come before the next account");
    assert_eq!(rows[2].account_id, second);
    assert_eq!(rows[3].account_id, business);
    // ...and within FIRST_PERSONAL, period_start ascending (June before July) even though July was imported first.
    assert!(rows[0].period_start < rows[1].period_start, "June must sort before July regardless of import order");
    assert_eq!(rows[0].number, 6);
    assert_eq!(rows[1].number, 7);
}

#[test]
fn an_account_id_filter_narrows_to_that_account_only() {
    let s = four_statements_three_accounts();
    let first = account_id(&s, FIRST_PERSONAL);

    let rows = s.statement_history(Some(first), None).unwrap();

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.account_id == first));
}

#[test]
fn an_account_kind_filter_covers_every_account_of_that_kind() {
    let s = four_statements_three_accounts();

    let rows = s.statement_history(None, Some(AccountKind::Personal)).unwrap();

    assert_eq!(rows.len(), 3, "both personal accounts' statements, not just the first");
    assert!(rows.iter().all(|r| r.account_kind == AccountKind::Personal));
}

#[test]
fn an_account_id_and_a_kind_together_still_mean_just_that_one_account() {
    let s = four_statements_three_accounts();
    let first = account_id(&s, FIRST_PERSONAL);

    let rows = s.statement_history(Some(first), Some(AccountKind::Personal)).unwrap();

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.account_id == first));
}

#[test]
fn a_statement_with_no_opening_or_closing_line_reports_null_amounts_and_an_unverifiable_checksum() {
    let s = four_statements_three_accounts();
    let second = account_id(&s, SECOND_PERSONAL);

    let row = s.statement_history(Some(second), None).unwrap().into_iter().next().unwrap();

    assert_eq!(row.opening_cents, None);
    assert_eq!(row.closing_cents, None);
    assert_eq!(row.checksum, Checksum::NotVerifiable);
    assert_eq!(row.period_start, row.period_end, "with no opening line, both fall back to the header date");
}

#[test]
fn an_ordinary_statement_reports_its_real_opening_closing_and_an_ok_checksum() {
    let s = four_statements_three_accounts();
    let first = account_id(&s, FIRST_PERSONAL);

    let june = s.statement_history(Some(first), None).unwrap().into_iter().find(|r| r.number == 6).unwrap();

    assert_eq!(june.opening_cents, Some(69_392));
    assert_eq!(june.closing_cents, Some(42_424));
    assert_eq!(june.checksum, Checksum::Ok);
    assert_eq!(june.account_label, "Osobný 1");
}
