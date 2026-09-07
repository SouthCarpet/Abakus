//! Section 9 category workflow acceptance: C01 move/promote, C02 kind-change
//! acknowledgement, C03 name validation and duplicates, C05 seed rule
//! redirect. Fixtures go through the real parser/import/classify path
//! (`import_statement`) wherever the validated API can build the state; the
//! one exception (C03's pre-existing legacy duplicate, which the validated
//! create path now refuses to construct) uses the same file-backed
//! independent-`rusqlite::Connection` pattern already established in
//! `tests/migration.rs`, never `Store` internals.
use parser::{parse_text, AccountKind};
use rules::{RuleKind, Status};
use store::{Category, CategoryKind, CategoryUpdateRequest, Store, StoreError, TxFilter};

const STATEMENT_TEXT: &str = "Osobný účet     SK44 1100 0000 0000 1234 5678          Mena  EUR                          BIC (SWIFT)   TATRSKBX
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
              Miesto platby:    Neuss                 ALDI SUED
              Dátum:  01.07.26  Čas:  12:00:00        Suma:          20.00- EUR
--------------------------------------------------------------------------------------------------
02.07.2026    EUR AP nákup POS                                                                  15.00-
              Miesto platby:    Neuss                 LIDL
              Dátum:  02.07.26  Čas:  12:00:00        Suma:          15.00- EUR
--------------------------------------------------------------------------------------------------
03.07.2026    EUR AP nákup POS                                                                   8.00-
              Miesto platby:    Neuss                 MCDONALDS
              Dátum:  03.07.26  Čas:  12:00:00        Suma:           8.00- EUR
--------------------------------------------------------------------------------------------------
04.07.2026    EUR AP nákup POS                                                                   9.99-
              Miesto platby:    Internet              SPOTIFY AB
              Dátum:  04.07.26  Čas:  12:00:00        Suma:           9.99- EUR
--------------------------------------------------------------------------------------------------
06.07.2026    EUR AP nákup POS                                                                  12.99-
              Miesto platby:    Internet              NETFLIX
              Dátum:  06.07.26  Čas:  12:00:00        Suma:          12.99- EUR
--------------------------------------------------------------------------------------------------
              Zostatok na účte ku dňu vystavenia výpisu:                                       420.03
--------------------------------------------------------------------------------------------------
Mena    EUR                                          Výpis číslo:        7        Strana:        1
";

fn loaded() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
    s.import_statement(&parse_text(STATEMENT_TEXT).unwrap(), "h1").unwrap();
    s
}
fn id_of(s: &Store, merchant: &str) -> i64 { s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.merchant_raw == merchant).unwrap().id }
fn find(cats: &[Category], name: &str) -> Category { cats.iter().find(|c| c.name == name).unwrap().clone() }

// C01: move an active leaf between expense parents, then promote it to root.
#[test]
fn c01_moving_and_promoting_a_leaf_preserves_identity_and_history() {
    let mut s = loaded();
    let cats = s.list_categories().unwrap();
    let auto = find(&cats, "Auto/doprava").id;
    let potraviny = find(&cats, "potraviny").id;
    let aldi = id_of(&s, "ALDI SUED");
    s.confirm(&[aldi]).unwrap();

    let moved = s.update_category(&CategoryUpdateRequest { id: potraviny, parent_id: Some(auto), name: "potraviny".into(), kind: CategoryKind::Expense, acknowledge_kind_change: false }).unwrap();
    assert_eq!(moved.parent_id, Some(auto));
    assert_eq!(moved.kind, CategoryKind::Expense, "parent kind wins");
    let row = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == aldi).unwrap();
    assert_eq!(row.category_id, Some(potraviny), "history/status untouched by a move");
    assert_eq!(row.status, Status::Confirmed);

    let promoted = s.update_category(&CategoryUpdateRequest { id: potraviny, parent_id: None, name: "potraviny".into(), kind: CategoryKind::Expense, acknowledge_kind_change: false }).unwrap();
    assert_eq!(promoted.parent_id, None);
    assert_eq!(promoted.kind, CategoryKind::Expense, "promoting a child to root retains its old kind by default");
}

