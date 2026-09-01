//! A17/F2: what an account edit may and may not do.
use parser::{parse_text, AccountKind};
use rules::Status;
use store::{Store, StoreError, TxFilter};

const PERSONAL_IBAN: &str = "SK4411000000000012345678";
const BUSINESS_IBAN: &str = "SK3711000000000098765432";

fn fixture(name: &str) -> parser::Statement {
    parse_text(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/synthetic/").to_string() + name).unwrap()).unwrap()
}

fn personal_with_one_import() -> (Store, i64) {
    let mut s = Store::open_in_memory().unwrap();
    let id = s.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
    s.import_statement(&fixture("personal-2026-06.txt"), "h1").unwrap();
    (s, id)
}

/// The label lives in one place and every historical view reads it through a
/// join, so a rename shows up everywhere at once. This fails the moment
/// someone denormalizes the label into `statements` or into a row.
#[test]
fn the_label_is_editable_and_every_historical_view_shows_the_new_one() {
    let mut st = fixture("business-2026-06.txt");
    st.closing_cents = Some(208_800); // makes the checksum banner show this statement
    let mut s = Store::open_in_memory().unwrap();
    let id = s.upsert_account(BUSINESS_IBAN, AccountKind::Business, "Firemný").unwrap();
    s.import_statement(&st, "h1").unwrap();

    let updated = s.update_account(id, "Firma s.r.o.", AccountKind::Business, false).unwrap();

    assert_eq!(updated.label, "Firma s.r.o.");
    assert_eq!(s.recent_statements(10).unwrap()[0].account_label, "Firma s.r.o.");
    assert_eq!(s.statements_with_bad_checksum().unwrap()[0].account_label, "Firma s.r.o.");
}

/// The IBAN is the identity of the account: `update_account` does not take one,
/// so no edit can move an account's history onto another IBAN.
#[test]
fn an_edit_never_changes_the_iban_or_creates_a_second_account() {
    let (mut s, id) = personal_with_one_import();

    s.update_account(id, "Iný názov", AccountKind::Personal, false).unwrap();

    let accounts = s.list_accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].iban, PERSONAL_IBAN);
    assert_eq!(accounts[0].id, id);
}

/// The decision taken for A17/F2: a kind change on an account that already has
/// imports is REFUSED, and the refusal carries the numbers that would be
/// recast, until the caller acknowledges it explicitly.
#[test]
fn a_kind_change_is_refused_while_the_account_has_imports() {
    let (mut s, id) = personal_with_one_import();

    let e = s.update_account(id, "Osobný", AccountKind::Business, false).unwrap_err();

    match e {
        StoreError::AccountKindLocked { iban, statements, transactions } => {
            assert_eq!(iban, PERSONAL_IBAN);
            assert_eq!((statements, transactions), (1, 8), "the refusal says exactly what would be recast");
        }
        other => panic!("expected AccountKindLocked, got {other:?}"),
    }
    assert_eq!(s.list_accounts().unwrap()[0].kind, AccountKind::Personal, "a refused change writes nothing");
}

/// D4: which accounts exist decides which rows are own-account transfers, so an
/// accepted kind change runs the same reclassification an added account runs.
/// Without that call the row below stays `confirmed` and this test fails.
#[test]
fn an_acknowledged_kind_change_recasts_the_account_and_reclassifies() {
    let (mut s, id) = personal_with_one_import();
    s.upsert_account(BUSINESS_IBAN, AccountKind::Business, "Firemný").unwrap();
    let tpp = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.counterparty_iban.as_deref() == Some(BUSINESS_IBAN)).unwrap();
    let cat = s.category_by_path("Nákupy/domácnosť").unwrap().unwrap();
    s.assign(&[tpp.id], cat, false).unwrap();
    assert_eq!(s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == tpp.id).unwrap().status, Status::Confirmed);

    let updated = s.update_account(id, "Osobný", AccountKind::Business, true).unwrap();

    assert_eq!(updated.kind, AccountKind::Business);
    let after = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == tpp.id).unwrap();
    assert_eq!((after.status, after.category_id), (Status::Transfer, None), "the reclassification path must run after an accepted kind change");
}

#[test]
fn a_kind_change_needs_no_acknowledgement_while_the_account_has_no_history() {
    let mut s = Store::open_in_memory().unwrap();
    let id = s.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();

    let updated = s.update_account(id, "Osobný", AccountKind::Business, false).unwrap();

    assert_eq!(updated.kind, AccountKind::Business, "there is no history to recast yet");
}

/// The save path used by the settings screen goes through the same guard, so
/// a kind cannot be recast by resaving an account either.
#[test]
fn saving_an_existing_account_never_recasts_its_kind_silently() {
    let (mut s, _) = personal_with_one_import();

    let e = s.upsert_account(PERSONAL_IBAN, AccountKind::Business, "Osobný").unwrap_err();

    assert!(matches!(e, StoreError::AccountKindLocked { .. }), "got {e:?}");
    assert_eq!(s.list_accounts().unwrap()[0].kind, AccountKind::Personal);
}

#[test]
fn saving_an_existing_account_still_updates_its_label() {
    let (mut s, id) = personal_with_one_import();

    let same = s.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný účet").unwrap();

    assert_eq!(same, id, "the IBAN identifies the account, a resave never creates a second one");
    assert_eq!(s.list_accounts().unwrap()[0].label, "Osobný účet");
}

#[test]
fn editing_an_unknown_account_is_an_error() {
    let mut s = Store::open_in_memory().unwrap();
    let e = s.update_account(42, "x", AccountKind::Personal, false).unwrap_err();
    assert!(matches!(e, StoreError::UnknownAccountId { id: 42 }), "got {e:?}");
}
