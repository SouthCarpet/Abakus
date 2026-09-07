//! 078 audit: deleting one account with its statements, transactions and the
//! rules only it produced, without touching a retained account's rows,
//! including a historical transfer between the two. Same shape as
//! `delete_statement.rs`'s tests, scaled to account granularity. The
//! mid-transaction rollback test needs raw SQL against the private
//! connection (same reason `accounts.rs`'s own rollback test does) and lives
//! as an inline unit test in `src/delete_account.rs` instead of here.
use parser::{parse_text, AccountKind};
use rules::{RuleKind, Status};
use store::recurring::{Cadence, RecurringDecisionInput, RecurringQuery, RecurringSelection, SaveRecurringRequest};
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

fn rows(s: &Store) -> Vec<store::TxRow> { s.list_transactions(&TxFilter::default()).unwrap() }
fn account_rows(s: &Store, account_id: i64) -> Vec<store::TxRow> { s.list_transactions(&TxFilter { account_id: Some(account_id), ..Default::default() }).unwrap() }
fn rule_exists(s: &Store, kind: RuleKind, key: &str, place: Option<&str>) -> bool {
    s.list_rules().unwrap().iter().any(|r| r.kind == kind && r.key == key && r.place.as_deref() == place)
}
fn account_id(s: &Store, iban: &str) -> i64 { s.list_accounts().unwrap().into_iter().find(|a| a.iban == iban).unwrap().id }

/// A personal-account statement with one BAUHAUS card purchase, so two
/// statements (or two accounts) can share a merchant and differ only in
/// place. Mirrors `delete_statement.rs`'s `two_bauhaus_statements` builder.
fn personal_bauhaus(number: u32, date: &str, day: &str, short_day: &str, place: &str) -> parser::Statement {
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
              Miesto platby:    {place}                 BAUHAUS
              Dátum:  {short_day}  Čas:  12:00:00        Suma:          20.00- EUR
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                                       480.00
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        {number}        Strana:        1
");
    parse_text(&text).unwrap()
}

