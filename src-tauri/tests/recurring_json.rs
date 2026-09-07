//! Owned by lane A. Same drift-gate shape as `commands_json.rs`: round-trip
//! the exact JSON the UI would send/receive against the real wire types
//! from `store::recurring` (recurring-contract.md §7), not a duplicated
//! test-only struct. Outer invoke argument names are camelCase; object
//! fields inside them (request AND response) are snake_case.
use chrono::NaiveDate;
use serde_json::json;
use store::recurring::{
    Cadence, PriceChange, RecurringAmounts, RecurringDecision, RecurringDecisionInput, RecurringDecisionMode, RecurringDetail, RecurringDetailRequest, RecurringDirection, RecurringOverview,
    RecurringQuery, RecurringRow, RecurringScope, RecurringSelection, RecurringState, RowDecision, SaveRecurringRequest, UnknownReason,
};

/// `transaction_recurring_context(transactionId, asOf)`: two primitive
/// invoke arguments, not one struct, so Tauri deserializes each field
/// separately (same shape as `commands_json.rs`'s `ClearPasswordArgs`).
#[derive(serde::Deserialize)]
struct TransactionRecurringContextArgs {
    #[serde(rename = "transactionId")]
    transaction_id: i64,
    #[serde(rename = "asOf")]
    as_of: NaiveDate,
}

#[derive(serde::Deserialize)]
struct ResetRecurringArgs {
    #[serde(rename = "decisionId")]
    decision_id: i64,
}

#[test]
fn recurring_overview_query_arg_matches_the_ui_call() {
    let v = json!({"from": null, "to": null, "account_kind": "personal", "today": "2026-09-07"});
    let query: RecurringQuery = serde_json::from_value(v).unwrap();
    assert_eq!(query.from, None);
    assert_eq!(query.to, None);
    assert_eq!(query.account_kind, Some(parser::AccountKind::Personal));
    assert_eq!(query.today, NaiveDate::from_ymd_opt(2026, 9, 7).unwrap());
}

#[test]
fn recurring_query_rejects_an_unknown_field() {
    let v = json!({"from": null, "to": null, "account_kind": null, "today": "2026-09-07", "bogus": 1});
    assert!(serde_json::from_value::<RecurringQuery>(v).is_err(), "RecurringQuery must reject unknown fields (deny_unknown_fields)");
}

#[test]
fn recurring_detail_request_arg_matches_the_ui_call() {
    let v = json!({
        "series_key": "g:abc123",
        "query": {"from": "2026-09-01", "to": "2026-09-30", "account_kind": null, "today": "2026-09-07"}
    });
    let req: RecurringDetailRequest = serde_json::from_value(v).unwrap();
    assert_eq!(req.series_key, "g:abc123");
    assert_eq!(req.query.from, Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()));
}

#[test]
fn transaction_recurring_context_args_match_the_ui_call() {
    let args: TransactionRecurringContextArgs = serde_json::from_value(json!({"transactionId": 42, "asOf": "2026-09-07"})).unwrap();
    assert_eq!(args.transaction_id, 42);
    assert_eq!(args.as_of, NaiveDate::from_ymd_opt(2026, 9, 7).unwrap());
}

#[test]
fn reset_recurring_args_match_the_ui_call() {
    let args: ResetRecurringArgs = serde_json::from_value(json!({"decisionId": 7})).unwrap();
    assert_eq!(args.decision_id, 7);
}

#[test]
fn save_recurring_request_group_scope_confirmed_deserializes() {
    let v = json!({
        "request": {
            "decision_id": null,
            "selection": {"scope": "group", "transaction_id": 5},
            "decision": {"mode": "confirmed", "cadence": "monthly", "anchor_date": "2026-01-31"}
        }
    });
    let req: SaveRecurringRequest = serde_json::from_value(v["request"].clone()).unwrap();
    assert_eq!(req.decision_id, None);
    assert!(matches!(req.selection, RecurringSelection::Group { transaction_id: 5 }));
    assert!(matches!(req.decision, RecurringDecisionInput::Confirmed { cadence: Cadence::Monthly, .. }));
}

