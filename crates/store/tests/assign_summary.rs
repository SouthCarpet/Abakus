use parser::{parse_text, AccountKind};
use rules::{RuleKind, Status};
use store::{Store, TxFilter};

fn fixture(name: &str) -> parser::Statement { parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap()).unwrap() }
fn loaded() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    s.import_statement(&fixture("business-2026-06.txt"), "h2").unwrap();
    s
}
fn id_of(s: &Store, merchant: &str) -> i64 { s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.merchant_raw == merchant).unwrap().id }

#[test] fn totals_exclude_transfers_and_net_refunds() {
    let sm = loaded().summary(None, None, None).unwrap();
    assert_eq!(sm.income_cents, 151_000); assert_eq!(sm.expense_cents, 52_199); assert_eq!(sm.transfer_cents, 138_000); assert_eq!(sm.net_cents, 151_000 - 52_199);
    // 5, not 6: `Poplatok za vedenie účtu` hits the `poplatok` seed and is Suggested (review fix 2026-08-30).
    assert_eq!(sm.unassigned_count, 5); assert_eq!(sm.by_month.len(), 2);
}
#[test] fn account_filter_limits_the_summary() {
    let s = loaded(); let business = s.list_accounts().unwrap().into_iter().find(|a| a.kind == AccountKind::Business).unwrap().id;
    let sm = s.summary(None, None, Some(business)).unwrap();
    assert_eq!(sm.income_cents, 150_000); assert_eq!(sm.expense_cents, 32_231);
}
#[test] fn date_filter_uses_tx_date_not_posted_date() {
    let s = loaded();
    let june = s.list_transactions(&TxFilter { from: chrono::NaiveDate::from_ymd_opt(2026, 6, 1), ..Default::default() }).unwrap();
    assert!(june.iter().all(|r| r.tx_date >= chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap())); assert!(june.iter().all(|r| r.merchant_raw != "ALDI SUED"));
}
#[test] fn assign_confirms_and_creates_exact_and_merchant_rules() {
    let mut s = loaded(); let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    let o = s.assign(&[id_of(&s, "OF")], cat, false).unwrap();
    assert_eq!((o.updated, o.rules_created), (1, 2));
    let rules = s.list_rules().unwrap();
    assert!(rules.iter().any(|r| r.kind == RuleKind::Exact && r.key == "of" && r.place.as_deref() == Some("london") && r.category_id == cat));
    assert!(rules.iter().any(|r| r.kind == RuleKind::Merchant && r.key == "of" && r.category_id == cat));
    let row = s.list_transactions(&TxFilter { text: Some("OF".into()), ..Default::default() }).unwrap().into_iter().find(|r| r.merchant_raw == "OF").unwrap();
    assert_eq!((row.status, row.category_id), (Status::Confirmed, Some(cat)));
}
#[test] fn confirm_turns_a_suggestion_into_a_rule() {
    let mut s = loaded(); let aldi = id_of(&s, "ALDI SUED");
    assert_eq!(s.confirm(&[aldi]).unwrap(), 1);
    let row = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == aldi).unwrap();
    assert_eq!(row.status, Status::Confirmed); assert!(s.list_rules().unwrap().iter().any(|r| r.kind == RuleKind::Exact && r.key == "aldi sued"));
}

/// A4 rebuild: two genuine Amazon card purchases (not the personal fixture's
/// Amazon refund) in a small inline statement, so `apply_to_matching` has a
/// second real row of the same merchant to reclassify, and `OF` (from the
/// unrelated `loaded()` fixtures) proves the merchant filter does not spill
/// over.
const AMAZON_JULY: &str = "Osobný účet     SK44 1100 0000 0000 1234 5678          Mena  EUR                          BIC (SWIFT)   TATRSKBX
IBAN SK44 1100 0000 0000 1234 5678
Číslo klienta:  1234567
Majiteľ účtu:   JANA VZOROVÁ
Tatra banka, a.s., Hodžovo nám. 3
811 06 Bratislava
Dialog  0800 00 1100              ID:   00                                    Výpis číslo:        7
Osobný účet     SK44 1100 0000 0000 1234 5678     Majiteľ Jana Vzorová                  Dátum 31.07.2026
Dátum sprac.  Popis                                     Dátum zúčt.                              Suma
--------------------------------------------------------------------------------------------------
              Posledný výpis  30.06.2026                                                       500.00
