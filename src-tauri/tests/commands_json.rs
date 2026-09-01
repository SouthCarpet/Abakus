//! Amendment A6: the drift gate that replaces tauri-specta. For every command
//! argument shape and result struct, round-trip the exact JSON the UI sends
//! (literals copied from `src/api.ts` call sites) against the real Rust types.
use abakus_lib::import_flow::{ImportReport, ImportStatus};
use abakus_lib::net::AuditFailure;
use abakus_lib::update::Release;
use chrono::{NaiveDate, TimeZone, Utc};
use parser::{AccountKind, Checksum};
use rules::{RuleKind, Status};
use serde_json::json;
use store::{
    Account, AssignOutcome, BadChecksum, Category, CategoryKind, NetLogRow, RecentStatement,
    RuleView, StatementDeleteOutcome, StatementDeletePreview, Summary, TxFilter, TxRow,
};

/// Tauri deserializes each command argument from its own JSON field (there is
/// no single Rust struct for a multi-parameter command); these local structs
/// mirror the argument names/types so the JSON shape stays under test.
#[derive(serde::Deserialize)]
struct ImportStatementsArgs { paths: Vec<String> }

#[derive(serde::Deserialize)]
struct AssignArgs {
    ids: Vec<i64>,
    #[serde(rename = "categoryId")] category_id: i64,
    #[serde(rename = "applyToMatching")] apply_to_matching: bool,
}

#[derive(serde::Deserialize)]
struct RecentStatementsArgs { limit: usize }

#[derive(serde::Deserialize)]
struct SetCheckUpdatesArgs { on: bool }

#[derive(serde::Deserialize)]
struct NetLogArgs { limit: usize }

#[derive(serde::Deserialize)]
struct ClearPasswordArgs {
    #[serde(rename = "accountId")] account_id: i64,
}

#[derive(serde::Deserialize)]
struct SaveCategoryArgs {
    id: Option<i64>,
    #[serde(rename = "parentId")] parent_id: Option<i64>,
    name: String,
    kind: CategoryKind,
}

#[derive(serde::Deserialize)]
struct SummaryArgs {
    from: Option<String>,
    to: Option<String>,
    #[serde(rename = "accountId")] account_id: Option<i64>,
    /// A17/F3. Tauri lets a command omit an `Option` argument, so the field
    /// defaults here the same way.
    #[serde(default, rename = "accountKind")] account_kind: Option<AccountKind>,
}

#[derive(serde::Deserialize)]
struct StatementIdArgs {
    #[serde(rename = "statementId")] statement_id: i64,
}

#[derive(serde::Deserialize)]
struct UpdateAccountArgs {
    id: i64,
    label: String,
    kind: AccountKind,
    #[serde(rename = "acknowledgeKindChange")] acknowledge_kind_change: bool,
}

#[derive(serde::Deserialize)]
struct ImportWithPasswordArgs { path: String, password: String, remember: bool }

#[derive(serde::Deserialize)]
struct SaveAccountArgs { iban: String, kind: AccountKind, label: String }

#[derive(serde::Deserialize)]
struct ArchiveCategoryArgs { id: i64 }

#[derive(serde::Deserialize)]
struct DeleteRuleArgs { id: i64 }

#[derive(serde::Deserialize)]
struct ConfirmArgs { ids: Vec<i64> }

#[derive(serde::Deserialize)]
struct ExportCsvArgs { filter: TxFilter, path: String }

#[test]
fn import_statements_args_match_the_ui_call() {
    let args: ImportStatementsArgs = serde_json::from_value(json!({"paths": ["x.pdf"]})).unwrap();
    assert_eq!(args.paths, vec!["x.pdf".to_string()]);
}

#[test]
fn assign_args_match_the_ui_call() {
    let args: AssignArgs = serde_json::from_value(json!({"ids": [1], "categoryId": 2, "applyToMatching": false})).unwrap();
    assert_eq!(args.ids, vec![1]);
    assert_eq!(args.category_id, 2);
    assert!(!args.apply_to_matching);
}

#[test]
fn tx_filter_deserializes_snake_case_including_statement_id() {
    let v = json!({
        "from": "2026-06-01",
        "to": "2026-06-30",
        "account_id": 1,
        "category_id": 2,
        "status": "confirmed",
        "text": "aldi",
        "statement_id": 5
    });
    let f: TxFilter = serde_json::from_value(v).unwrap();
    assert_eq!(f.from, Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()));
    assert_eq!(f.to, Some(NaiveDate::from_ymd_opt(2026, 6, 30).unwrap()));
    assert_eq!(f.account_id, Some(1));
    assert_eq!(f.category_id, Some(2));
    assert_eq!(f.status, Some(Status::Confirmed));
    assert_eq!(f.text.as_deref(), Some("aldi"));
    assert_eq!(f.statement_id, Some(5));
}

