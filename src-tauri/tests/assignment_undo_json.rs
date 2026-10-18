//! Plan 091 undo command and response JSON contract.
use serde_json::json;
use store::{BulkAssignOutcome, UndoAssignmentOutcome};

#[derive(serde::Deserialize)]
struct BulkAssignArgs {
    ids: Vec<i64>,
    #[serde(rename = "categoryId")]
    category_id: i64,
    #[serde(rename = "applyToMatching")]
    apply_to_matching: bool,
}

#[derive(serde::Deserialize)]
struct UndoLastAssignmentArgs {
    #[serde(rename = "expectedUndoId")]
    expected_undo_id: String,
}

#[test]
fn bulk_assign_arguments_match_the_typescript_request() {
    let args: BulkAssignArgs = serde_json::from_value(json!({
        "ids": [7, 9],
        "categoryId": 4,
        "applyToMatching": true
    }))
    .unwrap();

    assert_eq!(args.ids, vec![7, 9]);
    assert_eq!(args.category_id, 4);
    assert!(args.apply_to_matching);
}

#[test]
fn bulk_assign_outcome_serializes_every_field() {
    let outcome = BulkAssignOutcome {
        updated: 3,
        rules_created: 2,
        skipped_transfers: 1,
        undo_id: Some("assignment-undo-17".into()),
    };

    assert_eq!(
        serde_json::to_value(outcome).unwrap(),
        json!({
            "updated": 3,
            "rules_created": 2,
            "skipped_transfers": 1,
            "undo_id": "assignment-undo-17"
        })
    );
}

#[test]
fn undo_argument_and_nullable_outcome_match_the_typescript_request() {
    let args: UndoLastAssignmentArgs = serde_json::from_value(json!({
        "expectedUndoId": "assignment-undo-17"
    }))
    .unwrap();
    assert_eq!(args.expected_undo_id, "assignment-undo-17");

    assert_eq!(
        serde_json::to_value(Some(UndoAssignmentOutcome { restored_rows: 3 })).unwrap(),
        json!({"restored_rows": 3})
    );
    assert_eq!(
        serde_json::to_value(Option::<UndoAssignmentOutcome>::None).unwrap(),
        serde_json::Value::Null
    );
}