/// Same merchant on the business account, in a different place.
fn business_bauhaus(number: u32, date: &str, day: &str, short_day: &str, place: &str) -> parser::Statement {
    let text = format!(
"Podnikateľský účet   SK37 1100 0000 0000 9876 5432       Mena  EUR                       BIC (SWIFT)   TATRSKBX
IBAN SK37 1100 0000 0000 9876 5432
Číslo klienta:  7654321
Majiteľ účtu:   JANA VZOROVÁ
Tatra banka, a.s., Hodžovo nám. 3
811 06 Bratislava
CENTRÁLA/HEAD OFFICE              ID:   00                                    Výpis číslo:        {number}
Podnikateľský účet   SK37 1100 0000 0000 9876 5432   Majiteľ Jana Vzorová              Dátum {date}
Dátum sprac.  Popis                                     Dátum zúčt.                              Suma
--------------------------------------------------------------------------------------------------
              Posledný výpis  30.06.2026                                                     2,000.00
{day}    EUR AP nákup POS                                                                  30.00-
              Miesto platby:    {place}                 BAUHAUS
              Dátum:  {short_day}  Čas:  12:00:00        Suma:          30.00- EUR
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                                   1,970.00
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        {number}        Strana:        1
");
    parse_text(&text).unwrap()
}

fn two_personal_bauhaus_statements() -> (Store, i64) {
    let mut s = Store::open_in_memory().unwrap();
    let id = s.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
    s.import_statement(&personal_bauhaus(7, "31.07.2026", "01.07.2026", "01.07.26", "Neuss"), "h-july").unwrap();
    s.import_statement(&personal_bauhaus(8, "31.08.2026", "01.08.2026", "01.08.26", "Erfttal"), "h-august").unwrap();
    (s, id)
}

// --- 1: unknown id -----------------------------------------------------------

#[test]
fn deleting_an_unknown_account_is_an_error_and_changes_nothing() {
    let mut s = loaded();
    let before = rows(&s);

    let e = s.delete_account(9_999).unwrap_err();

    assert!(matches!(e, StoreError::UnknownAccountId { id: 9_999 }), "got {e:?}");
    assert_eq!(rows(&s), before);
    assert_eq!(s.list_accounts().unwrap().len(), 2);
}

#[test]
fn previewing_an_unknown_account_is_the_same_error() {
    let s = loaded();
    let e = s.account_delete_preview(9_999).unwrap_err();
    assert!(matches!(e, StoreError::UnknownAccountId { id: 9_999 }), "got {e:?}");
}

// --- 2/3: preview never mutates, and matches the delete ----------------------

#[test]
fn preview_never_writes_to_the_database() {
    let s = loaded();
    let id = account_id(&s, PERSONAL_IBAN);
    let before_accounts = s.list_accounts().unwrap();
    let before_rows = rows(&s);

    let preview = s.account_delete_preview(id).unwrap();

    assert_eq!(preview.account_id, id);
    assert_eq!(preview.label, "Osobný");
    assert_eq!(preview.statement_count, 1);
    assert_eq!(preview.transaction_count, 8);
    assert!(!preview.has_password);
    assert_eq!(s.list_accounts().unwrap(), before_accounts, "a preview must not mutate accounts");
    assert_eq!(rows(&s), before_rows, "a preview must not mutate transactions");
}

#[test]
fn the_preview_promises_exactly_what_the_delete_removes() {
    let mut s = loaded();
    let id = account_id(&s, PERSONAL_IBAN);

    let preview = s.account_delete_preview(id).unwrap();
    let outcome = s.delete_account(id).unwrap();

    assert_eq!(preview.statement_count as usize, outcome.statements_deleted);
    assert_eq!(preview.transaction_count as usize, outcome.transactions_deleted);
    assert_eq!(preview.rules_deleted as usize, outcome.rules_deleted);
}

// --- 4: the account, its statements and transactions are gone ---------------

#[test]
fn deleting_an_account_removes_its_own_statements_and_transactions() {
    let mut s = loaded();
    let id = account_id(&s, PERSONAL_IBAN);

    let outcome = s.delete_account(id).unwrap();

    assert_eq!(outcome.statements_deleted, 1);
    assert_eq!(outcome.transactions_deleted, 8);
    assert!(s.account_by_id(id).unwrap().is_none());
    assert_eq!(account_rows(&s, id), Vec::new());
    assert_eq!(s.statement_count().unwrap(), 1, "only the business statement is left");
}

/// The point of the whole feature, mirrored from the statement-delete test:
/// once the account is gone (and re-added), the same PDF can be re-imported
/// (file_hash and every transaction fingerprint must be free again).
#[test]
fn deleting_an_account_releases_its_file_hash_and_fingerprints() {
    let mut s = loaded();
    let id = account_id(&s, PERSONAL_IBAN);

    s.delete_account(id).unwrap();
    s.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
    let again = s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();

    assert!(!again.already_imported);
    assert_eq!((again.inserted, again.duplicates), (8, 0));
}

// --- 5/6/7: rule provenance --------------------------------------------------

/// A single `assign` call unconditionally teaches BOTH an exact rule and a
/// merchant rule (`apply_to_matching` only controls whether OTHER rows get
/// swept, per `assign.rs::assign_one`), so one assignment with no other
/// account or statement ever touching this merchant leaves both rules with
/// provenance from, and use by, this account alone.
#[test]
fn deleting_an_account_removes_only_the_rules_it_alone_produced() {
    let mut s = loaded();
    let id = account_id(&s, PERSONAL_IBAN);
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    let aldi = account_rows(&s, id).into_iter().find(|r| r.merchant_raw.contains("ALDI")).unwrap();
    s.assign(&[aldi.id], cat, false).unwrap();
    assert!(rule_exists(&s, RuleKind::Exact, "aldi sued", Some("neuss")));
    assert!(rule_exists(&s, RuleKind::Merchant, "aldi sued", None));

    let outcome = s.delete_account(id).unwrap();

    assert_eq!(outcome.rules_deleted, 2, "both the exact and the merchant rule this account alone produced and used");
    assert!(!rule_exists(&s, RuleKind::Exact, "aldi sued", Some("neuss")));
    assert!(!rule_exists(&s, RuleKind::Merchant, "aldi sued", None));
}

/// A rule two of the SAME account's own statements both produced must still
/// go when that account is deleted: this is exactly the case the
/// account-scoped query (as opposed to looping the statement-scoped one) has
/// to get right, since neither statement alone would look "doomed" against
/// the other.
#[test]
fn deleting_an_account_removes_a_rule_two_of_its_own_statements_both_produced() {
    let (mut s, id) = two_personal_bauhaus_statements();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    let newest = rows(&s)[0].id; // apply_to_matching sweeps the other BAUHAUS row too
    s.assign(&[newest], cat, true).unwrap();
    assert!(rule_exists(&s, RuleKind::Merchant, "bauhaus", None), "both rows now point at the shared merchant rule");

    let outcome = s.delete_account(id).unwrap();

    assert!(!rule_exists(&s, RuleKind::Merchant, "bauhaus", None), "no statement of any account is left to teach or use it");
    assert!(!rule_exists(&s, RuleKind::Exact, "bauhaus", Some("neuss")));
    assert!(!rule_exists(&s, RuleKind::Exact, "bauhaus", Some("erfttal")));
    assert_eq!(outcome.rules_deleted, 2, "the exact rule from the assigned row plus the shared merchant rule");
}

/// The other half: a rule produced only by the deleted account, but still
/// pointed at by a RETAINED account's transaction, must survive.
#[test]
fn a_rule_still_used_by_a_retained_accounts_transaction_survives() {
    let mut s = Store::open_in_memory().unwrap();
    let personal_id = s.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
    s.upsert_account(BUSINESS_IBAN, AccountKind::Business, "Firemný").unwrap();
    s.import_statement(&personal_bauhaus(7, "31.07.2026", "01.07.2026", "01.07.26", "Neuss"), "h-personal").unwrap();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    let personal_row = account_rows(&s, personal_id).into_iter().next().unwrap();
    s.assign(&[personal_row.id], cat, true).unwrap(); // teaches the merchant rule from the personal account
    assert!(rule_exists(&s, RuleKind::Merchant, "bauhaus", None));
    // Imported AFTER the rule exists, so classify_ids picks it up on import: a
    // genuinely "used by a retained account" row, reached through the public
    // import/classify path, not raw SQL.
    s.import_statement(&business_bauhaus(6, "31.07.2026", "02.07.2026", "02.07.26", "Erfttal"), "h-business").unwrap();
    let business_row = account_rows(&s, account_id(&s, BUSINESS_IBAN)).into_iter().next().unwrap();
    assert_eq!(business_row.status, Status::Suggested, "classified via the merchant rule on import");

    let outcome = s.delete_account(personal_id).unwrap();

    assert!(rule_exists(&s, RuleKind::Merchant, "bauhaus", None), "a retained account's transaction still uses it");
    assert!(!rule_exists(&s, RuleKind::Exact, "bauhaus", Some("neuss")), "nothing outside the deleted account ever used the exact rule, so it goes");
    assert_eq!(outcome.rules_deleted, 1, "only the exact rule; the merchant rule survives because the business row still points at it");
}

#[test]
fn deleting_an_account_never_removes_a_seed_rule() {
    let mut s = loaded();
    let id = account_id(&s, PERSONAL_IBAN);
    let seeds_before = s.list_rules().unwrap().iter().filter(|r| r.kind == RuleKind::Seed).count();

    s.delete_account(id).unwrap();

    let seeds_after = s.list_rules().unwrap().iter().filter(|r| r.kind == RuleKind::Seed).count();
    assert_eq!(seeds_after, seeds_before);
    assert!(rule_exists(&s, RuleKind::Seed, "aldi", None));
}

// --- 9/10: retained account rows, including a historical transfer -----------

#[test]
fn deleting_an_account_leaves_a_retained_accounts_confirmed_rows_alone() {
    let mut s = loaded();
    let personal_id = account_id(&s, PERSONAL_IBAN);
    let business_id = account_id(&s, BUSINESS_IBAN);
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    let business_row = account_rows(&s, business_id).into_iter().next().unwrap();
    s.assign(&[business_row.id], cat, false).unwrap();

    s.delete_account(personal_id).unwrap();

    let after = account_rows(&s, business_id).into_iter().find(|r| r.id == business_row.id).unwrap();
    assert_eq!((after.status, after.category_id), (Status::Confirmed, Some(cat)), "a retained account's confirmed row is never touched by another account's delete");
}

/// The personal fixture's `TPP 1100/000000-0098765432` standing order pays
/// the business IBAN, so once both accounts exist it classifies as a
/// transfer (own-account match beats every other rule). Deleting the
/// BUSINESS account must not turn that historical transfer, still sitting on
/// the retained personal account, into income or expense: the row's
/// `counterparty_iban` simply stops matching any CURRENT own account, and
/// `delete_account` never re-evaluates an already-`transfer` row (only
/// `reclassify_open` runs, which is scoped to `suggested`/`unassigned`).
#[test]
fn deleting_an_account_never_turns_a_retained_transfer_into_income_or_expense() {
    let mut s = loaded();
    let personal_id = account_id(&s, PERSONAL_IBAN);
    let business_id = account_id(&s, BUSINESS_IBAN);
    let transfer_before = account_rows(&s, personal_id)
        .into_iter()
        .find(|r| r.counterparty_iban.as_deref() == Some(BUSINESS_IBAN))
        .expect("the personal fixture has a standing order paying the business account");
    assert_eq!(transfer_before.status, Status::Transfer, "both accounts exist, so this is a transfer before the delete");

    s.delete_account(business_id).unwrap();

    let after = account_rows(&s, personal_id).into_iter().find(|r| r.id == transfer_before.id).unwrap();
    assert_eq!(after.status, Status::Transfer, "a historical transfer must stay a transfer after its counterparty account is gone");
    assert_eq!(after.category_id, None, "a transfer never carries an income/expense category");
    assert_eq!(after.amount_cents, transfer_before.amount_cents, "the row itself is untouched, only its account context changed");
}

/// The business fixture also carries a transfer in the other direction
/// (`Platba 1100/000000-0012345678` pays the personal IBAN): deleting the
/// PERSONAL account must equally leave that retained business-side transfer
/// alone.
#[test]
fn deleting_an_account_never_turns_the_other_directions_retained_transfer_into_income_or_expense() {
    let mut s = loaded();
    let personal_id = account_id(&s, PERSONAL_IBAN);
    let business_id = account_id(&s, BUSINESS_IBAN);
    let transfer_before = account_rows(&s, business_id)
        .into_iter()
        .find(|r| r.counterparty_iban.as_deref() == Some(PERSONAL_IBAN))
        .expect("the business fixture pays the personal account");
    assert_eq!(transfer_before.status, Status::Transfer);

    s.delete_account(personal_id).unwrap();

    let after = account_rows(&s, business_id).into_iter().find(|r| r.id == transfer_before.id).unwrap();
    assert_eq!(after.status, Status::Transfer);
    assert_eq!(after.category_id, None);
}

// --- 11: open-row reclassification never reaches confirmed/transfer rows ----

#[test]
fn reclassify_open_never_touches_a_retained_accounts_confirmed_or_transfer_rows() {
    let mut s = loaded();
    let personal_id = account_id(&s, PERSONAL_IBAN);
    let business_id = account_id(&s, BUSINESS_IBAN);
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    let business_row = account_rows(&s, business_id).into_iter().next().unwrap();
    s.assign(&[business_row.id], cat, false).unwrap();
    let transfer_before = account_rows(&s, business_id).into_iter().find(|r| r.status == Status::Transfer).expect("the business account has a transfer row (the payment to the personal account)");

    s.delete_account(personal_id).unwrap();

    let confirmed_after = account_rows(&s, business_id).into_iter().find(|r| r.id == business_row.id).unwrap();
    assert_eq!((confirmed_after.status, confirmed_after.category_id), (Status::Confirmed, Some(cat)));
    let transfer_after = account_rows(&s, business_id).into_iter().find(|r| r.id == transfer_before.id).unwrap();
    assert_eq!(transfer_after.status, Status::Transfer);
}

// --- 12: recurring decisions cascade with their own account only --------

/// recurring-contract.md §8: "Account deletion cascades only that
/// account's decisions and selection members." A confirmed recurring
/// decision on the deleted account must go with it; the retained account's
/// own confirmed decision must survive byte-for-byte.
#[test]
fn deleting_an_account_cascades_only_its_own_recurring_decisions() {
    let mut s = loaded();
    let personal_id = account_id(&s, PERSONAL_IBAN);
    let business_id = account_id(&s, BUSINESS_IBAN);
    let personal_tx = account_rows(&s, personal_id).into_iter().next().unwrap().id;
    let business_tx = account_rows(&s, business_id).into_iter().next().unwrap().id;
    let anchor = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
    let doomed = s.save_recurring(&SaveRecurringRequest { decision_id: None, selection: RecurringSelection::Group { transaction_id: personal_tx }, decision: RecurringDecisionInput::Confirmed { cadence: Cadence::Monthly, anchor_date: anchor } }).unwrap();
    let kept = s.save_recurring(&SaveRecurringRequest { decision_id: None, selection: RecurringSelection::Group { transaction_id: business_tx }, decision: RecurringDecisionInput::Confirmed { cadence: Cadence::Monthly, anchor_date: anchor } }).unwrap();

    s.delete_account(personal_id).unwrap();

    let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
    let overview = s.recurring_overview(&RecurringQuery { from: None, to: None, account_kind: None, today }).unwrap();
    assert!(!overview.rows.iter().any(|r| r.decision_id == Some(doomed.id)), "the deleted account's own recurring decision must cascade away");
    let kept_row = overview.rows.iter().find(|r| r.decision_id == Some(kept.id)).expect("the retained account's recurring decision must survive");
    assert_eq!(kept_row.account_id, business_id);
}

// A concrete case where reclassification changes a retained row's result
// (evidence disappearing, via the REFUND `refund_lookup` path rather than a
// rule) lives as an inline unit test in `src/delete_account.rs`: it needs
// raw SQL to put a transaction straight into `confirmed` without going
// through `assign` (which would also teach a merchant rule for the same
// merchant, and that rule matching first is exactly what a first attempt at
// this test tripped over).