#[test]
fn save_recurring_request_selected_scope_ignored_deserializes() {
    let v = json!({
        "decision_id": 9,
        "selection": {"scope": "selected", "transaction_ids": [1, 2, 3]},
        "decision": {"mode": "ignored"}
    });
    let req: SaveRecurringRequest = serde_json::from_value(v).unwrap();
    assert_eq!(req.decision_id, Some(9));
    match req.selection {
        RecurringSelection::Selected { transaction_ids } => assert_eq!(transaction_ids, vec![1, 2, 3]),
        other => panic!("expected Selected, got {other:?}"),
    }
    assert!(matches!(req.decision, RecurringDecisionInput::Ignored));
}

#[test]
fn save_recurring_request_rejects_unknown_fields_in_every_layer() {
    let outer_bogus = json!({"decision_id": null, "selection": {"scope": "group", "transaction_id": 1}, "decision": {"mode": "ignored"}, "bogus": 1});
    assert!(serde_json::from_value::<SaveRecurringRequest>(outer_bogus).is_err());
    let selection_bogus = json!({"decision_id": null, "selection": {"scope": "group", "transaction_id": 1, "bogus": 1}, "decision": {"mode": "ignored"}});
    assert!(serde_json::from_value::<SaveRecurringRequest>(selection_bogus).is_err());
}

#[test]
fn recurring_decision_response_serializes_snake_case_object_fields() {
    let d = RecurringDecision {
        id: 1,
        series_key: "g:abc".into(),
        group_key: "abc".into(),
        account_id: 2,
        scope: RecurringScope::Group,
        mode: RecurringDecisionMode::Confirmed,
        cadence: Some(Cadence::Yearly),
        anchor_date: Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
        updated_at: "2026-09-07 00:00:00".into(),
    };
    let v = serde_json::to_value(&d).unwrap();
    assert_eq!(v["series_key"], "g:abc");
    assert_eq!(v["group_key"], "abc");
    assert_eq!(v["account_id"], 2);
    assert_eq!(v["scope"], "group");
    assert_eq!(v["mode"], "confirmed");
    assert_eq!(v["cadence"], "yearly");
    assert_eq!(v["anchor_date"], "2026-01-15");
}

#[test]
fn recurring_row_and_price_change_serialize_the_exact_ts_field_names() {
    let row = RecurringRow {
        series_key: "g:abc".into(),
        group_key: "abc".into(),
        decision_id: None,
        scope: RecurringScope::Group,
        decision: RowDecision::Estimate,
        account_id: 1,
        account_label: "Osobný".into(),
        account_kind: parser::AccountKind::Personal,
        direction: RecurringDirection::Expense,
        name: "NETFLIX".into(),
        cadence: Some(Cadence::Monthly),
        anchor_date: Some(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap()),
        subscription_category: true,
        category_id: None,
        evidence_count: 3,
        last_paid: Some(NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()),
        next_due: Some(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap()),
        state: RecurringState::Upcoming,
        unknown_reason: None,
        grace_until: None,
        currency: None,
        original_amount_cents: None,
        amount_cents: Some(1200),
        amount_basis: Some(store::recurring::AmountBasis::StablePair),
        monthly_cents: Some(1200),
        annual_cents: Some(14_400),
        price_change: Some(PriceChange { currency: "EUR".into(), previous_cents: 1000, current_cents: 1200, delta_cents: 200, effective_from: NaiveDate::from_ymd_opt(2026, 3, 31).unwrap() }),
        manual_membership: false,
        foreign_eur_estimate: false,
    };
    let v = serde_json::to_value(&row).unwrap();
    assert_eq!(v["subscription_category"], true);
    assert_eq!(v["evidence_count"], 3);
    assert_eq!(v["next_due"], "2026-04-30");
    assert_eq!(v["state"], "upcoming");
    assert_eq!(v["unknown_reason"], serde_json::Value::Null);
    assert_eq!(v["amount_basis"], "stable_pair");
    assert_eq!(v["price_change"]["delta_cents"], 200);
    assert_eq!(v["manual_membership"], false);
    assert_eq!(v["foreign_eur_estimate"], false);
}

