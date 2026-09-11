//! 0.1.2: `statement_history`, the per-statement ledger behind the new
//! history screen. Read-only, and deliberately separate from
//! `query::recent_statements` (newest-first, capped, used by the Import
//! screen): this one returns every statement matching the filters, oldest
//! first per account, with no limit.
use crate::import::checksum_from_cols;
use crate::{Result, Store};
use chrono::NaiveDate;
use parser::{AccountKind, Checksum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatementHistoryRow {
    pub statement_id: i64,
    pub account_id: i64,
    pub account_label: String,
    pub account_kind: AccountKind,
    pub number: i64,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_cents: Option<i64>,
    pub closing_cents: Option<i64>,
    /// Retained transactions whose stored owner is this statement. A
    /// re-export can report zero after fingerprint deduplication keeps the
    /// rows owned by the first import.
    pub transaction_count: i64,
    /// Signed sum of the same retained transaction rows in integer cents.
    pub total_cents: i64,
    pub checksum: Checksum,
}

impl Store {
    /// Both filters are optional and independent (AND when both are given):
    /// `account_id` narrows to one account, `account_kind` to every account
    /// of that kind. Sorted account_id, period_start, period_end,
    /// statement_id ascending, so a re-export under a new file hash (which
    /// gets its own statement row, see `schema.sql`'s note on `file_hash`)
    /// still lands next to the statement it duplicates.
    pub fn statement_history(&self, account_id: Option<i64>, account_kind: Option<AccountKind>) -> Result<Vec<StatementHistoryRow>> {
        let (where_sql, params) = history_where(account_id, account_kind);
        let sql = format!(
            "SELECT s.id, s.account_id, a.label, a.kind, s.number, s.period_start, s.period_end, s.opening_cents, s.closing_cents, \
                    (SELECT COUNT(*) FROM transactions t WHERE t.statement_id = s.id), \
                    COALESCE((SELECT SUM(t.amount_cents) FROM transactions t WHERE t.statement_id = s.id), 0), \
                    s.checksum_status, s.checksum_off_by \
             FROM statements s JOIN accounts a ON a.id = s.account_id{where_sql} \
             ORDER BY s.account_id, s.period_start, s.period_end, s.id"
        );
        let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(refs.as_slice(), row_to_history)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

fn history_where(account_id: Option<i64>, account_kind: Option<AccountKind>) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut conds = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(id) = account_id {
        params.push(Box::new(id));
        conds.push(format!("s.account_id = ?{}", params.len()));
    }
    if let Some(kind) = account_kind {
        params.push(Box::new(crate::accounts::kind_str(kind).to_string()));
        conds.push(format!("a.kind = ?{}", params.len()));
    }
    let where_sql = if conds.is_empty() { String::new() } else { format!(" WHERE {}", conds.join(" AND ")) };
    (where_sql, params)
}

fn parse_date(r: &rusqlite::Row, i: usize) -> rusqlite::Result<NaiveDate> {
    let s: String = r.get(i)?;
    NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|e| rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e)))
}

/// `(statement_id, account_id, account_label, account_kind, number)`, columns
/// 0-4. Split from `row_to_history` purely to keep each function's
/// cyclomatic complexity under the project's Lizard budget, the same reason
/// `query.rs` splits `row_to_tx` into `row_identity`/`row_detail`.
type HistoryIdentity = (i64, i64, String, AccountKind, i64);

fn row_identity(r: &rusqlite::Row) -> rusqlite::Result<HistoryIdentity> {
    let kind_s: String = r.get(3)?;
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, if kind_s == "business" { AccountKind::Business } else { AccountKind::Personal }, r.get(4)?))
}

fn row_to_history(r: &rusqlite::Row) -> rusqlite::Result<StatementHistoryRow> {
    let (statement_id, account_id, account_label, account_kind, number) = row_identity(r)?;
    let status: String = r.get(11)?;
    let off_by: Option<i64> = r.get(12)?;
    Ok(StatementHistoryRow {
        statement_id,
        account_id,
        account_label,
        account_kind,
        number,
        period_start: parse_date(r, 5)?,
        period_end: parse_date(r, 6)?,
        opening_cents: r.get(7)?,
        closing_cents: r.get(8)?,
        transaction_count: r.get(9)?,
        total_cents: r.get(10)?,
        checksum: checksum_from_cols(&status, off_by),
    })
}

#[cfg(test)]
mod tests {
    use crate::Store;

    #[test]
    fn no_filters_returns_every_statement_empty_when_none_exist() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.statement_history(None, None).unwrap(), Vec::new());
    }
}
