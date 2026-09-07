//! Filtered transaction listing (`TxFilter` over numbered SQL parameters).
use crate::import::{checksum_from_cols, status_parse};
use crate::{Result, Store};
use chrono::NaiveDate;
use parser::fold::fold;
use parser::{AccountKind, Checksum};
use rules::Status;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TxFilter {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub account_id: Option<i64>,
    /// A17/F3: every account of that kind, not one of them. Absent in older UI
    /// payloads, hence `serde(default)`.
    #[serde(default)]
    pub account_kind: Option<AccountKind>,
    pub category_id: Option<i64>,
    pub status: Option<Status>,
    pub text: Option<String>,
    pub statement_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxRow {
    pub id: i64,
    pub account_id: i64,
    pub account_kind: AccountKind,
    pub statement_number: i64,
    pub posted_date: NaiveDate,
    pub tx_date: NaiveDate,
    pub kind: String,
    pub amount_cents: i64,
    pub orig_amount_cents: Option<i64>,
    pub orig_currency: Option<String>,
    pub merchant_raw: String,
    pub place: Option<String>,
    pub counterparty_name: Option<String>,
    pub counterparty_iban: Option<String>,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub parent_name: Option<String>,
    pub status: Status,
    pub source: String,
    pub raw_block: String,
    pub note: String,
}

/// Row for the Import screen's "Posledné importy" list (spec: newest first).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentStatement {
    pub statement_id: i64,
    pub number: i64,
    pub period_end: NaiveDate,
    pub account_label: String,
    pub transaction_count: i64,
    pub checksum: Checksum,
}

pub(crate) const BASE_SELECT: &str = "SELECT t.id, t.account_id, a.kind, s.number, t.posted_date, t.tx_date, t.kind, t.amount_cents, t.orig_amount_cents, t.orig_currency, t.merchant_raw, t.place, t.counterparty_name, t.counterparty_iban, t.category_id, c.name, p.name, t.status, t.source, t.raw_block, t.note FROM transactions t JOIN accounts a ON a.id = t.account_id JOIN statements s ON s.id = t.statement_id LEFT JOIN categories c ON c.id = t.category_id LEFT JOIN categories p ON p.id = c.parent_id";

fn push_param(params: &mut Vec<Box<dyn rusqlite::ToSql>>, p: Box<dyn rusqlite::ToSql>) -> usize {
    params.push(p);
    params.len()
}

/// Builds a ` WHERE ...` fragment with numbered params (`?1`, `?2`, ...).
/// Rusqlite/SQLite allow reusing a numbered parameter, which the category
/// and text conditions do (one bound value referenced twice or thrice).
pub(crate) fn where_clause(f: &TxFilter) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut conds: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(d) = f.from {
        let k = push_param(&mut params, Box::new(d.to_string()));
        conds.push(format!("t.tx_date >= ?{k}"));
    }
    if let Some(d) = f.to {
        let k = push_param(&mut params, Box::new(d.to_string()));
        conds.push(format!("t.tx_date <= ?{k}"));
    }
    if let Some(a) = f.account_id {
        let k = push_param(&mut params, Box::new(a));
        conds.push(format!("t.account_id = ?{k}"));
    }
    if let Some(s) = f.statement_id {
        let k = push_param(&mut params, Box::new(s));
        conds.push(format!("t.statement_id = ?{k}"));
    }
    if let Some(kind) = f.account_kind {
        // A subquery, not `a.kind`: this fragment is shared with the summary
        // aggregates, whose FROM clause does not join `accounts`.
        let k = push_param(&mut params, Box::new(crate::accounts::kind_str(kind).to_string()));
        conds.push(format!("t.account_id IN (SELECT id FROM accounts WHERE kind = ?{k})"));
    }
    if let Some(c) = f.category_id {
        let k = push_param(&mut params, Box::new(c));
        conds.push(format!("(t.category_id = ?{k} OR c.parent_id = ?{k})"));
    }
    if let Some(s) = f.status {
        let k = push_param(&mut params, Box::new(crate::import::status_str(s).to_string()));
        conds.push(format!("t.status = ?{k}"));
    }
    // `text` is deliberately NOT a SQL condition here: 0.1.2 extends the
    // search to `note`, and the contract wants fold()-based (diacritic- and
    // case-insensitive) LITERAL substring matching, where `%`/`_` are plain
    // characters, not SQL wildcards. SQLite's own LOWER()/LIKE cannot do
    // Unicode folding without an ICU build, so `filter_by_text` does this
    // match in Rust after the other conditions have narrowed the rows.
    let sql = if conds.is_empty() { String::new() } else { format!(" WHERE {}", conds.join(" AND ")) };
    (sql, params)
}

impl Store {
    pub fn list_transactions(&self, f: &TxFilter) -> Result<Vec<TxRow>> {
        let (w, params) = where_clause(f);
        let mut st = self.conn.prepare(&format!("{BASE_SELECT}{w} ORDER BY t.tx_date DESC, t.id DESC"))?;
        let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = st.query_map(refs.as_slice(), row_to_tx)?;
        let rows: Vec<TxRow> = rows.collect::<std::result::Result<_, _>>()?;
        Ok(match &f.text {
            Some(needle) => filter_by_text(rows, needle),
            None => rows,
        })
    }

