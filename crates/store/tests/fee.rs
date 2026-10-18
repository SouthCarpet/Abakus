use chrono::NaiveDate;
use parser::{AccountKind, Statement, Transaction, TxKind};
use store::{Store, TxFilter};

const PERSONAL_IBAN: &str = "SK4411000000000012345678";
const BUSINESS_IBAN: &str = "SK3711000000000098765432";

fn transaction(day: u32, kind: TxKind, amount_cents: i64, merchant: &str) -> Transaction {
    let date = NaiveDate::from_ymd_opt(2026, 6, day).unwrap();
    let mut transaction = Transaction::blank(
        date,
        amount_cents,
        kind,
        format!("{day:02}.06.2026    {merchant}                       {:.2}{}", (amount_cents.abs() / 100), if amount_cents < 0 { "-" } else { "" }),
    );
    transaction.merchant_raw = merchant.to_string();
    transaction
}

fn statement(iban: &str, account_kind: AccountKind, number: u32, transactions: Vec<Transaction>) -> Statement {
    Statement {
        iban: iban.to_string(),
        account_kind,
        number,
        period_start: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        period_end: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        opening_cents: None,
        closing_cents: None,
        transactions,
        warnings: Vec::new(),
    }
}

fn store_with_fee_rows() -> Store {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
    store.upsert_account(BUSINESS_IBAN, AccountKind::Business, "Firemný").unwrap();

    let mut transfer = transaction(4, TxKind::TransferOut, -2_000, "Vlastný prevod");
    transfer.counterparty_iban = Some(BUSINESS_IBAN.to_string());
    store.import_statement(
        &statement(
            PERSONAL_IBAN,
            AccountKind::Personal,
            7,
            vec![
                transaction(1, TxKind::Fee, -250, "Poplatok za vedenie účtu"),
                transaction(2, TxKind::Fee, 50, "Poplatok vrátený bankou"),
                transaction(3, TxKind::Card, -1_000, "Obchod"),
                transfer,
                transaction(5, TxKind::Other, 5_000, "Príjem"),
            ],
        ),
        "personal-fees",
    ).unwrap();
    store.import_statement(
        &statement(
            BUSINESS_IBAN,
            AccountKind::Business,
            8,
            vec![transaction(6, TxKind::Fee, -300, "Poplatky za transakcie")],
        ),
        "business-fees",
    ).unwrap();
    store
}

#[test]
fn personal_fee_aggregate_is_a_scoped_subset_of_expenses_without_changing_other_totals() {
    let store = store_with_fee_rows();
    let personal_id = store.account_by_iban(PERSONAL_IBAN).unwrap().unwrap().id;

    let personal = store.summary(None, None, Some(personal_id)).unwrap();
    assert_eq!(personal.income_cents, 5_000);
    assert_eq!(personal.expense_cents, 1_200, "a positive fee correction reduces both fees and expenses by the same signed contribution");
    assert_eq!(personal.fee_cents, 200, "fees are a subset of expense_cents, expressed as signed expense contributions");
    assert_eq!(personal.transfer_cents, 2_000, "the internal transfer remains separate from expenses and fees");
    assert_eq!(personal.net_cents, 3_800);
}

#[test]
fn fee_by_month_uses_the_same_personal_account_scope_as_the_aggregate() {
    let store = store_with_fee_rows();
    let personal_id = store.account_by_iban(PERSONAL_IBAN).unwrap().unwrap().id;

    let personal = store.summary(None, None, Some(personal_id)).unwrap();
    assert_eq!(personal.by_month, vec![store::MonthRow { month: "2026-06".into(), income_cents: 5_000, expense_cents: 1_200, fee_cents: 200 }]);
}

#[test]
fn fee_statement_history_keeps_the_signed_all_kind_sum() {
    let store = store_with_fee_rows();
    let personal_id = store.account_by_iban(PERSONAL_IBAN).unwrap().unwrap().id;

    let history = store.statement_history(Some(personal_id), Some(AccountKind::Personal)).unwrap();
    assert_eq!(history[0].total_cents, 1_800, "statement history keeps its signed all-kind sum and automatically includes fee rows");
}

#[test]
fn fee_aggregate_for_all_accounts_includes_each_account() {
    let store = store_with_fee_rows();

    let all = store.summary(None, None, None).unwrap();
    assert_eq!(all.expense_cents, 1_500);
    assert_eq!(all.fee_cents, 500);
}

#[test]
fn fee_aggregate_with_an_inclusive_date_filter_uses_the_same_scope_as_expenses() {
    let store = store_with_fee_rows();
    let personal_id = store.account_by_iban(PERSONAL_IBAN).unwrap().unwrap().id;

    let through_first_day = store.summary(
        Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
        Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
        Some(personal_id),
    ).unwrap();
    assert_eq!(through_first_day.expense_cents, 250);
    assert_eq!(through_first_day.fee_cents, 250, "fee totals use the same account and inclusive date scope as the expense total");
}

#[test]
fn kind_filter_composes_with_text_and_csv_uses_the_same_rows() {
    let store = store_with_fee_rows();
    let personal_id = store.account_by_iban(PERSONAL_IBAN).unwrap().unwrap().id;
    let filter = TxFilter {
        account_id: Some(personal_id),
        kind: Some(TxKind::Fee),
        text: Some("poplatok".into()),
        ..Default::default()
    };

    let rows = store.list_transactions(&filter).unwrap();
    let csv = store.export_csv(&filter).unwrap();

    assert_eq!(rows.len(), 2, "kind, account and literal text filters intersect");
    assert!(rows.iter().all(|row| row.kind == "fee"));
    assert_eq!(csv.lines().count(), 3, "the CSV header plus the same two filtered rows");
    assert!(!csv.contains("Obchod"));
}

#[test]
fn changing_only_kind_does_not_change_the_fingerprint_or_duplicate_on_reimport() {
    let mut store = Store::open_in_memory().unwrap();
    store.upsert_account(PERSONAL_IBAN, AccountKind::Personal, "Osobný").unwrap();
    let other = transaction(1, TxKind::Other, -250, "Poplatok za vedenie účtu");
    let mut fee = other.clone();
    fee.kind = TxKind::Fee;

    let first = store.import_statement(&statement(PERSONAL_IBAN, AccountKind::Personal, 7, vec![other]), "legacy-file").unwrap();
    let second = store.import_statement(&statement(PERSONAL_IBAN, AccountKind::Personal, 7, vec![fee]), "new-parser-file").unwrap();

    assert_eq!((first.inserted, first.duplicates), (1, 0));
    assert_eq!((second.inserted, second.duplicates), (0, 1), "kind is intentionally absent from the stable transaction fingerprint");
    assert_eq!(store.list_transactions(&TxFilter::default()).unwrap().len(), 1);
}
