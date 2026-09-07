use super::coverage::{coverage_for, StatementRange};
use super::dates::resolve_period;
use super::totals::calculate;
use super::{
    ReportAccount, ReportClock, ReportDateRange, ReportPreview, ReportRequest, ReportScope,
    ReportSnapshot, ReportTransaction,
};
use crate::{CategoryKind, Result, Store, StoreError, TxRow};
use chrono::{Datelike, NaiveDate};
use parser::AccountKind;
use rules::Status;
use rusqlite::{params_from_iter, Transaction};
use std::collections::HashMap;

const JS_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const REPORT_TRANSACTION_LIMIT: usize = 100_000;
const REPORT_TEXT_LIMIT: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportCaptureLimits {
    pub max_transactions: usize,
    pub max_text_bytes: usize,
}

impl Default for ReportCaptureLimits {
    fn default() -> Self {
        Self {
            max_transactions: REPORT_TRANSACTION_LIMIT,
            max_text_bytes: REPORT_TEXT_LIMIT,
        }
    }
}

impl Store {
    pub fn report_snapshot(
        &mut self,
        request: &ReportRequest,
        clock: &ReportClock,
    ) -> Result<ReportSnapshot> {
        self.report_snapshot_with_hook(request, clock, ReportCaptureLimits::default(), || {})
    }

    #[doc(hidden)]
    pub fn report_snapshot_with_hook<F>(
        &mut self,
        request: &ReportRequest,
        clock: &ReportClock,
        limits: ReportCaptureLimits,
        after_snapshot_established: F,
    ) -> Result<ReportSnapshot>
    where
        F: FnOnce(),
    {
        let tx = self.conn.transaction()?;
        let result = capture(&tx, request, clock, limits, after_snapshot_established);
        match result {
            Ok(snapshot) => {
                tx.commit()?;
                Ok(snapshot)
            }
            Err(error) => {
                tx.rollback()?;
                Err(error)
            }
        }
    }
}

fn capture<F>(
    tx: &Transaction,
    request: &ReportRequest,
    clock: &ReportClock,
    limits: ReportCaptureLimits,
    after_snapshot_established: F,
) -> Result<ReportSnapshot>
where
    F: FnOnce(),
{
    let all_accounts = read_accounts(tx)?;
    let accounts = select_accounts(all_accounts, &request.scope)?;
    validate_account_ids(&accounts)?;
    after_snapshot_established();
    let (finite_range, unfinished_period) = resolve_period(&request.period, clock.today)?;
    let transactions = read_transactions(tx, &accounts, finite_range, limits)?;
    let range = finite_range.or_else(|| transaction_range(&transactions));
    let statements = read_statements(tx, &accounts)?;
    let coverage = accounts
        .iter()
        .map(|account| coverage_for(account.id, &statements, finite_range))
        .collect::<Result<Vec<_>>>()?;
    let (totals, months, categories) = calculate(&transactions, range)?;
    let preview = make_preview(
        request,
        accounts,
        &transactions,
        &coverage,
        range,
        unfinished_period,
        clock,
    )?;
    Ok(ReportSnapshot {
        preview,
        totals,
        months,
        categories,
        coverage,
        transactions,
    })
}