#[test]
fn c01_a_root_with_children_cannot_be_reparented_under_another_root() {
    let mut s = Store::open_in_memory().unwrap();
    let cats = s.list_categories().unwrap();
    let jedlo = find(&cats, "Jedlo").id;
    let auto = find(&cats, "Auto/doprava").id;

    let e = s.update_category(&CategoryUpdateRequest { id: jedlo, parent_id: Some(auto), name: "Jedlo".into(), kind: CategoryKind::Expense, acknowledge_kind_change: false }).unwrap_err();
    assert!(matches!(e, StoreError::Parse(_)), "got {e:?}");
    assert_eq!(s.list_categories().unwrap().into_iter().find(|c| c.id == jedlo).unwrap().parent_id, None, "refusal must not write");
}

#[test]
fn c01_self_descendant_missing_archived_and_system_parent_targets_are_all_refused() {
    let mut s = Store::open_in_memory().unwrap();
    let cats = s.list_categories().unwrap();
    let jedlo = find(&cats, "Jedlo").id;
    let potraviny = find(&cats, "potraviny").id;
    let hotovost = find(&cats, "Hotovosť").id;
    let faktury = find(&cats, "Faktúry").id;
    s.archive_category(faktury).unwrap();

    let base = CategoryUpdateRequest { id: jedlo, parent_id: None, name: "Jedlo".into(), kind: CategoryKind::Expense, acknowledge_kind_change: false };
    assert!(s.update_category(&CategoryUpdateRequest { parent_id: Some(jedlo), ..base.clone() }).is_err(), "self");
    assert!(s.update_category(&CategoryUpdateRequest { parent_id: Some(potraviny), ..base.clone() }).is_err(), "descendant (non-top-level target)");
    assert!(s.update_category(&CategoryUpdateRequest { parent_id: Some(999_999), ..base.clone() }).is_err(), "missing");
    assert!(s.update_category(&CategoryUpdateRequest { parent_id: Some(faktury), ..base.clone() }).is_err(), "archived");
    assert!(s.update_category(&CategoryUpdateRequest { parent_id: Some(hotovost), ..base }).is_err(), "system");
}

#[test]
fn c01_the_protected_hotovost_subtree_keeps_rename_but_refuses_move_and_kind_change() {
    let mut s = Store::open_in_memory().unwrap();
    let cats = s.list_categories().unwrap();
    let bankomat = find(&cats, "bankomat").id;
    let hotovost = find(&cats, "Hotovosť").id;
    let auto = find(&cats, "Auto/doprava").id;

    let renamed = s.update_category(&CategoryUpdateRequest { id: bankomat, parent_id: Some(hotovost), name: "bankomat SK".into(), kind: CategoryKind::Expense, acknowledge_kind_change: false }).unwrap();
    assert_eq!(renamed.name, "bankomat SK", "rename inside the protected subtree stays permitted");

    let moved = s.update_category(&CategoryUpdateRequest { id: bankomat, parent_id: Some(auto), name: "bankomat SK".into(), kind: CategoryKind::Expense, acknowledge_kind_change: false });
    assert!(moved.is_err(), "moving out of Hotovosť must be refused");
    // "Parent kind wins" already makes a child's own kind field inert while
    // its parent stays the same: requesting Income here is a no-op, not a
    // write that could ever need (or bypass) the protected-subtree gate.
    let kind_request_is_a_noop = s.update_category(&CategoryUpdateRequest { id: bankomat, parent_id: Some(hotovost), name: "bankomat SK".into(), kind: CategoryKind::Income, acknowledge_kind_change: true }).unwrap();
    assert_eq!(kind_request_is_a_noop.kind, CategoryKind::Expense, "a child's kind cannot move away from its unchanged parent's");
}

