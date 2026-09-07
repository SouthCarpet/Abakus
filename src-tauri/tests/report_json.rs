//! Exact TypeScript-facing JSON contract for PDF report commands (P03).
use abakus_lib::report::PdfReportOutcome;
use chrono::NaiveDate;
use parser::AccountKind;
use serde_json::json;
use std::collections::BTreeSet;
use store::report::{
    ReportAccount, ReportDateRange, ReportPeriod, ReportPreview, ReportRequest, ReportScope,
};

#[test]
fn report_request_deserializes_the_exact_snake_case_union() {
    let request: ReportRequest = serde_json::from_value(json!({
        "period": {"kind": "six_months", "ending_month": "2026-09"},
        "scope": {"kind": "kind", "account_kind": "personal"}
    }))
    .unwrap();
    assert_eq!(
        request.period,
        ReportPeriod::SixMonths {
            ending_month: "2026-09".into()
        }
    );
    assert_eq!(
        request.scope,
        ReportScope::Kind {
            account_kind: AccountKind::Personal
        }
    );
}

#[test]
fn report_request_rejects_unknown_fields_at_every_input_level() {
    for value in [
        json!({"period":{"kind":"all_time"},"scope":{"kind":"all"},"text":"filter leak"}),
        json!({"period":{"kind":"month","month":"2026-09","page":2},"scope":{"kind":"all"}}),
        json!({"period":{"kind":"all_time"},"scope":{"kind":"account","account_id":1,"status":"confirmed"}}),
    ] {
        assert!(serde_json::from_value::<ReportRequest>(value).is_err());
    }
}

#[test]
fn preview_and_outcome_serialize_dates_counts_and_path_exactly() {
    let preview = ReportPreview {
        range: Some(ReportDateRange {
            from: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            to: NaiveDate::from_ymd_opt(2026, 9, 30).unwrap(),
        }),
        scope_label: "Osobné účty".into(),
        accounts: vec![ReportAccount {
            id: 7,
            label: "Domácnosť".into(),
            kind: AccountKind::Personal,
            iban_suffix: "5678".into(),
        }],
        transaction_count: 31,
        latest_transaction_date: Some(NaiveDate::from_ymd_opt(2026, 9, 29).unwrap()),
        captured_at: "2026-09-07T14:30:00+02:00".into(),
        unfinished_period: true,
        accounts_without_statements: 0,
        accounts_with_gaps: 1,
        unverified_statement_count: 2,
        invalid_statement_range_count: 3,
    };
    let value = serde_json::to_value(PdfReportOutcome {
        path: "A:\\reports\\month.pdf".into(),
        bytes: 123_456,
        pages: 7,
        report: preview,
    })
    .unwrap();
    assert_eq!(value["path"], "A:\\reports\\month.pdf");
    assert_eq!(value["bytes"], 123_456);
    assert_eq!(value["pages"], 7);
    assert_eq!(value["report"]["range"]["from"], "2026-09-01");
    assert_eq!(value["report"]["accounts"][0]["kind"], "personal");
    assert_eq!(value["report"]["transaction_count"], 31);
    assert_eq!(
        value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["bytes", "pages", "path", "report"])
    );
    assert_eq!(
        value["report"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "accounts",
            "accounts_with_gaps",
            "accounts_without_statements",
            "captured_at",
            "invalid_statement_range_count",
            "latest_transaction_date",
            "range",
            "scope_label",
            "transaction_count",
            "unfinished_period",
            "unverified_statement_count",
        ])
    );
}
