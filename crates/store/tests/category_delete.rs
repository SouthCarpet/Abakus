use rules::{RuleKind, Status};
use store::{
    CategoryDeletePreview, CategoryDeleteRequest, CategoryKind, Store, StoreError, TxFilter,
};

fn database() -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("abakus.db");
    let store = Store::open(&path).unwrap();
    (directory, path, store)
}

fn insert_transaction(
    path: &std::path::Path,
    fingerprint: &str,
    category_id: i64,
    status: &str,
    rule_id: Option<i64>,
) -> i64 {
    let connection = rusqlite::Connection::open(path).unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    connection
        .execute_batch(
            "INSERT OR IGNORE INTO accounts (id, iban, kind, label) VALUES (1, 'SK001', 'personal', 'Test');
             INSERT OR IGNORE INTO statements (id, account_id, number, period_start, period_end, checksum_status, file_hash)
             VALUES (1, 1, 1, '2026-01-01', '2026-01-31', 'ok', 'statement');",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO transactions (
                 statement_id, account_id, fingerprint, posted_date, tx_date, kind,
                 amount_cents, merchant_raw, merchant_norm, raw_block, category_id,
                 status, rule_id, source
             ) VALUES (1, 1, ?1, '2026-01-02', '2026-01-02', 'card', -100,
                 ?1, ?1, 'raw', ?2, ?3, ?4, 'manual')",
            rusqlite::params![fingerprint, category_id, status, rule_id],
        )
        .unwrap();
    connection.last_insert_rowid()
}

fn empty_preview(category_id: i64) -> CategoryDeletePreview {
    CategoryDeletePreview {
        category_id,
        affected_categories: Vec::new(),
        transaction_count: 0,
        confirmed_count: 0,
        rule_count: 0,
        rule_source_count: 0,
        recurring_member_count: 0,
    }
}

fn assert_delete_effects(
    store: &Store,
    connection: &rusqlite::Connection,
    category_ids: [i64; 2],
    transaction_ids: [i64; 2],
    rule_id: i64,
) {
    assert!(!store
        .list_categories()
        .unwrap()
        .iter()
        .any(|category| category_ids.contains(&category.id)));
    let rows = store.list_transactions(&TxFilter::default()).unwrap();
    for id in transaction_ids {
        let row = rows.iter().find(|row| row.id == id).unwrap();
        assert_eq!(
            (row.category_id, row.status, row.source.as_str()),
            (None, Status::Unassigned, "none")
        );
        let stored_rule_id: Option<i64> = connection
            .query_row(
                "SELECT rule_id FROM transactions WHERE id = ?1",
                [id],
                |stored| stored.get(0),
            )
            .unwrap();
        assert_eq!(stored_rule_id, None);
    }
    assert!(!store
        .list_rules()
        .unwrap()
        .iter()
        .any(|rule| rule.id == rule_id));
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM rule_sources", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM recurring_members", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        1
    );
}

#[test]
fn deleting_a_category_requires_its_exact_preview_and_moves_confirmed_descendants_to_unassigned() {
    // Oracle: plan 091 point 4 requires an explicit preview, subtree deletion,
    // confirmed-row acknowledgement, safe rule provenance cleanup, and retained
    // recurring membership. Inputs are one root, one child, and two assigned rows.
    let (_directory, path, mut store) = database();
    let root = store
        .save_category(None, None, "Test root", CategoryKind::Expense)
        .unwrap();
    let child = store
        .save_category(None, Some(root.id), "Test child", CategoryKind::Expense)
        .unwrap();
    let rule_id = store
        .insert_rule(RuleKind::Merchant, "child", None, child.id)
        .unwrap();
    let confirmed_id = insert_transaction(&path, "confirmed", root.id, "confirmed", None);
    let suggested_id = insert_transaction(&path, "suggested", child.id, "suggested", Some(rule_id));
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(&format!(
            "INSERT INTO rule_sources (rule_id, transaction_id, statement_id) VALUES ({rule_id}, {suggested_id}, 1);
             INSERT INTO recurring_decisions (id, account_id, group_key, scope, mode, cadence, anchor_date)
             VALUES (1, 1, 'child', 'selected', 'confirmed', 'monthly', '2026-01-02');
             INSERT INTO recurring_members (decision_id, fingerprint) VALUES (1, 'suggested');"
        ))
        .unwrap();

    let preview = store.category_delete_preview(root.id).unwrap();

    assert_eq!(preview.category_id, root.id);
    let affected: Vec<_> = preview
        .affected_categories
        .iter()
        .map(|category| (category.id, category.parent_id, category.name.as_str()))
        .collect();
    assert_eq!(
        affected,
        vec![
            (root.id, None, "Test root"),
            (child.id, Some(root.id), "Test child")
        ]
    );
    assert_eq!(preview.transaction_count, 2);
    assert_eq!(preview.confirmed_count, 1);
    assert_eq!(preview.rule_count, 1);
    assert_eq!(preview.rule_source_count, 1);
    assert_eq!(preview.recurring_member_count, 1);

    let outcome = store
        .delete_category(&CategoryDeleteRequest {
            preview: preview.clone(),
        })
        .unwrap();

    assert_eq!(outcome, preview);
    assert_delete_effects(
        &store,
        &connection,
        [root.id, child.id],
        [confirmed_id, suggested_id],
        rule_id,
    );
}