// C02: kind change on a root with active + archived children and linked transactions.
struct C02Fixture { store: Store, jedlo: i64, aldi: i64, req: CategoryUpdateRequest }

fn c02_fixture() -> C02Fixture {
    let mut s = loaded();
    let cats = s.list_categories().unwrap();
    let jedlo = find(&cats, "Jedlo").id;
    s.archive_category(find(&cats, "objednávky").id).unwrap();
    let aldi = id_of(&s, "ALDI SUED");
    let lidl = id_of(&s, "LIDL");
    s.confirm(&[aldi, lidl]).unwrap();
    let _mcdonalds = id_of(&s, "MCDONALDS"); // left as the natural seed-suggested third row
    let req = CategoryUpdateRequest { id: jedlo, parent_id: None, name: "Jedlo".into(), kind: CategoryKind::Income, acknowledge_kind_change: false };
    C02Fixture { store: s, jedlo, aldi, req }
}

#[test]
fn c02_the_preview_gives_exact_counts_and_an_unacknowledged_save_writes_nothing() {
    let C02Fixture { mut store, jedlo, req, .. } = c02_fixture();

    let preview = store.category_update_preview(&req).unwrap();
    assert_eq!(preview.affected_categories, 4, "Jedlo + potraviny + reštaurácia + objednávky (archived counts too)");
    assert_eq!(preview.transaction_count, 3);
    assert_eq!(preview.confirmed_count, 2);
    assert!(preview.requires_confirmation);

    let no_ack = store.update_category(&req).unwrap_err();
    assert!(matches!(no_ack, StoreError::Parse(_)));
    assert_eq!(store.list_categories().unwrap().into_iter().find(|c| c.id == jedlo).unwrap().kind, CategoryKind::Expense, "no ack must leave everything unchanged");
}

#[test]
fn c02_an_acknowledged_save_propagates_kind_to_every_child_and_leaves_confirmed_transactions_alone() {
    let C02Fixture { mut store, aldi, req, .. } = c02_fixture();

    let acked = store.update_category(&CategoryUpdateRequest { acknowledge_kind_change: true, ..req }).unwrap();

    assert_eq!(acked.kind, CategoryKind::Income);
    let after = store.list_categories().unwrap();
    for name in ["potraviny", "reštaurácia", "objednávky"] { assert_eq!(find(&after, name).kind, CategoryKind::Income, "{name} must follow its root, archived ones included"); }
    let confirmed_row = store.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == aldi).unwrap();
    assert_eq!(confirmed_row.category_id, Some(find(&after, "potraviny").id), "confirmed transaction category id is untouched by a kind change");
}

#[test]
fn c02_a_kind_change_with_no_linked_transactions_needs_no_acknowledgement() {
    let mut s = Store::open_in_memory().unwrap();
    let cats = s.list_categories().unwrap();
    let faktury = find(&cats, "Faktúry").id;

    let updated = s.update_category(&CategoryUpdateRequest { id: faktury, parent_id: None, name: "Faktúry".into(), kind: CategoryKind::Expense, acknowledge_kind_change: false }).unwrap();
    assert_eq!(updated.kind, CategoryKind::Expense);
}

// C03: name validation.
#[test]
fn c03_name_length_and_control_character_bounds() {
    let mut s = Store::open_in_memory().unwrap();
    let cats = s.list_categories().unwrap();
    let jedlo = find(&cats, "Jedlo").id;
    let req = |name: &str| CategoryUpdateRequest { id: jedlo, parent_id: None, name: name.into(), kind: CategoryKind::Expense, acknowledge_kind_change: false };

    assert!(s.update_category(&req("")).is_err(), "empty");
    assert!(s.update_category(&req("   ")).is_err(), "whitespace only");
    assert!(s.update_category(&req("obsahuje\0nul")).is_err(), "NUL");
    assert!(s.update_category(&req("obsahuje\ttab")).is_err(), "control character");
    assert!(s.update_category(&req(&"č".repeat(101))).is_err(), "101 scalar values");
    assert!(s.update_category(&req(&"č".repeat(100))).is_ok(), "exactly 100 succeeds");
    assert!(s.save_category(None, None, "Obsahuje/lomku", CategoryKind::Expense).is_ok(), "slash stays legal");
}