fn read_accounts(tx: &Transaction) -> Result<Vec<StoredAccount>> {
    let mut statement = tx.prepare("SELECT id, iban, kind, label FROM accounts ORDER BY id")?;
    let rows = statement.query_map([], |row| {
        let kind: String = row.get(2)?;
        Ok(StoredAccount {
            id: row.get(0)?,
            iban: row.get(1)?,
            kind: parse_account_kind(&kind),
            label: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

#[derive(Debug)]
struct StoredAccount {
    id: i64,
    iban: String,
    kind: AccountKind,
    label: String,
}

fn select_accounts(all: Vec<StoredAccount>, scope: &ReportScope) -> Result<Vec<ReportAccount>> {
    if let ReportScope::Account { account_id } = scope {
        validate_requested_id(*account_id)?;
        if !all.iter().any(|account| account.id == *account_id) {
            return Err(StoreError::UnknownAccountId { id: *account_id });
        }
    }
    Ok(all
        .into_iter()
        .filter(|account| account_matches(account, scope))
        .map(to_report_account)
        .collect())
}

fn account_matches(account: &StoredAccount, scope: &ReportScope) -> bool {
    match scope {
        ReportScope::All => true,
        ReportScope::Kind { account_kind } => account.kind == *account_kind,
        ReportScope::Account { account_id } => account.id == *account_id,
    }
}

fn to_report_account(account: StoredAccount) -> ReportAccount {
    let suffix: String = account
        .iban
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    ReportAccount {
        id: account.id,
        label: account.label,
        kind: account.kind,
        iban_suffix: suffix,
    }
}

fn validate_requested_id(id: i64) -> Result<()> {
    if id <= 0 || id > JS_SAFE_INTEGER {
        return Err(StoreError::Parse(
            "ID účtu musí byť kladné bezpečné celé číslo JavaScriptu.".into(),
        ));
    }
    Ok(())
}

fn validate_account_ids(accounts: &[ReportAccount]) -> Result<()> {
    for account in accounts {
        validate_requested_id(account.id)?;
    }
    Ok(())
}

fn read_transactions(
    tx: &Transaction,
    accounts: &[ReportAccount],
    range: Option<ReportDateRange>,
    limits: ReportCaptureLimits,
) -> Result<Vec<ReportTransaction>> {
    if accounts.is_empty() {
        return Ok(Vec::new());
    }
    let (where_sql, values) = transaction_scope(accounts, range);
    let sql = format!(
        "SELECT t.id,t.account_id,a.kind,s.number,t.posted_date,t.tx_date,t.kind,t.amount_cents,\
         t.orig_amount_cents,t.orig_currency,t.merchant_raw,t.place,t.counterparty_name,t.counterparty_iban,\
         t.category_id,c.name,p.name,t.status,t.source,'' AS raw_block,t.note,\
         max(length(CAST(t.merchant_raw AS BLOB)),length(CAST(COALESCE(t.counterparty_name,'') AS BLOB)))+\
         length(CAST(COALESCE(t.note,'') AS BLOB))+length(CAST(a.label AS BLOB))+\
         length(CAST(COALESCE(t.place,'') AS BLOB))+length(CAST(COALESCE(c.name,'') AS BLOB))+\
         length(CAST(COALESCE(p.name,'') AS BLOB))+length(CAST(COALESCE(t.orig_currency,'') AS BLOB)) AS visible_bytes,\
         c.kind \
         FROM transactions t JOIN accounts a ON a.id=t.account_id JOIN statements s ON s.id=t.statement_id \
         LEFT JOIN categories c ON c.id=t.category_id LEFT JOIN categories p ON p.id=c.parent_id \
         {where_sql} ORDER BY t.tx_date DESC,t.id DESC"
    );
    let labels = accounts
        .iter()
        .map(|account| (account.id, account.label.clone()))
        .collect::<HashMap<_, _>>();
    let mut statement = tx.prepare(&sql)?;
    let mut cursor = statement.query(params_from_iter(values.iter()))?;
    let mut rows = Vec::new();
    let account_bytes = account_text_bytes(accounts)?;
    let mut text_bytes = checked_text_total(0, account_bytes, limits.max_text_bytes)?;
    while let Some(row) = cursor.next()? {
        enforce_row_limit(rows.len(), limits.max_transactions)?;
        let row_bytes = row_text_bytes(row)?;
        text_bytes = checked_text_total(text_bytes, row_bytes, limits.max_text_bytes)?;
        let report_row = map_report_transaction(row, &labels)?;
        rows.push(report_row);
    }
    Ok(rows)
}

fn transaction_scope(
    accounts: &[ReportAccount],
    range: Option<ReportDateRange>,
) -> (String, Vec<rusqlite::types::Value>) {
    let mut values = accounts
        .iter()
        .map(|account| rusqlite::types::Value::Integer(account.id))
        .collect::<Vec<_>>();
    let placeholders = (1..=accounts.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut conditions = vec![format!("t.account_id IN ({placeholders})")];
    if let Some(range) = range {
        let from_index = values.len() + 1;
        let to_index = values.len() + 2;
        conditions.push(format!("t.tx_date>=?{from_index}"));
        conditions.push(format!("t.tx_date<=?{to_index}"));
        values.push(rusqlite::types::Value::Text(range.from.to_string()));
        values.push(rusqlite::types::Value::Text(range.to.to_string()));
    }
    (format!("WHERE {}", conditions.join(" AND ")), values)
}

fn map_report_transaction(
    row: &rusqlite::Row,
    labels: &HashMap<i64, String>,
) -> rusqlite::Result<ReportTransaction> {
    let transaction = map_transaction(row)?;
    let account_label = labels
        .get(&transaction.account_id)
        .cloned()
        .unwrap_or_default();
    let category_kind = row
        .get::<_, Option<String>>(22)?
        .as_deref()
        .map(parse_category_kind);
    Ok(ReportTransaction {
        transaction,
        account_label,
        category_kind,
    })
}

fn map_transaction(row: &rusqlite::Row) -> rusqlite::Result<TxRow> {
    let (
        id,
        account_id,
        account_kind,
        statement_number,
        posted_date,
        tx_date,
        kind,
        amount_cents,
        orig_amount_cents,
        orig_currency,
    ) = transaction_identity(row)?;
    let (
        merchant_raw,
        place,
        counterparty_name,
        counterparty_iban,
        category_id,
        category_name,
        parent_name,
        status,
        source,
        raw_block,
        note,
    ) = transaction_detail(row)?;
    Ok(TxRow {
        id,
        account_id,
        account_kind,
        statement_number,
        posted_date,
        tx_date,
        kind,
        amount_cents,
        orig_amount_cents,
        orig_currency,
        merchant_raw,
        place,
        counterparty_name,
        counterparty_iban,
        category_id,
        category_name,
        parent_name,
        status,
        source,
        raw_block,
        note,
    })
}

type TransactionIdentity = (
    i64,
    i64,
    AccountKind,
    i64,
    NaiveDate,
    NaiveDate,
    String,
    i64,
    Option<i64>,
    Option<String>,
);

fn transaction_identity(row: &rusqlite::Row) -> rusqlite::Result<TransactionIdentity> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        parse_account_kind(&row.get::<_, String>(2)?),
        row.get(3)?,
        parse_date(row, 4)?,
        parse_date(row, 5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}

type TransactionDetail = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<String>,
    Status,
    String,
    String,
    String,
);

fn transaction_detail(row: &rusqlite::Row) -> rusqlite::Result<TransactionDetail> {
    Ok((
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
        row.get(14)?,
        row.get(15)?,
        row.get(16)?,
        crate::import::status_parse(&row.get::<_, String>(17)?),
        row.get(18)?,
        row.get(19)?,
        row.get(20)?,
    ))
}

fn parse_date(row: &rusqlite::Row, index: usize) -> rusqlite::Result<NaiveDate> {
    let value: String = row.get(index)?;
    let parsed = NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })?;
    if !(1..=9999).contains(&parsed.year()) {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "dátum transakcie používa nepodporovaný rok",
            )),
        ));
    }
    Ok(parsed)
}