#[test]
fn import_report_serializes_camel_case() {
    let r = ImportReport {
        path: "x.pdf".into(),
        status: ImportStatus::Imported,
        account_label: Some("Osobný".into()),
        account_kind: Some(AccountKind::Personal),
        iban_masked: Some("SK44...5678".into()),
        iban: Some("SK4411000000000012345678".into()),
        statement_number: Some(6),
        period_start: None,
        period_end: None,
        inserted: 8,
        duplicates: 0,
        checksum: Some(Checksum::Ok),
        warnings: Vec::new(),
        message: None,
        statement_id: Some(9),
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["accountLabel"], json!("Osobný"));
    assert_eq!(v["accountKind"], json!("personal"));
    assert_eq!(v["ibanMasked"], json!("SK44...5678"));
    assert_eq!(v["statementNumber"], json!(6));
    assert_eq!(v["statementId"], json!(9));
    assert!(v.get("account_label").is_none(), "must not fall back to snake_case");
}

#[test]
fn tx_row_and_summary_serialize_snake_case() {
    let row = TxRow {
        id: 1, account_id: 2, account_kind: AccountKind::Personal, statement_number: 6,
        posted_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(), tx_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        kind: "card".into(), amount_cents: -199, orig_amount_cents: None, orig_currency: None,
        merchant_raw: "ALDI SUED".into(), place: Some("Neuss".into()), counterparty_name: None, counterparty_iban: None,
        category_id: None, category_name: None, parent_name: None, status: Status::Suggested, source: "seed".into(), raw_block: "raw".into(),
    };
    let v = serde_json::to_value(&row).unwrap();
    assert_eq!(v["account_id"], json!(2));
    assert_eq!(v["merchant_raw"], json!("ALDI SUED"));
    assert!(v.get("accountId").is_none(), "TxRow keeps snake_case field names, no camelCase");

    let summary = Summary { income_cents: 0, expense_cents: 199, transfer_cents: 0, net_cents: -199, unassigned_count: 0, suggested_count: 1, by_category: Vec::new(), by_month: Vec::new(), by_month_category: Vec::new(), top_merchants: Vec::new() };
    let sv = serde_json::to_value(&summary).unwrap();
    assert_eq!(sv["expense_cents"], json!(199));
    assert!(sv.get("expenseCents").is_none(), "Summary keeps snake_case field names, no camelCase");
}

#[test]
fn checksum_off_by_serializes_with_tag_and_content() {
    let v = serde_json::to_value(Checksum::OffBy(32)).unwrap();
    assert_eq!(v, json!({"status": "off_by", "off_by": 32}));
    assert_eq!(serde_json::to_value(Checksum::Ok).unwrap(), json!({"status": "ok"}));
}

#[test]
fn recent_statements_args_match_the_ui_call() {
    let args: RecentStatementsArgs = serde_json::from_value(json!({"limit": 5})).unwrap();
    assert_eq!(args.limit, 5);
}

#[test]
fn set_check_updates_args_match_the_ui_call() {
    let args: SetCheckUpdatesArgs = serde_json::from_value(json!({"on": true})).unwrap();
    assert!(args.on);
}

#[test]
fn net_log_args_match_the_ui_call() {
    let args: NetLogArgs = serde_json::from_value(json!({"limit": 20})).unwrap();
    assert_eq!(args.limit, 20);
}

#[test]
fn clear_password_args_match_the_ui_call() {
    let args: ClearPasswordArgs = serde_json::from_value(json!({"accountId": 3})).unwrap();
    assert_eq!(args.account_id, 3);
}

#[test]
fn save_category_args_match_the_ui_call() {
    let args: SaveCategoryArgs = serde_json::from_value(json!({"id": null, "parentId": 2, "name": "nová", "kind": "expense"})).unwrap();
    assert_eq!(args.id, None);
    assert_eq!(args.parent_id, Some(2));
    assert_eq!(args.name, "nová");
    assert_eq!(args.kind, CategoryKind::Expense);
}

#[test]
fn summary_args_match_the_ui_call() {
    let args: SummaryArgs = serde_json::from_value(json!({"from": "2026-06-01", "to": null, "accountId": 1})).unwrap();
    assert_eq!(args.from.as_deref(), Some("2026-06-01"));
    assert_eq!(args.to, None);
    assert_eq!(args.account_id, Some(1));
}

#[test]
fn import_with_password_args_match_the_ui_call() {
    let args: ImportWithPasswordArgs = serde_json::from_value(json!({"path": "x.pdf", "password": "heslo", "remember": true})).unwrap();
    assert_eq!(args.path, "x.pdf");
    assert_eq!(args.password, "heslo");
    assert!(args.remember);
}