#[test]
fn c03_a_new_duplicate_is_refused_but_an_edit_that_keeps_a_legacy_duplicates_name_and_parent_may_succeed() {
    let mut s = Store::open_in_memory().unwrap();
    let cats = s.list_categories().unwrap();
    let jedlo = find(&cats, "Jedlo").id;
    let zdravie = find(&cats, "Zdravie").id;

    let create = s.save_category(None, Some(zdravie), "lekáreň", CategoryKind::Expense);
    assert!(create.is_err(), "creating a folded duplicate sibling is refused");

    let moved = s.update_category(&CategoryUpdateRequest { id: jedlo, parent_id: None, name: "Zdravie".into(), kind: CategoryKind::Expense, acknowledge_kind_change: false });
    assert!(moved.is_err(), "renaming into an existing root name is refused");

    let unchanged = s.update_category(&CategoryUpdateRequest { id: jedlo, parent_id: None, name: "Jedlo".into(), kind: CategoryKind::Expense, acknowledge_kind_change: false });
    assert!(unchanged.is_ok(), "an edit whose folded name and parent stay unchanged succeeds");
}

/// A genuine pre-existing legacy duplicate (two root categories that fold to
/// the same name) can only exist in a database the current validated create
/// path would never produce, so this builds a real file-backed database with
/// an independent `rusqlite::Connection`, exactly like `tests/migration.rs`,
/// then reopens it through `Store::open`.
#[test]
fn c03_editing_a_legacy_duplicate_without_changing_its_folded_name_or_parent_still_succeeds() {
    let path = std::env::temp_dir().join(format!("abakus-category-legacy-dup-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(include_str!("../src/schema.sql")).unwrap();
    }
    {
        let s = Store::open(&path).unwrap();
        assert!(s.list_categories().unwrap().iter().any(|c| c.name == "Jedlo"), "the normal seed ran on the fresh schema");
    }
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute("INSERT INTO categories (parent_id, name, kind, sort) VALUES (NULL, 'JEDLO', 'expense', 99)", []).unwrap();
    }
    let mut s = Store::open(&path).unwrap();
    let dup_id = s.list_categories().unwrap().into_iter().find(|c| c.name == "JEDLO").unwrap().id;

    // The row keeps its own literal name and parent: allowed even though its
    // folded name duplicates the original "Jedlo" root.
    let unchanged = s.update_category(&CategoryUpdateRequest { id: dup_id, parent_id: None, name: "JEDLO".into(), kind: CategoryKind::Income, acknowledge_kind_change: false }).unwrap();
    assert_eq!((unchanged.name.as_str(), unchanged.kind), ("JEDLO", CategoryKind::Income));

    // A rename to a brand new, non-colliding name is ordinary editing.
    let renamed = s.update_category(&CategoryUpdateRequest { id: dup_id, parent_id: None, name: "JEDLO archívne".into(), kind: CategoryKind::Income, acknowledge_kind_change: false }).unwrap();
    assert_eq!(renamed.name, "JEDLO archívne");

    // But moving/renaming it to collide with a DIFFERENT existing sibling is still refused.
    let e = s.update_category(&CategoryUpdateRequest { id: dup_id, parent_id: None, name: "Zdravie".into(), kind: CategoryKind::Income, acknowledge_kind_change: false }).unwrap_err();
    assert!(matches!(e, StoreError::Parse(_)));

    std::fs::remove_file(&path).ok();
}