01.07.2026    EUR AP nákup POS                                                                  20.00-
              Miesto platby:    Neuss                 AMAZON* AB1111CD2
              Dátum:  01.07.26  Čas:  12:00:00        Suma:          20.00- EUR
--------------------------------------------------------------------------------------------------
02.07.2026    EUR AP nákup POS                                                                  30.00-
              Miesto platby:    LUXEMBOURG            AMAZON* XY2222ZZ3
              Dátum:  02.07.26  Čas:  13:00:00        Suma:          30.00- EUR
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                                       450.00
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        7        Strana:        1
";

#[test] fn apply_to_matching_reclassifies_open_rows_of_that_merchant() {
    let mut s = loaded();
    let of = id_of(&s, "OF");
    s.import_statement(&fixture_from(AMAZON_JULY), "amz-extra").unwrap();
    let cat = s.category_by_path("Nákupy/obchod").unwrap().unwrap();
    let first = id_of(&s, "AMAZON* AB1111CD2");
    s.assign(&[first], cat, true).unwrap();
    let rows = s.list_transactions(&TxFilter::default()).unwrap();
    assert!(rows.iter().filter(|r| r.merchant_raw.starts_with("AMAZON")).all(|r| r.category_id == Some(cat)));
    let second = rows.iter().find(|r| r.merchant_raw == "AMAZON* XY2222ZZ3").unwrap();
    assert_eq!((second.status, second.category_id), (Status::Confirmed, Some(cat)));
    let of_after = rows.iter().find(|r| r.id == of).unwrap();
    assert_eq!(of_after.status, Status::Unassigned);
}
fn fixture_from(text: &str) -> parser::Statement { parse_text(text).unwrap() }

#[test] fn deleting_a_rule_reopens_the_suggestion_it_made() {
    let mut s = loaded();
    let aldi = id_of(&s, "ALDI SUED");
    let seed_rule = s.list_rules().unwrap().into_iter().find(|r| r.kind == RuleKind::Seed && r.key == "aldi").unwrap().id;
    // `delete_rule` runs `reclassify_open` itself (A4): no manual poke needed.
    s.delete_rule(seed_rule).unwrap();
    let row = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == aldi).unwrap();
    assert_eq!(row.status, Status::Unassigned);
}
#[test] fn csv_has_header_and_one_line_per_row() {
    let csv = loaded().export_csv(&TxFilter::default()).unwrap();
    assert!(csv.starts_with("datum;ucet;obchodnik;miesto;suma_eur;kategoria;podkategoria;stav\n")); assert_eq!(csv.lines().count(), 13);
}

/// A4: one call exercises every numbered parameter, including the reused
/// ones (`category_id` twice, `text` three times).
#[test] fn combined_filter_matches_the_exact_surviving_row() {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
    let personal_outcome = s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    s.import_statement(&fixture("business-2026-06.txt"), "h2").unwrap();
    let personal = s.list_accounts().unwrap().into_iter().find(|a| a.kind == AccountKind::Personal).unwrap().id;
    let aldi = id_of(&s, "ALDI SUED");
    let cat = s.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    let f = TxFilter {
        from: chrono::NaiveDate::from_ymd_opt(2026, 5, 1),
        to: chrono::NaiveDate::from_ymd_opt(2026, 6, 30),
        account_id: Some(personal),
        category_id: Some(cat),
        status: Some(Status::Suggested),
        text: Some("ALDI".into()),
        statement_id: Some(personal_outcome.statement_id),
    };
    let rows = s.list_transactions(&f).unwrap();
    assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![aldi]);
}