fn parse_account_kind(value: &str) -> AccountKind {
    if value == "business" {
        AccountKind::Business
    } else {
        AccountKind::Personal
    }
}

fn parse_category_kind(value: &str) -> CategoryKind {
    if value == "income" {
        CategoryKind::Income
    } else {
        CategoryKind::Expense
    }
}

fn enforce_row_limit(current_len: usize, maximum: usize) -> Result<()> {
    if current_len >= maximum {
        return Err(StoreError::Parse(format!("Report prekročil limit {maximum} transakcií. Zvoľte kratšie obdobie alebo menej účtov.")));
    }
    Ok(())
}

fn account_text_bytes(accounts: &[ReportAccount]) -> Result<usize> {
    accounts.iter().try_fold(0_usize, |total, account| {
        total
            .checked_add(account.label.len())
            .and_then(|value| value.checked_add(account.iban_suffix.len()))
            .ok_or_else(|| StoreError::Parse("Veľkosť textu reportu pretiekla.".into()))
    })
}

fn row_text_bytes(row: &rusqlite::Row) -> Result<usize> {
    let bytes: i64 = row.get(21)?;
    usize::try_from(bytes)
        .map_err(|_| StoreError::Parse("Veľkosť textu reportu nie je platná.".into()))
}

fn checked_text_total(current: usize, added: usize, maximum: usize) -> Result<usize> {
    let total = current
        .checked_add(added)
        .ok_or_else(|| StoreError::Parse("Veľkosť textu reportu pretiekla.".into()))?;
    if total > maximum {
        return Err(StoreError::Parse(format!("Report prekročil limit {maximum} bajtov textu. Zvoľte kratšie obdobie alebo menej účtov.")));
    }
    Ok(total)
}

