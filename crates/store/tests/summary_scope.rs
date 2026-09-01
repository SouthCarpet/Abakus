//! A17/F3: the Prehľad totals per account KIND. The screen could only pass one
//! account id, so it showed the first account of a kind and quietly dropped the
//! rest. These tests pin that every account of a kind counts.
use parser::{parse_text, AccountKind};
use store::{Store, TxFilter};

const FIRST_PERSONAL: &str = "SK4411000000000012345678";
const SECOND_PERSONAL: &str = "SK3112000000198742637541";
const BUSINESS_IBAN: &str = "SK3711000000000098765432";

fn fixture(name: &str) -> parser::Statement {
    parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap()).unwrap()
}

/// One card purchase of 20.00 EUR, for whichever IBAN the caller passes.
fn small_personal_statement(iban_spaced: &str) -> parser::Statement {
    let text = format!(
"Osobný účet     {iban_spaced}          Mena  EUR                          BIC (SWIFT)   TATRSKBX
IBAN {iban_spaced}
Číslo klienta:  1234567
Majiteľ účtu:   JANA VZOROVÁ
Tatra banka, a.s., Hodžovo nám. 3
811 06 Bratislava
Dialog  0800 00 1100              ID:   00                                    Výpis číslo:        7
Osobný účet     {iban_spaced}     Majiteľ Jana Vzorová                  Dátum 31.07.2026
Dátum sprac.  Popis                                     Dátum zúčt.                              Suma
--------------------------------------------------------------------------------------------------
              Posledný výpis  30.06.2026                                                       500.00
01.07.2026    EUR AP nákup POS                                                                  20.00-
              Miesto platby:    Neuss                 BAUHAUS
              Dátum:  01.07.26  Čas:  12:00:00        Suma:          20.00- EUR
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                                       480.00
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        7        Strana:        1
");
    parse_text(&text).unwrap()
}

/// Two personal accounts and one business account, each with one statement.
fn two_personal_one_business() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account(FIRST_PERSONAL, AccountKind::Personal, "Osobný 1").unwrap();
    s.upsert_account(SECOND_PERSONAL, AccountKind::Personal, "Osobný 2").unwrap();
    s.upsert_account(BUSINESS_IBAN, AccountKind::Business, "Firemný").unwrap();
    s.import_statement(&small_personal_statement("SK44 1100 0000 0000 1234 5678"), "p1").unwrap();
    s.import_statement(&small_personal_statement("SK31 1200 0000 1987 4263 7541"), "p2").unwrap();
    s.import_statement(&fixture("business-2026-06.txt"), "b1").unwrap();
    s
}

fn account_id(s: &Store, iban: &str) -> i64 { s.account_by_iban(iban).unwrap().unwrap().id }

/// The regression itself: one account of the kind is not the kind.
#[test]
fn the_kind_filter_counts_every_account_of_that_kind() {
    let s = two_personal_one_business();
    let first = s.summary(None, None, Some(account_id(&s, FIRST_PERSONAL))).unwrap();
    let second = s.summary(None, None, Some(account_id(&s, SECOND_PERSONAL))).unwrap();

    let personal = s.summary_filtered(None, None, None, Some(AccountKind::Personal)).unwrap();

    assert_eq!(first.expense_cents, 2_000);
    assert_eq!(second.expense_cents, 2_000);
    assert_eq!(personal.expense_cents, first.expense_cents + second.expense_cents, "both personal accounts must be in the total");
    assert!(personal.expense_cents > first.expense_cents, "picking the first account of the kind is the bug this pins");
}

#[test]
fn the_kind_filter_leaves_the_other_kind_out() {
    let s = two_personal_one_business();
    let everything = s.summary(None, None, None).unwrap();

    let personal = s.summary_filtered(None, None, None, Some(AccountKind::Personal)).unwrap();
    let business = s.summary_filtered(None, None, None, Some(AccountKind::Business)).unwrap();

    assert_eq!(business.expense_cents, s.summary(None, None, Some(account_id(&s, BUSINESS_IBAN))).unwrap().expense_cents);
    assert!(business.expense_cents > 0, "the business statement has expenses of its own");
    assert_eq!(personal.expense_cents + business.expense_cents, everything.expense_cents, "the two kinds together are the whole database");
    assert_eq!(personal.income_cents + business.income_cents, everything.income_cents);
}

#[test]
fn listing_by_kind_returns_the_rows_of_every_account_of_that_kind() {
    let s = two_personal_one_business();

    let rows = s.list_transactions(&TxFilter { account_kind: Some(AccountKind::Personal), ..Default::default() }).unwrap();

    let accounts: std::collections::HashSet<i64> = rows.iter().map(|r| r.account_id).collect();
    assert_eq!(accounts.len(), 2, "rows of both personal accounts");
    assert!(rows.iter().all(|r| r.account_kind == AccountKind::Personal));
    assert!(!accounts.contains(&account_id(&s, BUSINESS_IBAN)));
}

/// Both filters at once still mean "this one account", so the kind filter
/// cannot be used to widen a single-account view by accident.
#[test]
fn an_account_id_and_a_kind_together_narrow_to_the_account() {
    let s = two_personal_one_business();
    let first = account_id(&s, FIRST_PERSONAL);

    let rows = s.list_transactions(&TxFilter { account_id: Some(first), account_kind: Some(AccountKind::Personal), ..Default::default() }).unwrap();

    assert!(rows.iter().all(|r| r.account_id == first));
    assert!(!rows.is_empty());
}