    /// Newest-first statements for the Import screen's history section, with
    /// their transaction count and checksum badge; `limit` bounds the list
    /// (Settings passes a large limit to read the total count off `.len()`
    /// instead of a dedicated counting command).
    pub fn recent_statements(&self, limit: usize) -> Result<Vec<RecentStatement>> {
        let mut st = self.conn.prepare(
            "SELECT s.id, s.number, s.period_end, a.label, COUNT(t.id), s.checksum_status, s.checksum_off_by \
             FROM statements s \
             JOIN accounts a ON a.id = s.account_id \
             LEFT JOIN transactions t ON t.statement_id = s.id \
             GROUP BY s.id \
             ORDER BY s.imported_at DESC, s.id DESC \
             LIMIT ?1",
        )?;
        let rows = st.query_map([limit as i64], |r| {
            let period_end: String = r.get(2)?;
            let status: String = r.get(5)?;
            let off_by: Option<i64> = r.get(6)?;
            Ok(RecentStatement {
                statement_id: r.get(0)?,
                number: r.get(1)?,
                period_end: NaiveDate::parse_from_str(&period_end, "%Y-%m-%d")
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e)))?,
                account_label: r.get(3)?,
                transaction_count: r.get(4)?,
                checksum: checksum_from_cols(&status, off_by),
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

/// Literal substring on `fold()`ed text: no SQL wildcards, no diacritics, no
/// case. Empty search text matches everything (same as the old `LIKE
/// '%'||''||'%'` it replaces), so the early return is a fast path, not a
/// behaviour change.
fn filter_by_text(rows: Vec<TxRow>, needle: &str) -> Vec<TxRow> {
    let needle = fold(needle);
    if needle.is_empty() {
        return rows;
    }
    rows.into_iter().filter(|r| row_matches_text(r, &needle)).collect()
}

fn row_matches_text(r: &TxRow, folded_needle: &str) -> bool {
    fold(&r.merchant_raw).contains(folded_needle)
        || fold(&r.note).contains(folded_needle)
        || r.place.as_deref().is_some_and(|s| fold(s).contains(folded_needle))
        || r.counterparty_name.as_deref().is_some_and(|s| fold(s).contains(folded_needle))
}

// A defaulted 1970 date would hide a corrupt row behind a plausible-looking
// date (adjudicated review finding); surface the parse failure as a real
// SQLite error instead.
fn parse_row_date(r: &rusqlite::Row, i: usize) -> rusqlite::Result<NaiveDate> {
    let s: String = r.get(i)?;
    NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|e| rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e)))
}

/// `(id, account_id, account_kind, statement_number, posted_date, tx_date, kind, amount_cents, orig_amount_cents, orig_currency)`,
/// columns 0-9 of `BASE_SELECT`.
type TxIdentity = (i64, i64, AccountKind, i64, NaiveDate, NaiveDate, String, i64, Option<i64>, Option<String>);

fn row_identity(r: &rusqlite::Row) -> rusqlite::Result<TxIdentity> {
    let kind_s: String = r.get(2)?;
    Ok((
        r.get(0)?,
        r.get(1)?,
        if kind_s == "business" { AccountKind::Business } else { AccountKind::Personal },
        r.get(3)?,
        parse_row_date(r, 4)?,
        parse_row_date(r, 5)?,
        r.get(6)?,
        r.get(7)?,
        r.get(8)?,
        r.get(9)?,
    ))
}

/// `(merchant_raw, place, counterparty_name, counterparty_iban, category_id, category_name, parent_name, status, source, raw_block, note)`,
/// columns 10-20 of `BASE_SELECT`.
type TxDetail = (String, Option<String>, Option<String>, Option<String>, Option<i64>, Option<String>, Option<String>, Status, String, String, String);

fn row_detail(r: &rusqlite::Row) -> rusqlite::Result<TxDetail> {
    Ok((
        r.get(10)?,
        r.get(11)?,
        r.get(12)?,
        r.get(13)?,
        r.get(14)?,
        r.get(15)?,
        r.get(16)?,
        status_parse(&r.get::<_, String>(17)?),
        r.get(18)?,
        r.get(19)?,
        r.get(20)?,
    ))
}

/// Split into `row_identity`/`row_detail` (and `parse_row_date`) purely to
/// keep each function's cyclomatic complexity under the project's Lizard
/// budget: every `?` here is a branch under that analyzer, and one flat
/// function over all 20 `BASE_SELECT` columns measured 23.
fn row_to_tx(r: &rusqlite::Row) -> rusqlite::Result<TxRow> {
    let (id, account_id, account_kind, statement_number, posted_date, tx_date, kind, amount_cents, orig_amount_cents, orig_currency) = row_identity(r)?;
    let (merchant_raw, place, counterparty_name, counterparty_iban, category_id, category_name, parent_name, status, source, raw_block, note) = row_detail(r)?;
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
