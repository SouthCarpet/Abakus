//! Section 9 wire drift gate, same shape as `commands_json.rs`: round-trip
//! the exact JSON the UI's `categoryApi` sends against the real Rust types,
//! and serialize the real response structs to check their field names.
use serde_json::json;
use store::{
    Category, CategoryDeleteItem, CategoryDeletePreview, CategoryDeleteRequest, CategoryKind,
    CategoryUpdatePreview, CategoryUpdateRequest, RuleDeletePreview, RuleRedirectOutcome,
};

#[derive(serde::Deserialize)]
struct CategoryUpdatePreviewArgs {
    request: CategoryUpdateRequest,
}

#[derive(serde::Deserialize)]
struct UpdateCategoryArgs {
    request: CategoryUpdateRequest,
}

#[derive(serde::Deserialize)]
struct CategoryDeletePreviewArgs {
    #[serde(rename = "categoryId")]
    category_id: i64,
}

#[derive(serde::Deserialize)]
struct DeleteCategoryArgs {
    request: CategoryDeleteRequest,
}

#[derive(serde::Deserialize)]
struct SeedRuleForTransactionArgs {
    #[serde(rename = "transactionId")]
    transaction_id: i64,
}

#[derive(serde::Deserialize)]
struct UpdateRuleCategoryArgs {
    #[serde(rename = "ruleId")]
    rule_id: i64,
    #[serde(rename = "categoryId")]
    category_id: i64,
}

#[derive(serde::Deserialize)]
struct RuleDeletePreviewArgs {
    #[serde(rename = "ruleId")]
    rule_id: i64,
}

#[test]
fn category_update_preview_args_match_the_ui_call() {
    let v = json!({"request": {"id": 4, "parent_id": 1, "name": "potraviny", "kind": "expense", "acknowledge_kind_change": false}});
    let args: CategoryUpdatePreviewArgs = serde_json::from_value(v).unwrap();
    assert_eq!(
        (
            args.request.id,
            args.request.parent_id,
            args.request.name.as_str(),
            args.request.kind,
            args.request.acknowledge_kind_change
        ),
        (4, Some(1), "potraviny", CategoryKind::Expense, false)
    );
}

#[test]
fn update_category_args_match_the_ui_call_including_a_null_parent() {
    let v = json!({"request": {"id": 4, "parent_id": null, "name": "Jedlo", "kind": "income", "acknowledge_kind_change": true}});
    let args: UpdateCategoryArgs = serde_json::from_value(v).unwrap();
    assert_eq!(
        (
            args.request.id,
            args.request.parent_id,
            args.request.kind,
            args.request.acknowledge_kind_change
        ),
        (4, None, CategoryKind::Income, true)
    );
}

#[test]
fn category_delete_preview_args_match_the_ui_call() {
    let args: CategoryDeletePreviewArgs = serde_json::from_value(json!({"categoryId": 4})).unwrap();
    assert_eq!(args.category_id, 4);
}

#[test]
fn delete_category_args_require_the_exact_preview_shape() {
    let value = json!({"request": {"preview": {
        "category_id": 4,
        "affected_categories": [{"id": 4, "parent_id": null, "name": "Jedlo", "archived": false}],
        "transaction_count": 5,
        "confirmed_count": 2,
        "rule_count": 1,
        "rule_source_count": 3,
        "recurring_member_count": 1
    }}});
    let args: DeleteCategoryArgs = serde_json::from_value(value).unwrap();
    assert_eq!(args.request.preview.category_id, 4);
    assert_eq!(args.request.preview.confirmed_count, 2);
    assert_eq!(args.request.preview.affected_categories[0].name, "Jedlo");
}

#[test]
fn delete_category_request_rejects_missing_or_unknown_preview_fields() {
    let missing = json!({"preview": {"category_id": 4}});
    let unknown = json!({"preview": {
        "category_id": 4,
        "affected_categories": [],
        "transaction_count": 0,
        "confirmed_count": 0,
        "rule_count": 0,
        "rule_source_count": 0,
        "recurring_member_count": 0,
        "extra": true
    }});
    assert!(serde_json::from_value::<CategoryDeleteRequest>(missing).is_err());
    assert!(serde_json::from_value::<CategoryDeleteRequest>(unknown).is_err());
}