#[test]
fn deleting_an_archived_empty_category_is_allowed() {
    // Oracle: point 4 includes archived and empty categories. An empty archived
    // category has one affected category and no dependent rows.
    let (_directory, _path, mut store) = database();
    let category = store
        .save_category(None, None, "Old", CategoryKind::Expense)
        .unwrap();
    store.archive_category(category.id).unwrap();

    let preview = store.category_delete_preview(category.id).unwrap();
    assert_eq!(preview.affected_categories.len(), 1);
    assert!(preview.affected_categories[0].archived);
    assert_eq!(
        (
            preview.transaction_count,
            preview.rule_count,
            preview.recurring_member_count
        ),
        (0, 0, 0)
    );

    store
        .delete_category(&CategoryDeleteRequest { preview })
        .unwrap();
    assert!(!store
        .list_categories()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == category.id));
}

#[test]
fn unknown_category_preview_and_delete_are_side_effect_free() {
    // Oracle: point 4 requires unknown IDs to fail. The database starts and
    // ends with the same category count.
    let (_directory, _path, mut store) = database();
    let count_before = store.list_categories().unwrap().len();

    let preview_error = store.category_delete_preview(999_999).unwrap_err();
    let delete_error = store
        .delete_category(&CategoryDeleteRequest {
            preview: empty_preview(999_999),
        })
        .unwrap_err();

    assert!(matches!(
        preview_error,
        StoreError::UnknownCategory { id: 999_999 }
    ));
    assert!(matches!(
        delete_error,
        StoreError::UnknownCategory { id: 999_999 }
    ));
    assert_eq!(store.list_categories().unwrap().len(), count_before);
}

#[test]
fn stale_preview_rejects_a_new_transaction_and_connection_can_retry() {
    // Oracle: apply must not move more rows than the user acknowledged. The
    // preview contains zero rows, then a confirmed row is added before apply.
    let (_directory, path, mut store) = database();
    let category = store
        .save_category(None, None, "Changing", CategoryKind::Expense)
        .unwrap();
    let stale = store.category_delete_preview(category.id).unwrap();
    let transaction_id = insert_transaction(&path, "late", category.id, "confirmed", None);

    let error = store
        .delete_category(&CategoryDeleteRequest { preview: stale })
        .unwrap_err();

    assert!(matches!(error, StoreError::Parse(ref message) if message.contains("náhľadu")));
    assert!(store
        .list_categories()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == category.id));
    let row = store
        .list_transactions(&TxFilter::default())
        .unwrap()
        .into_iter()
        .find(|row| row.id == transaction_id)
        .unwrap();
    assert_eq!(
        (row.category_id, row.status),
        (Some(category.id), Status::Confirmed)
    );

    let current = store.category_delete_preview(category.id).unwrap();
    store
        .delete_category(&CategoryDeleteRequest { preview: current })
        .unwrap();
    assert!(!store
        .list_categories()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == category.id));
}

#[test]
fn system_categories_and_their_protected_children_cannot_be_deleted() {
    // Oracle: system categories and the full system subtree are protected.
    // Hotovosť is system-owned and bankomat is its non-system child.
    let (_directory, _path, mut store) = database();
    let cash_root = store
        .list_categories()
        .unwrap()
        .into_iter()
        .find(|category| category.name == "Hotovosť")
        .unwrap();
    let cash_child_id = store
        .category_by_path("Hotovosť/bankomat")
        .unwrap()
        .unwrap();

    let root_error = store.category_delete_preview(cash_root.id).unwrap_err();
    let child_error = store.category_delete_preview(cash_child_id).unwrap_err();
    let delete_error = store
        .delete_category(&CategoryDeleteRequest {
            preview: empty_preview(cash_child_id),
        })
        .unwrap_err();

    assert!(matches!(root_error, StoreError::Parse(ref message) if message.contains("Systémovú")));
    assert!(matches!(child_error, StoreError::Parse(ref message) if message.contains("chránenej")));
    assert!(
        matches!(delete_error, StoreError::Parse(ref message) if message.contains("chránenej"))
    );
    assert!(store
        .list_categories()
        .unwrap()
        .iter()
        .any(|category| category.id == cash_root.id));
    assert!(store
        .list_categories()
        .unwrap()
        .iter()
        .any(|category| category.id == cash_child_id));
}