/// A1: transfer rows are a protected invariant; `assign` never touches them.
#[test] fn assign_skips_transfer_rows_and_reports_them() {
    let mut s = loaded();
    let transfer_row = s.list_transactions(&TxFilter { status: Some(Status::Transfer), ..Default::default() }).unwrap().into_iter().next().unwrap();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    let o = s.assign(&[transfer_row.id], cat, false).unwrap();
    assert_eq!((o.updated, o.skipped_transfers), (0, 1));
    let after = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == transfer_row.id).unwrap();
    assert_eq!(after.status, Status::Transfer);
}
#[test] fn assign_rolls_back_the_whole_batch_on_any_failure() {
    let mut s = loaded();
    let of = id_of(&s, "OF");
    let before = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == of).unwrap();
    let bad_category = 999_999; // does not exist: violates the categories FK
    let missing_id = -1; // does not exist either
    assert!(s.assign(&[of, missing_id], bad_category, false).is_err());
    let after = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == of).unwrap();
    assert_eq!(after, before);
}
#[test] fn reclassify_after_account_change_turns_a_now_own_transfer_and_moves_the_summary() {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    let before = s.summary(None, None, None).unwrap();
    // The TPP row's counterparty is the business account, not yet registered: it lands unassigned.
    let tpp = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.counterparty_iban.as_deref() == Some("SK3711000000000098765432")).unwrap();
    assert_eq!(tpp.status, Status::Unassigned);
    s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
    s.reclassify_after_account_change().unwrap();
    let row = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == tpp.id).unwrap();
    assert_eq!(row.status, Status::Transfer);
    let after = s.summary(None, None, None).unwrap();
    assert!(after.transfer_cents > before.transfer_cents);
}

/// A1: `reclassify_after_account_change` must flip a row to `Transfer` and
/// drop its category even when the row was already confirmed, not just
/// suggested or unassigned. This pins the UPDATE having no `status IN (...)`
/// narrowing.
#[test] fn reclassify_after_account_change_flips_even_confirmed_rows() {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    let tpp = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.counterparty_iban.as_deref() == Some("SK3711000000000098765432")).unwrap();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    s.assign(&[tpp.id], cat, false).unwrap();
    let confirmed = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == tpp.id).unwrap();
    assert_eq!(confirmed.status, Status::Confirmed);
    s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
    s.reclassify_after_account_change().unwrap();
    let row = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == tpp.id).unwrap();
    assert_eq!(row.status, Status::Transfer);
    assert_eq!(row.category_id, None);
}

/// A1: `confirm` only turns a `Suggested` row into a rule; it must never
/// touch a `Transfer` row.
#[test] fn confirm_never_touches_a_transfer_row() {
    let mut s = loaded();
    let transfer_row = s.list_transactions(&TxFilter { status: Some(Status::Transfer), ..Default::default() }).unwrap().into_iter().next().unwrap();
    assert_eq!(s.confirm(&[transfer_row.id]).unwrap(), 0);
    let after = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == transfer_row.id).unwrap();
    assert_eq!(after.status, Status::Transfer);
    assert_eq!(after.category_id, None);
}

/// A3: checksum banner data.
#[test] fn statements_with_bad_checksum_reports_the_off_by_amount() {
    let mut st = fixture("business-2026-06.txt");
    st.closing_cents = Some(208_800); // was 2,088.32; edited to 2,088.00
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account("SK3711000000000098765432", AccountKind::Business, "Firemný").unwrap();
    s.import_statement(&st, "h1").unwrap();
    let bad = s.statements_with_bad_checksum().unwrap();
    assert_eq!(bad.len(), 1);
    assert_eq!(bad[0].off_by_cents, 32);
    assert_eq!(bad[0].account_label, "Firemný");
}