#[test]
fn unknown_reason_and_state_use_the_exact_contract_tokens() {
    for (variant, token) in [
        (UnknownReason::MissingCoverage, "missing_coverage"),
        (UnknownReason::AmbiguousMembership, "ambiguous_membership"),
        (UnknownReason::NoEvidence, "no_evidence"),
        (UnknownReason::UnmatchedHistory, "unmatched_history"),
    ] {
        assert_eq!(serde_json::to_value(variant).unwrap(), token);
    }
    for (variant, token) in [(RecurringState::Active, "active"), (RecurringState::Upcoming, "upcoming"), (RecurringState::Missing, "missing"), (RecurringState::Ended, "ended"), (RecurringState::Unknown, "unknown")] {
        assert_eq!(serde_json::to_value(variant).unwrap(), token);
    }
}

/// `RecurringOverview`/`RecurringDetail`/`RecurringAmounts` round-trip
/// through the exact field names a frontend consumer would read.
#[test]
fn recurring_overview_and_amounts_shape_match_the_contract() {
    let overview = RecurringOverview {
        as_of: NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
        history_from: None,
        future_period: false,
        unfinished_period: true,
        rows: Vec::new(),
        confirmed: RecurringAmounts { monthly_income_cents: 0, monthly_expense_cents: 1200, monthly_net_cents: -1200, annual_income_cents: 0, annual_expense_cents: 14_400, annual_net_cents: -14_400, remaining_income_cents: 0, remaining_expense_cents: 1200 },
        estimates: RecurringAmounts::default(),
        excluded: store::recurring::RecurringExcluded { missing: 0, ended: 0, unknown: 1 },
        expense_share_basis_points: Some(488),
        average_expense_cents: Some(110_000),
        average_months: vec!["2026-07".into(), "2026-08".into()],
    };
    let v = serde_json::to_value(&overview).unwrap();
    assert_eq!(v["excluded"]["unknown"], 1);
    assert_eq!(v["expense_share_basis_points"], 488);
    assert_eq!(v["average_months"][0], "2026-07");
    assert_eq!(v["confirmed"]["monthly_net_cents"], -1200);

    let detail = RecurringDetail { row: overview_row(), transactions: Vec::new(), matching_transaction_ids: vec![1, 2, 3], compatible_transactions: Vec::new() };
    let dv = serde_json::to_value(&detail).unwrap();
    assert_eq!(dv["matching_transaction_ids"], json!([1, 2, 3]));
}

fn overview_row() -> RecurringRow {
    RecurringRow {
        series_key: "g:abc".into(),
        group_key: "abc".into(),
        decision_id: None,
        scope: RecurringScope::Group,
        decision: RowDecision::Estimate,
        account_id: 1,
        account_label: "Osobný".into(),
        account_kind: parser::AccountKind::Personal,
        direction: RecurringDirection::Expense,
        name: "NETFLIX".into(),
        cadence: None,
        anchor_date: None,
        subscription_category: false,
        category_id: None,
        evidence_count: 0,
        last_paid: None,
        next_due: None,
        state: RecurringState::Unknown,
        unknown_reason: Some(UnknownReason::NoEvidence),
        grace_until: None,
        currency: None,
        original_amount_cents: None,
        amount_cents: None,
        amount_basis: None,
        monthly_cents: None,
        annual_cents: None,
        price_change: None,
        manual_membership: false,
        foreign_eur_estimate: false,
    }
}
