use parser::TxKind;
use serde_json::json;
use store::{MonthRow, Summary, TxFilter};

#[test]
fn fee_kind_filter_deserializes_and_absence_keeps_the_filter_open() {
    let fee: TxFilter = serde_json::from_value(json!({"kind": "fee"})).unwrap();
    let without_kind: TxFilter = serde_json::from_value(json!({})).unwrap();
    let null_kind: TxFilter = serde_json::from_value(json!({"kind": null})).unwrap();

    assert_eq!(fee.kind, Some(TxKind::Fee), "src/api.ts sends the snake_case fee enum value");
    assert_eq!(without_kind.kind, None, "older payloads omit kind and keep all transaction kinds");
    assert_eq!(null_kind.kind, None, "the optional filter also accepts explicit null");
}

#[test]
fn unknown_kind_filter_is_rejected_instead_of_silently_listing_all_rows() {
    // The typed TxFilter contract accepts only parser TxKind values.
    assert!(serde_json::from_value::<TxFilter>(json!({"kind": "fees"})).is_err());
}

#[test]
fn fee_summary_fields_serialize_as_required_signed_cent_fields() {
    let summary = Summary {
        income_cents: 1_000,
        expense_cents: 300,
        transfer_cents: 0,
        fee_cents: 25,
        net_cents: 700,
        unassigned_count: 0,
        suggested_count: 1,
        by_category: Vec::new(),
        by_month: vec![MonthRow { month: "2026-06".into(), income_cents: 1_000, expense_cents: 300, fee_cents: 25 }],
        by_month_category: Vec::new(),
        top_merchants: Vec::new(),
    };

    let json = serde_json::to_value(summary).unwrap();
    assert_eq!(json["fee_cents"], 25);
    assert_eq!(json["by_month"][0]["fee_cents"], 25);
    assert!(json.get("feeCents").is_none(), "Summary keeps the existing snake_case DTO convention");
}