#[test]
fn transfer_with_a_category_reference_stops_deletion_without_mutation() {
    // Oracle: transfers must remain unchanged. A transfer that references the
    // target category is unsafe inconsistent state, so preview and apply fail.
    let (_directory, path, mut store) = database();
    let category = store
        .save_category(None, None, "Unsafe", CategoryKind::Expense)
        .unwrap();
    let transfer_id = insert_transaction(&path, "transfer", category.id, "transfer", None);

    let preview_error = store.category_delete_preview(category.id).unwrap_err();
    let delete_error = store
        .delete_category(&CategoryDeleteRequest {
            preview: empty_preview(category.id),
        })
        .unwrap_err();

    assert!(matches!(preview_error, StoreError::Parse(ref message) if message.contains("prevod")));
    assert!(matches!(delete_error, StoreError::Parse(ref message) if message.contains("prevod")));
    let row = store
        .list_transactions(&TxFilter::default())
        .unwrap()
        .into_iter()
        .find(|row| row.id == transfer_id)
        .unwrap();
    assert_eq!(
        (row.category_id, row.status),
        (Some(category.id), Status::Transfer)
    );
    assert!(store
        .list_categories()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == category.id));
}

#[test]
fn rule_reference_outside_the_subtree_stops_deletion() {
    // Oracle: deleting a category must not silently detach or reclassify a row
    // outside the acknowledged subtree, even if its rule targets the subtree.
    let (_directory, path, mut store) = database();
    let target = store
        .save_category(None, None, "Target", CategoryKind::Expense)
        .unwrap();
    let outside = store
        .save_category(None, None, "Outside", CategoryKind::Expense)
        .unwrap();
    let rule_id = store
        .insert_rule(RuleKind::Merchant, "outside", None, target.id)
        .unwrap();
    let transaction_id =
        insert_transaction(&path, "outside", outside.id, "suggested", Some(rule_id));

    let error = store.category_delete_preview(target.id).unwrap_err();

    assert!(
        matches!(error, StoreError::Parse(ref message) if message.contains("mimo odstraňovanej vetvy"))
    );
    assert!(store
        .list_rules()
        .unwrap()
        .iter()
        .any(|rule| rule.id == rule_id));
    let row = store
        .list_transactions(&TxFilter::default())
        .unwrap()
        .into_iter()
        .find(|row| row.id == transaction_id)
        .unwrap();
    assert_eq!(
        (row.category_id, row.status),
        (Some(outside.id), Status::Suggested)
    );
}

#[test]
fn sql_failure_rolls_back_all_related_rows_and_connection_can_retry() {
    // Oracle: point 4 requires atomic deletion and a usable connection after
    // failure. A trigger fails the category DELETE after row and rule updates.
    let (_directory, path, mut store) = database();
    let category = store
        .save_category(None, None, "Rollback", CategoryKind::Expense)
        .unwrap();
    let rule_id = store
        .insert_rule(RuleKind::Merchant, "rollback", None, category.id)
        .unwrap();
    let transaction_id =
        insert_transaction(&path, "rollback", category.id, "confirmed", Some(rule_id));
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(&format!(
            "INSERT INTO rule_sources (rule_id, transaction_id, statement_id) VALUES ({rule_id}, {transaction_id}, 1);
             CREATE TRIGGER fail_category_delete BEFORE DELETE ON categories WHEN OLD.id = {} BEGIN SELECT RAISE(ABORT, 'forced category delete failure'); END;",
            category.id
        ))
        .unwrap();
    let preview = store.category_delete_preview(category.id).unwrap();

    let error = store
        .delete_category(&CategoryDeleteRequest {
            preview: preview.clone(),
        })
        .unwrap_err();

    assert!(matches!(error, StoreError::Db(_)));
    assert!(store
        .list_categories()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == category.id));
    assert!(store
        .list_rules()
        .unwrap()
        .iter()
        .any(|rule| rule.id == rule_id));
    let row = store
        .list_transactions(&TxFilter::default())
        .unwrap()
        .into_iter()
        .find(|row| row.id == transaction_id)
        .unwrap();
    assert_eq!(
        (row.category_id, row.status, row.source.as_str()),
        (Some(category.id), Status::Confirmed, "manual")
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM rule_sources WHERE rule_id = ?1",
                [rule_id],
                |stored| stored.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );

    connection
        .execute_batch("DROP TRIGGER fail_category_delete")
        .unwrap();
    store
        .delete_category(&CategoryDeleteRequest { preview })
        .unwrap();
    assert!(!store
        .list_categories()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == category.id));
}