#[test]
fn save_account_args_match_the_ui_call() {
    let args: SaveAccountArgs = serde_json::from_value(json!({"iban": "SK4411000000000012345678", "kind": "personal", "label": "Osobný"})).unwrap();
    assert_eq!(args.iban, "SK4411000000000012345678");
    assert_eq!(args.kind, AccountKind::Personal);
    assert_eq!(args.label, "Osobný");
}

#[test]
fn archive_category_args_match_the_ui_call() {
    let args: ArchiveCategoryArgs = serde_json::from_value(json!({"id": 4})).unwrap();
    assert_eq!(args.id, 4);
}

#[test]
fn delete_rule_args_match_the_ui_call() {
    let args: DeleteRuleArgs = serde_json::from_value(json!({"id": 9})).unwrap();
    assert_eq!(args.id, 9);
}

#[test]
fn confirm_args_match_the_ui_call() {
    let args: ConfirmArgs = serde_json::from_value(json!({"ids": [1, 2, 3]})).unwrap();
    assert_eq!(args.ids, vec![1, 2, 3]);
}

#[test]
fn export_csv_args_match_the_ui_call() {
    let v = json!({
        "filter": {
            "from": "2026-06-01",
            "to": null,
            "account_id": 1,
            "category_id": null,
            "status": "confirmed",
            "text": null,
            "statement_id": null
        },
        "path": "out.csv"
    });
    let args: ExportCsvArgs = serde_json::from_value(v).unwrap();
    assert_eq!(args.filter.from, Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()));
    assert_eq!(args.filter.account_id, Some(1));
    assert_eq!(args.filter.status, Some(Status::Confirmed));
    assert_eq!(args.path, "out.csv");
}

#[test]
fn release_serializes_with_the_fields_the_ui_reads() {
    let release = Release { tag: "0.2.0".into(), url: "https://github.com/SouthCarpet/Abakus/releases/tag/0.2.0".into(), notes: "poznamky".into() };
    let v = serde_json::to_value(&release).unwrap();
    assert_eq!(v["tag"], json!("0.2.0"));
    assert_eq!(v["url"], json!("https://github.com/SouthCarpet/Abakus/releases/tag/0.2.0"));
    assert_eq!(v["notes"], json!("poznamky"));
}

#[test]
fn net_log_row_serializes_snake_case() {
    let row = NetLogRow {
        id: 1,
        started_at: Utc.with_ymd_and_hms(2026, 8, 31, 10, 0, 0).unwrap(),
        url: "https://api.github.com/repos/SouthCarpet/Abakus/releases/latest".into(),
        status: "200".into(),
        duration_ms: 120,
        bytes_in: 512,
    };
    let v = serde_json::to_value(&row).unwrap();
    assert_eq!(v["duration_ms"], json!(120));
    assert_eq!(v["bytes_in"], json!(512));
    assert!(v.get("durationMs").is_none(), "NetLogRow keeps snake_case field names, no camelCase");
}

#[test]
fn recent_statement_serializes_snake_case() {
    let row = RecentStatement {
        statement_id: 1,
        number: 6,
        period_end: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        account_label: "Osobný".into(),
        transaction_count: 8,
        checksum: Checksum::Ok,
    };
    let v = serde_json::to_value(&row).unwrap();
    assert_eq!(v["statement_id"], json!(1));
    assert_eq!(v["transaction_count"], json!(8));
    assert_eq!(v["checksum"], json!({"status": "ok"}));
    assert!(v.get("statementId").is_none(), "RecentStatement keeps snake_case field names, no camelCase");
}

/// Controller addition from the t11 review: round-trip the result shapes the
/// t11 reviewer verified by hand on 193287f, so drift is caught automatically.
#[test]
fn assign_outcome_serializes_snake_case_incl_skipped_transfers() {
    let o = AssignOutcome { updated: 3, rules_created: 2, skipped_transfers: 1 };
    let v = serde_json::to_value(&o).unwrap();
    assert_eq!(v, json!({"updated": 3, "rules_created": 2, "skipped_transfers": 1}));
}

#[test]
fn bad_checksum_serializes_snake_case_all_four_fields() {
    let b = BadChecksum { statement_id: 5, number: 7, account_label: "Osobný".into(), off_by_cents: 32 };
    let v = serde_json::to_value(&b).unwrap();
    assert_eq!(v, json!({"statement_id": 5, "number": 7, "account_label": "Osobný", "off_by_cents": 32}));
}