// C05: seed rule provenance and redirect.
#[test]
fn c05_seed_rule_pointer_and_redirect_reclassifies_only_open_rows() {
    let mut s = loaded();
    let spotify_open = id_of(&s, "SPOTIFY AB");
    // An unrelated, already-confirmed row (a different merchant entirely, so
    // confirming it cannot learn a rule that shadows the spotify seed rule).
    let aldi_confirmed = id_of(&s, "ALDI SUED");
    s.confirm(&[aldi_confirmed]).unwrap();
    let aldi_category = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == aldi_confirmed).unwrap().category_id.unwrap();
    let netflix = id_of(&s, "NETFLIX");
    let netflix_category = s.list_transactions(&TxFilter::default()).unwrap().into_iter().find(|r| r.id == netflix).unwrap().category_id.unwrap();

    let pointer = s.seed_rule_for_transaction(spotify_open).unwrap().expect("classified by the spotify seed rule");
    assert_eq!(pointer.key, "spotify");

    let outcome = s.update_rule_category(pointer.id, netflix_category).unwrap();
    assert_eq!((outcome.rule_id, outcome.category_id), (pointer.id, netflix_category));

    let rows = s.list_transactions(&TxFilter::default()).unwrap();
    let open_after = rows.iter().find(|r| r.id == spotify_open).unwrap();
    let confirmed_after = rows.iter().find(|r| r.id == aldi_confirmed).unwrap();
    assert_eq!(open_after.category_id, Some(netflix_category), "open row follows the redirected rule");
    assert_eq!((confirmed_after.status, confirmed_after.category_id), (Status::Confirmed, Some(aldi_category)), "an unrelated confirmed row is byte-for-byte unchanged");
}

#[test]
fn c05_a_missing_transaction_is_an_error_not_a_null_pointer() {
    let s = Store::open_in_memory().unwrap();
    let e = s.seed_rule_for_transaction(9_999).unwrap_err();
    assert!(matches!(e, StoreError::UnknownTransaction { id: 9_999 }));
}

#[test]
fn c05_a_learned_rule_pointer_and_transaction_with_no_rule_both_return_none() {
    let mut s = loaded();
    let aldi = id_of(&s, "ALDI SUED");
    let potraviny = s.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    s.assign(&[aldi], potraviny, false).unwrap();
    assert!(s.seed_rule_for_transaction(aldi).unwrap().is_none(), "a learned exact-rule match is not seed provenance");
}

#[test]
fn c05_redirect_is_refused_for_a_root_with_active_children_but_allowed_with_only_archived_ones() {
    let mut s = Store::open_in_memory().unwrap();
    let cats = s.list_categories().unwrap();
    let jedlo = find(&cats, "Jedlo").id;
    let spotify_rule = s.list_rules().unwrap().into_iter().find(|r| r.key == "spotify").unwrap().id;

    assert!(s.update_rule_category(spotify_rule, jedlo).is_err(), "Jedlo has active children");

    let children: Vec<i64> = s.list_categories().unwrap().into_iter().filter(|c| c.parent_id == Some(jedlo)).map(|c| c.id).collect();
    for c in children { s.archive_category(c).unwrap(); }
    assert!(s.update_rule_category(spotify_rule, jedlo).is_ok(), "a root with only archived children is a usable leaf-equivalent target");
}

#[test]
fn c05_redirect_is_limited_to_seed_rules() {
    let mut s = loaded();
    let potraviny = s.category_by_path("Jedlo/potraviny").unwrap().unwrap();
    let restauracia = s.category_by_path("Jedlo/reštaurácia").unwrap().unwrap();
    let aldi = id_of(&s, "ALDI SUED");
    s.assign(&[aldi], potraviny, false).unwrap();
    let learned = s.list_rules().unwrap().into_iter().find(|r| r.kind == RuleKind::Exact).unwrap().id;

    let e = s.update_rule_category(learned, restauracia).unwrap_err();
    assert!(matches!(e, StoreError::Parse(_)));
}
