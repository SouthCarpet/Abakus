//! Exact JSON oracles from plan 091 review21-contract.md: standalone DTO,
//! snake_case evidence/status, camelCase Tauri argument, and null != [].
use parser::Checksum;
use serde_json::json;
use store::{StatementReview, StatementReviewStatus};

#[test]
fn known_empty_review_serializes_exact_public_fields_and_no_open_checks() {
    let review = StatementReview { statement_id: 7, checksum: Checksum::Ok, parser_warnings: Some(vec![]),
        unassigned_count: 0, suggested_count: 0, status: StatementReviewStatus::NoOpenChecks };
    let expected = json!({"statement_id":7,"checksum":{"status":"ok"},"parser_warnings":[],
        "unassigned_count":0,"suggested_count":0,"status":"no_open_checks"});
    assert_eq!(serde_json::to_value(&review).unwrap(), expected);
    assert_eq!(serde_json::from_value::<StatementReview>(expected).unwrap(), review);
}

#[test]
fn legacy_unknown_review_serializes_null_and_evidence_incomplete() {
    let review = StatementReview { statement_id: 8, checksum: Checksum::NotVerifiable, parser_warnings: None,
        unassigned_count: 0, suggested_count: 0, status: StatementReviewStatus::EvidenceIncomplete };
    let expected = json!({"statement_id":8,"checksum":{"status":"not_verifiable"},"parser_warnings":null,
        "unassigned_count":0,"suggested_count":0,"status":"evidence_incomplete"});
    assert_eq!(serde_json::to_value(&review).unwrap(), expected);
    assert_eq!(serde_json::from_value::<StatementReview>(expected).unwrap(), review);
}

#[test]
fn warning_mismatch_review_preserves_exact_text_cents_and_open_counts() {
    let review = StatementReview { statement_id: 9, checksum: Checksum::OffBy(-123),
        parser_warnings: Some(vec!["Žlté\nriadky".into(), "Žlté\nriadky".into()]),
        unassigned_count: 1, suggested_count: 2, status: StatementReviewStatus::NeedsAttention };
    let expected = json!({"statement_id":9,"checksum":{"status":"off_by","off_by":-123},
        "parser_warnings":["Žlté\nriadky","Žlté\nriadky"],"unassigned_count":1,"suggested_count":2,"status":"needs_attention"});
    assert_eq!(serde_json::to_value(&review).unwrap(), expected);
    assert_eq!(serde_json::from_value::<StatementReview>(expected).unwrap(), review);
}

#[test]
fn attention_status_keeps_null_warning_evidence_and_unverifiable_checksum() {
    let review = StatementReview { statement_id: 10, checksum: Checksum::NotVerifiable, parser_warnings: None,
        unassigned_count: 1, suggested_count: 0, status: StatementReviewStatus::NeedsAttention };
    assert_eq!(serde_json::to_value(review).unwrap(), json!({"statement_id":10,
        "checksum":{"status":"not_verifiable"},"parser_warnings":null,
        "unassigned_count":1,"suggested_count":0,"status":"needs_attention"}));
}

#[derive(serde::Deserialize)]
struct StatementReviewArgs { #[serde(rename = "statementId")] statement_id: i64 }

#[test]
fn statement_review_argument_accepts_the_ts_camel_case_id() {
    let args: StatementReviewArgs = serde_json::from_value(json!({"statementId":17})).unwrap();
    assert_eq!(args.statement_id, 17);
}
