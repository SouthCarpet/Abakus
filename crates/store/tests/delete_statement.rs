//! A17/F1: deleting one imported statement. These tests pin the cascade from
//! the outside (public store API only), so a later narrowing of any single
//! step has to break one of them.
use parser::{parse_text, AccountKind, Checksum};
use rules::{RuleKind, Status};
use store::{Store, StoreError, TxFilter};

const PERSONAL_IBAN: &str = "SK4411000000000012345678";
const BUSINESS_IBAN: &str = "SK3711000000000098765432";

fn fixture(name: &str) -> parser::Statement {
    parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap()).unwrap()
}

fn loaded() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
    s.upsert_account(BUSINESS_IBAN, AccountKind::Business, "Firemný").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    s.import_statement(&fixture("business-2026-06.txt"), "h2").unwrap();
    s
}

/// One personal statement with a single card purchase, so two statements can
/// share a merchant and differ only in the place (which is what tells an
/// exact rule from a merchant rule apart).
fn card_statement(number: u32, date: &str, day: &str, short_day: &str, place: &str, merchant: &str) -> parser::Statement {
    let text = format!(
"Osobný účet     SK44 1100 0000 0000 1234 5678          Mena  EUR                          BIC (SWIFT)   TATRSKBX
IBAN SK44 1100 0000 0000 1234 5678
Číslo klienta:  1234567
Majiteľ účtu:   JANA VZOROVÁ
Tatra banka, a.s., Hodžovo nám. 3
811 06 Bratislava
Dialog  0800 00 1100              ID:   00                                    Výpis číslo:        {number}
Osobný účet     SK44 1100 0000 0000 1234 5678     Majiteľ Jana Vzorová                  Dátum {date}
Dátum sprac.  Popis                                     Dátum zúčt.                              Suma
--------------------------------------------------------------------------------------------------
              Posledný výpis  30.06.2026                                                       500.00
{day}    EUR AP nákup POS                                                                  20.00-
              Miesto platby:    {place}                 {merchant}
              Dátum:  {short_day}  Čas:  12:00:00        Suma:          20.00- EUR
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                                       480.00
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        {number}        Strana:        1
");
    parse_text(&text).unwrap()
}

/// Two statements of the same account with the same merchant in two places.
fn two_bauhaus_statements() -> (Store, i64, i64) {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
    let july = s.import_statement(&card_statement(7, "31.07.2026", "01.07.2026", "01.07.26", "Neuss", "BAUHAUS"), "h-july").unwrap().statement_id;
    let august = s.import_statement(&card_statement(8, "31.08.2026", "01.08.2026", "01.08.26", "Erfttal", "BAUHAUS"), "h-august").unwrap().statement_id;
    (s, july, august)
}

fn rows(s: &Store) -> Vec<store::TxRow> { s.list_transactions(&TxFilter::default()).unwrap() }
fn row_of(s: &Store, statement_id: i64) -> store::TxRow { s.list_transactions(&TxFilter { statement_id: Some(statement_id), ..Default::default() }).unwrap().into_iter().next().unwrap() }
fn rule_exists(s: &Store, kind: RuleKind, key: &str, place: Option<&str>) -> bool {
    s.list_rules().unwrap().iter().any(|r| r.kind == kind && r.key == key && r.place.as_deref() == place)
}
fn hit_count(s: &Store, kind: RuleKind, key: &str) -> i64 {
    s.list_rules_view().unwrap().into_iter().find(|r| r.kind == kind && r.key == key).map(|r| r.hit_count).unwrap()
}

/// The point of the whole feature: after a delete the same PDF imports again.
/// Fails if the statement row survives (file_hash is UNIQUE) or if the
/// transactions survive (fingerprint is UNIQUE, so the rows would come back as
/// duplicates instead of inserts).
#[test]
fn deleting_a_statement_releases_the_file_hash_and_the_fingerprints() {
    let mut s = loaded();
    let personal = s.recent_statements(10).unwrap().into_iter().find(|r| r.account_label == "Osobný").unwrap().statement_id;

    let outcome = s.delete_statement(personal).unwrap();
    assert_eq!(outcome.transactions_deleted, 8);
    assert_eq!(outcome.number, 6);

    let again = s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    assert!(!again.already_imported, "the same file hash must be free again");
    assert_eq!((again.inserted, again.duplicates), (8, 0), "every fingerprint must be free again");
}

#[test]
fn deleting_a_statement_drops_it_from_recent_imports_and_bad_checksums() {
    let mut st = fixture("business-2026-06.txt");
    st.closing_cents = Some(208_800); // off by 32 cents
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
    s.upsert_account(BUSINESS_IBAN, AccountKind::Business, "Firemný").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    let bad = s.import_statement(&st, "h2").unwrap().statement_id;
    assert_eq!(s.statements_with_bad_checksum().unwrap().len(), 1);
    assert_eq!(s.recent_statements(10).unwrap().len(), 2);

    s.delete_statement(bad).unwrap();

    assert!(s.statements_with_bad_checksum().unwrap().is_empty(), "the checksum banner must lose the deleted statement");
    let recent = s.recent_statements(10).unwrap();
    assert_eq!(recent.len(), 1, "only the surviving statement is listed");
    assert_eq!(recent[0].checksum, Checksum::Ok);
}

/// Summaries are computed from the surviving rows, so this checks the real
/// numbers rather than trusting the query: the whole-database summary after
/// the delete must equal the business-only summary from before it.
#[test]
fn deleting_a_statement_leaves_the_surviving_summary_correct() {
    let mut s = loaded();
    let business_id = s.list_accounts().unwrap().into_iter().find(|a| a.kind == AccountKind::Business).unwrap().id;
    let expected = s.summary(None, None, Some(business_id)).unwrap();
    let personal = s.recent_statements(10).unwrap().into_iter().find(|r| r.account_label == "Osobný").unwrap().statement_id;

    s.delete_statement(personal).unwrap();

    let after = s.summary(None, None, None).unwrap();
    assert_eq!(after.income_cents, expected.income_cents);
    assert_eq!(after.expense_cents, expected.expense_cents);
    assert_eq!(after.transfer_cents, expected.transfer_cents);
    assert_eq!(after.net_cents, expected.net_cents);
    assert_eq!(after.unassigned_count, expected.unassigned_count);
    assert_eq!(after.suggested_count, expected.suggested_count);
    assert_eq!(after.by_month, expected.by_month);
    assert_eq!(after.by_category, expected.by_category);
}

/// Provenance in one picture: both statements taught the same merchant rule,
/// each taught its own exact rule. Deleting one statement may take only what
/// that statement alone produced.
#[test]
fn deleting_a_statement_removes_only_the_rules_it_alone_produced() {
    let (mut s, july, august) = two_bauhaus_statements();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    s.assign(&[row_of(&s, july).id], cat, false).unwrap();
    s.assign(&[row_of(&s, august).id], cat, false).unwrap();
    assert!(rule_exists(&s, RuleKind::Exact, "bauhaus", Some("neuss")));
    assert!(rule_exists(&s, RuleKind::Exact, "bauhaus", Some("erfttal")));

    let outcome = s.delete_statement(july).unwrap();

    assert_eq!(outcome.rules_deleted, 1, "only the exact rule of the deleted statement goes");
    assert!(!rule_exists(&s, RuleKind::Exact, "bauhaus", Some("neuss")), "the rule only the deleted statement produced is gone");
    assert!(rule_exists(&s, RuleKind::Exact, "bauhaus", Some("erfttal")), "the surviving statement's own rule stays");
    assert!(rule_exists(&s, RuleKind::Merchant, "bauhaus", None), "a rule the surviving statement also produced stays");
    let survivor = row_of(&s, august);
    assert_eq!((survivor.status, survivor.category_id), (Status::Confirmed, Some(cat)), "a confirmed category on a surviving row is never touched");
}

/// The other half of the rule: the merchant rule here was produced by the
/// deleted statement ALONE, but a surviving row is still classified by it, so
/// it must stay. Narrowing the check to provenance only would delete it and
/// this test would fail.
#[test]
fn a_rule_a_surviving_row_still_uses_is_kept_with_that_rows_confirmed_category() {
    let (mut s, july, august) = two_bauhaus_statements();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    s.assign(&[row_of(&s, july).id], cat, true).unwrap();
    let swept = row_of(&s, august);
    assert_eq!((swept.status, swept.category_id), (Status::Confirmed, Some(cat)), "apply_to_matching confirmed the August row");

    let outcome = s.delete_statement(july).unwrap();

    assert!(rule_exists(&s, RuleKind::Merchant, "bauhaus", None), "a surviving row still uses this rule");
    assert!(!rule_exists(&s, RuleKind::Exact, "bauhaus", Some("neuss")), "nothing survives that used the exact rule");
    assert_eq!(outcome.rules_deleted, 1);
    let survivor = row_of(&s, august);
    assert_eq!((survivor.status, survivor.category_id), (Status::Confirmed, Some(cat)));
}

#[test]
fn deleting_a_statement_never_removes_a_seed_rule() {
    let mut s = loaded();
    let seeds_before = s.list_rules().unwrap().iter().filter(|r| r.kind == RuleKind::Seed).count();
    let personal = s.recent_statements(10).unwrap().into_iter().find(|r| r.account_label == "Osobný").unwrap().statement_id;

    let outcome = s.delete_statement(personal).unwrap();

    assert_eq!(outcome.rules_deleted, 0, "an import that learned nothing deletes no rule");
    let seeds_after = s.list_rules().unwrap().iter().filter(|r| r.kind == RuleKind::Seed).count();
    assert_eq!(seeds_after, seeds_before, "seed rules are never deleted, even when only the deleted rows used them");
    assert!(rule_exists(&s, RuleKind::Seed, "aldi", None));
}

#[test]
fn deleting_a_statement_leaves_accounts_categories_settings_and_the_net_log_alone() {
    let mut s = loaded();
    s.set_check_updates(true).unwrap();
    s.append_net_log(chrono::Utc::now(), "https://api.github.com/x", "200", 5, 2).unwrap();
    let accounts_before = s.list_accounts().unwrap();
    let categories_before = s.list_categories().unwrap();
    let personal = s.recent_statements(10).unwrap().into_iter().find(|r| r.account_label == "Osobný").unwrap().statement_id;

    s.delete_statement(personal).unwrap();

    assert_eq!(s.list_accounts().unwrap(), accounts_before);
    assert_eq!(s.list_categories().unwrap(), categories_before);
    assert!(s.get_check_updates().unwrap(), "settings are not part of the cascade");
    assert_eq!(s.net_log(10).unwrap().len(), 1, "the network audit log is not part of the cascade");
}

/// `hit_count` is written by `touch_rule` on every classification pass, so a
/// delete plus its reclassification round leaves it inflated unless the count
/// is rebuilt from the rows that really point at each rule, AFTER the
/// reclassification.
#[test]
fn hit_counts_are_recomputed_from_the_surviving_rows() {
    let (mut s, july, _august) = two_bauhaus_statements();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    s.assign(&[row_of(&s, july).id], cat, true).unwrap();

    s.delete_statement(july).unwrap();

    assert_eq!(hit_count(&s, RuleKind::Merchant, "bauhaus"), 1, "exactly the one surviving row that points at it");
}

#[test]
fn hit_count_of_a_seed_rule_drops_to_zero_when_its_only_rows_are_deleted() {
    let mut s = loaded();
    let personal = s.recent_statements(10).unwrap().into_iter().find(|r| r.account_label == "Osobný").unwrap().statement_id;
    assert!(hit_count(&s, RuleKind::Seed, "aldi") > 0, "the ALDI row was classified by the seed rule");

    s.delete_statement(personal).unwrap();

    assert_eq!(hit_count(&s, RuleKind::Seed, "aldi"), 0, "no surviving row uses it any more");
}

/// The confirmation text must not promise one thing and do another: the
/// preview and the delete run the same rule query.
#[test]
fn the_preview_promises_exactly_what_the_delete_removes() {
    let (mut s, july, august) = two_bauhaus_statements();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    s.assign(&[row_of(&s, july).id], cat, false).unwrap();
    s.assign(&[row_of(&s, august).id], cat, false).unwrap();

    let preview = s.statement_delete_preview(july).unwrap();
    assert_eq!(preview.number, 7);
    assert_eq!(preview.account_label, "Osobný");
    assert_eq!(preview.transaction_count, 1);
    assert_eq!(preview.confirmed_count, 1);
    assert_eq!(preview.period_end, chrono::NaiveDate::from_ymd_opt(2026, 7, 31).unwrap());

    let outcome = s.delete_statement(july).unwrap();
    assert_eq!(preview.rules_deleted as usize, outcome.rules_deleted);
    assert_eq!(preview.transaction_count as usize, outcome.transactions_deleted);
}

#[test]
fn the_preview_of_a_plain_import_promises_no_rule_loss() {
    let s = loaded();
    let personal = s.recent_statements(10).unwrap().into_iter().find(|r| r.account_label == "Osobný").unwrap().statement_id;
    let preview = s.statement_delete_preview(personal).unwrap();
    assert_eq!((preview.transaction_count, preview.confirmed_count, preview.rules_deleted), (8, 0, 0));
}

#[test]
fn deleting_an_unknown_statement_is_an_error_and_changes_nothing() {
    let mut s = loaded();
    let before = rows(&s);

    let e = s.delete_statement(9_999).unwrap_err();

    assert!(matches!(e, StoreError::UnknownStatement { id: 9_999 }), "got {e:?}");
    assert_eq!(rows(&s), before);
    assert_eq!(s.statement_count().unwrap(), 2);
}

#[test]
fn deleting_one_statement_leaves_another_statements_rows_untouched() {
    let mut s = loaded();
    let business_id = s.list_accounts().unwrap().into_iter().find(|a| a.kind == AccountKind::Business).unwrap().id;
    let before = s.list_transactions(&TxFilter { account_id: Some(business_id), ..Default::default() }).unwrap();
    let personal = s.recent_statements(10).unwrap().into_iter().find(|r| r.account_label == "Osobný").unwrap().statement_id;

    s.delete_statement(personal).unwrap();

    let after = s.list_transactions(&TxFilter { account_id: Some(business_id), ..Default::default() }).unwrap();
    assert_eq!(after, before, "another statement's rows keep their status, category and identity");
    assert_eq!(s.statement_count().unwrap(), 1);
}

/// A17/F1 gap closed: `apply_to_matching` sweeps a merchant rule across a row
/// of ANOTHER statement, and that swept row now records its own provenance.
/// Without it, deleting the teaching statement first correctly kept the rule
/// (a surviving row still used it), but the swept statement's own delete
/// later had no provenance to find: the rule outlived every statement with
/// any claim on it.
#[test]
fn a_swept_row_teaches_its_own_statement_the_rule_so_it_dies_with_the_last_one() {
    let (mut s, july, august) = two_bauhaus_statements();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    s.assign(&[row_of(&s, july).id], cat, true).unwrap();
    assert!(rule_exists(&s, RuleKind::Merchant, "bauhaus", None));

    let first = s.delete_statement(july).unwrap();
    assert_eq!(first.rules_deleted, 1, "only july's own exact rule; the merchant rule is still used by august");
    assert!(rule_exists(&s, RuleKind::Merchant, "bauhaus", None), "the surviving row still needs it");

    let second = s.delete_statement(august).unwrap();

    assert_eq!(second.rules_deleted, 1, "the merchant rule dies with the last statement that has any claim on it");
    assert!(!rule_exists(&s, RuleKind::Merchant, "bauhaus", None), "no statement is left to teach or use it");
}

/// A17/F1 seed guard: `match_kind <> 'seed'` in the doomed-rules query is the
/// only thing standing between a seed rule and deletion once it has a
/// `rule_sources` row. Seeds never get one through normal use (this test
/// gives one directly through the public API), so this is the only test that
/// exercises the filter itself rather than the fact seeds are normally
/// provenance-free.
#[test]
fn a_seed_rule_with_a_provenance_row_still_survives_a_delete() {
    let mut s = loaded();
    let personal = s.recent_statements(10).unwrap().into_iter().find(|r| r.account_label == "Osobný").unwrap().statement_id;
    let seed = s.list_rules().unwrap().into_iter().find(|r| r.kind == RuleKind::Seed).unwrap();
    let some_row = row_of(&s, personal);
    s.record_rule_source(seed.id, some_row.id).unwrap();

    let outcome = s.delete_statement(personal).unwrap();

    assert_eq!(outcome.rules_deleted, 0, "the seed filter must protect it even with a rule_sources row");
    assert!(rule_exists(&s, RuleKind::Seed, &seed.key, seed.place.as_deref()));
}