#[test]
fn account_serializes_snake_case() {
    let a = Account { id: 1, iban: "SK4411000000000012345678".into(), kind: AccountKind::Personal, label: "Osobný".into(), has_password: true };
    let v = serde_json::to_value(&a).unwrap();
    assert_eq!(v, json!({"id": 1, "iban": "SK4411000000000012345678", "kind": "personal", "label": "Osobný", "has_password": true}));
}

#[test]
fn category_serializes_snake_case() {
    let c = Category { id: 4, parent_id: Some(1), name: "potraviny".into(), kind: CategoryKind::Expense, sort: 0, system: false, archived: false };
    let v = serde_json::to_value(&c).unwrap();
    assert_eq!(v, json!({"id": 4, "parent_id": 1, "name": "potraviny", "kind": "expense", "sort": 0, "system": false, "archived": false}));
}

/// A17/F3: the Prehľad sends a KIND, not the id of one account of that kind.
#[test]
fn summary_args_carry_the_account_kind() {
    let args: SummaryArgs = serde_json::from_value(json!({"from": null, "to": null, "accountId": null, "accountKind": "business"})).unwrap();
    assert_eq!(args.account_kind, Some(AccountKind::Business));
    assert_eq!(args.account_id, None);
}

/// A17/F1: both delete commands take the statement id under the same name.
#[test]
fn statement_delete_args_match_the_ui_call() {
    let args: StatementIdArgs = serde_json::from_value(json!({"statementId": 5})).unwrap();
    assert_eq!(args.statement_id, 5);
}

/// A17/F2: no `iban` field, on purpose. The IBAN is the identity and an edit
/// cannot carry one.
#[test]
fn update_account_args_match_the_ui_call() {
    let args: UpdateAccountArgs = serde_json::from_value(json!({"id": 1, "label": "Firma s.r.o.", "kind": "business", "acknowledgeKindChange": true})).unwrap();
    assert_eq!((args.id, args.label.as_str(), args.kind, args.acknowledge_kind_change), (1, "Firma s.r.o.", AccountKind::Business, true));
}

#[test]
fn statement_delete_preview_serializes_snake_case() {
    let p = StatementDeletePreview {
        statement_id: 5, number: 6, account_label: "Osobný".into(),
        period_start: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(), period_end: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        transaction_count: 8, confirmed_count: 2, rules_deleted: 1,
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"statement_id": 5, "number": 6, "account_label": "Osobný", "period_start": "2026-06-01", "period_end": "2026-06-30", "transaction_count": 8, "confirmed_count": 2, "rules_deleted": 1}));
}

#[test]
fn statement_delete_outcome_serializes_snake_case() {
    let o = StatementDeleteOutcome { statement_id: 5, number: 6, transactions_deleted: 8, rules_deleted: 1, open_rows_reclassified: 3 };
    let v = serde_json::to_value(&o).unwrap();
    assert_eq!(v, json!({"statement_id": 5, "number": 6, "transactions_deleted": 8, "rules_deleted": 1, "open_rows_reclassified": 3}));
}

/// A17/F7: the shape Nastavenia reads when an audit row could not be written.
#[test]
fn audit_failure_serializes_with_the_fields_the_ui_reads() {
    let f = AuditFailure { at: Utc.with_ymd_and_hms(2026, 9, 1, 10, 0, 0).unwrap(), url: "https://api.github.com/x".into(), error: "store lock poisoned".into() };
    let v = serde_json::to_value(&f).unwrap();
    assert_eq!(v["url"], json!("https://api.github.com/x"));
    assert_eq!(v["error"], json!("store lock poisoned"));
    assert!(v.get("at").is_some(), "the UI shows when the audit row was lost");
}

/// The A17/F3 filter field must be optional on the wire: an older payload
/// without it still deserializes.
#[test]
fn tx_filter_account_kind_is_optional_and_round_trips() {
    let without: TxFilter = serde_json::from_value(json!({"from": null, "to": null, "account_id": null, "category_id": null, "status": null, "text": null, "statement_id": null})).unwrap();
    assert_eq!(without.account_kind, None);
    let with: TxFilter = serde_json::from_value(json!({"from": null, "to": null, "account_id": null, "account_kind": "personal", "category_id": null, "status": null, "text": null, "statement_id": null})).unwrap();
    assert_eq!(with.account_kind, Some(AccountKind::Personal));
}

#[test]
fn rule_view_serializes_snake_case() {
    let r = RuleView { id: 9, kind: RuleKind::Exact, key: "aldi".into(), place: Some("neuss".into()), category_id: 4, category_name: "potraviny".into(), parent_name: Some("Jedlo".into()), hit_count: 2 };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v, json!({"id": 9, "kind": "exact", "key": "aldi", "place": "neuss", "category_id": 4, "category_name": "potraviny", "parent_name": "Jedlo", "hit_count": 2}));
}