/// `CategoryUpdateRequest` rejects unknown fields (§9 wire contract): a stray
/// or misspelled field must fail loudly, not silently pass through.
#[test]
fn category_update_request_rejects_unknown_fields() {
    let v = json!({"id": 4, "parent_id": null, "name": "Jedlo", "kind": "expense", "acknowledge_kind_change": false, "extra": true});
    assert!(serde_json::from_value::<CategoryUpdateRequest>(v).is_err());
}

#[test]
fn seed_rule_for_transaction_args_match_the_ui_call() {
    let args: SeedRuleForTransactionArgs =
        serde_json::from_value(json!({"transactionId": 42})).unwrap();
    assert_eq!(args.transaction_id, 42);
}

#[test]
fn update_rule_category_args_match_the_ui_call() {
    let args: UpdateRuleCategoryArgs =
        serde_json::from_value(json!({"ruleId": 9, "categoryId": 4})).unwrap();
    assert_eq!((args.rule_id, args.category_id), (9, 4));
}

#[test]
fn rule_delete_preview_args_match_the_ui_call() {
    let args: RuleDeletePreviewArgs = serde_json::from_value(json!({"ruleId": 9})).unwrap();
    assert_eq!(args.rule_id, 9);
}

#[test]
fn category_update_preview_serializes_snake_case() {
    let p = CategoryUpdatePreview {
        effective_kind: CategoryKind::Income,
        affected_categories: 3,
        transaction_count: 5,
        confirmed_count: 2,
        requires_confirmation: true,
    };
    let v = serde_json::to_value(p).unwrap();
    assert_eq!(
        v,
        json!({"effective_kind": "income", "affected_categories": 3, "transaction_count": 5, "confirmed_count": 2, "requires_confirmation": true})
    );
}

#[test]
fn category_delete_preview_serializes_every_acknowledged_effect() {
    let preview = CategoryDeletePreview {
        category_id: 4,
        affected_categories: vec![CategoryDeleteItem {
            id: 4,
            parent_id: None,
            name: "Jedlo".into(),
            archived: false,
        }],
        transaction_count: 5,
        confirmed_count: 2,
        rule_count: 1,
        rule_source_count: 3,
        recurring_member_count: 1,
    };
    let value = serde_json::to_value(preview).unwrap();
    assert_eq!(
        value,
        json!({
            "category_id": 4,
            "affected_categories": [{"id": 4, "parent_id": null, "name": "Jedlo", "archived": false}],
            "transaction_count": 5,
            "confirmed_count": 2,
            "rule_count": 1,
            "rule_source_count": 3,
            "recurring_member_count": 1
        })
    );
}

#[test]
fn rule_redirect_outcome_serializes_snake_case() {
    let o = RuleRedirectOutcome {
        rule_id: 9,
        category_id: 4,
        updated: 2,
    };
    let v = serde_json::to_value(o).unwrap();
    assert_eq!(v, json!({"rule_id": 9, "category_id": 4, "updated": 2}));
}

#[test]
fn rule_delete_preview_serializes_exact_counts() {
    let preview = RuleDeletePreview {
        rule_id: 9,
        open_rule_references: 4,
        open_classification_changes: 2,
    };
    let value = serde_json::to_value(preview).unwrap();
    assert_eq!(
        value,
        json!({"rule_id": 9, "open_rule_references": 4, "open_classification_changes": 2})
    );
}

/// Round-trips the response `update_category` and `category_update_preview`
/// both return, so a UI-shape drift here is caught the same way
/// `commands_json.rs::category_serializes_snake_case` catches it for the
/// legacy `save_category`.
#[test]
fn category_response_still_serializes_snake_case_after_the_update_flow_addition() {
    let c = Category {
        id: 4,
        parent_id: Some(1),
        name: "potraviny".into(),
        kind: CategoryKind::Expense,
        sort: 0,
        system: false,
        archived: false,
    };
    let v = serde_json::to_value(&c).unwrap();
    assert_eq!(
        v,
        json!({"id": 4, "parent_id": 1, "name": "potraviny", "kind": "expense", "sort": 0, "system": false, "archived": false})
    );
}