fn read_statements(tx: &Transaction, accounts: &[ReportAccount]) -> Result<Vec<StatementRange>> {
    if accounts.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = (1..=accounts.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("SELECT account_id,period_start,period_end,checksum_status FROM statements WHERE account_id IN ({placeholders}) ORDER BY account_id,period_start,period_end,id");
    let ids = accounts
        .iter()
        .map(|account| account.id)
        .collect::<Vec<_>>();
    let mut statement = tx.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(ids), |row| {
        Ok(StatementRange {
            account_id: row.get(0)?,
            from: row.get(1)?,
            to: row.get(2)?,
            checksum_status: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

fn transaction_range(rows: &[ReportTransaction]) -> Option<ReportDateRange> {
    let mut dates = rows.iter().map(|row| row.transaction.tx_date);
    let first = dates.next()?;
    Some(dates.fold(
        ReportDateRange {
            from: first,
            to: first,
        },
        |range, date| ReportDateRange {
            from: range.from.min(date),
            to: range.to.max(date),
        },
    ))
}

fn make_preview(
    request: &ReportRequest,
    accounts: Vec<ReportAccount>,
    transactions: &[ReportTransaction],
    coverage: &[super::ReportCoverage],
    range: Option<ReportDateRange>,
    unfinished_period: bool,
    clock: &ReportClock,
) -> Result<ReportPreview> {
    let transaction_count = u32::try_from(transactions.len())
        .map_err(|_| StoreError::Parse("Počet transakcií prekročil podporovaný rozsah.".into()))?;
    let latest_transaction_date = transactions.iter().map(|row| row.transaction.tx_date).max();
    let accounts_without_statements = count_coverage(coverage, |item| item.statement_count == 0)?;
    let accounts_with_gaps = count_coverage(coverage, |item| !item.gaps.is_empty())?;
    let unverified_statement_count =
        sum_coverage(coverage, |item| item.unverified_statement_count)?;
    let invalid_statement_range_count = sum_coverage(coverage, |item| item.invalid_range_count)?;
    Ok(ReportPreview {
        range,
        scope_label: scope_label(&request.scope),
        accounts,
        transaction_count,
        latest_transaction_date,
        captured_at: clock.captured_at.to_rfc3339(),
        unfinished_period,
        accounts_without_statements,
        accounts_with_gaps,
        unverified_statement_count,
        invalid_statement_range_count,
    })
}

fn count_coverage(
    items: &[super::ReportCoverage],
    predicate: impl Fn(&super::ReportCoverage) -> bool,
) -> Result<u32> {
    u32::try_from(items.iter().filter(|item| predicate(item)).count())
        .map_err(|_| StoreError::Parse("Počet účtov prekročil podporovaný rozsah.".into()))
}

fn sum_coverage(
    items: &[super::ReportCoverage],
    field: impl Fn(&super::ReportCoverage) -> u32,
) -> Result<u32> {
    items.iter().try_fold(0_u32, |total, item| {
        total
            .checked_add(field(item))
            .ok_or_else(|| StoreError::Parse("Počet výpisov prekročil podporovaný rozsah.".into()))
    })
}

fn scope_label(scope: &ReportScope) -> String {
    match scope {
        ReportScope::All => "Všetky účty".into(),
        ReportScope::Kind {
            account_kind: AccountKind::Personal,
        } => "Osobné účty".into(),
        ReportScope::Kind {
            account_kind: AccountKind::Business,
        } => "Firemné účty".into(),
        ReportScope::Account { .. } => "Vybraný účet".into(),
    }
}
