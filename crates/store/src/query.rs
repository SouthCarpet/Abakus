//! Filtered transaction listing (`TxFilter` over numbered SQL parameters).
use crate::import::status_parse;
use crate::{Result, Store};
use chrono::NaiveDate;
use parser::AccountKind;
use rules::Status;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TxFilter {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub account_id: Option<i64>,
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
}

pub(crate) const BASE_SELECT: &str = "SELECT t.id, t.account_id, a.kind, s.number, t.posted_date, t.tx_date, t.kind, t.amount_cents, t.orig_amount_cents, t.orig_currency, t.merchant_raw, t.place, t.counterparty_name, t.counterparty_iban, t.category_id, c.name, p.name, t.status, t.source, t.raw_block FROM transactions t JOIN accounts a ON a.id = t.account_id JOIN statements s ON s.id = t.statement_id LEFT JOIN categories c ON c.id = t.category_id LEFT JOIN categories p ON p.id = c.parent_id";

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
    if let Some(c) = f.category_id {
        let k = push_param(&mut params, Box::new(c));
        conds.push(format!("(t.category_id = ?{k} OR c.parent_id = ?{k})"));
    }
    if let Some(s) = f.status {
        let k = push_param(&mut params, Box::new(crate::import::status_str(s).to_string()));
        conds.push(format!("t.status = ?{k}"));
    }
    if let Some(t) = &f.text {
        let k = push_param(&mut params, Box::new(t.clone()));
        conds.push(format!("(t.merchant_raw LIKE '%' || ?{k} || '%' OR t.counterparty_name LIKE '%' || ?{k} || '%' OR t.place LIKE '%' || ?{k} || '%')"));
    }
    let sql = if conds.is_empty() { String::new() } else { format!(" WHERE {}", conds.join(" AND ")) };
    (sql, params)
}

impl Store {
    pub fn list_transactions(&self, f: &TxFilter) -> Result<Vec<TxRow>> {
        let (w, params) = where_clause(f);
        let mut st = self.conn.prepare(&format!("{BASE_SELECT}{w} ORDER BY t.tx_date DESC, t.id DESC"))?;
        let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = st.query_map(refs.as_slice(), row_to_tx)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

fn row_to_tx(r: &rusqlite::Row) -> rusqlite::Result<TxRow> {
    // A defaulted 1970 date would hide a corrupt row behind a plausible-looking date
    // (adjudicated review finding); surface the parse failure as a real SQLite error instead.
    let date = |i: usize| -> rusqlite::Result<NaiveDate> {
        let s: String = r.get(i)?;
        NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|e| rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e)))
    };
    let kind_s: String = r.get(2)?;
    Ok(TxRow {
        id: r.get(0)?,
        account_id: r.get(1)?,
        account_kind: if kind_s == "business" { AccountKind::Business } else { AccountKind::Personal },
        statement_number: r.get(3)?,
        posted_date: date(4)?,
        tx_date: date(5)?,
        kind: r.get(6)?,
        amount_cents: r.get(7)?,
        orig_amount_cents: r.get(8)?,
        orig_currency: r.get(9)?,
        merchant_raw: r.get(10)?,
        place: r.get(11)?,
        counterparty_name: r.get(12)?,
        counterparty_iban: r.get(13)?,
        category_id: r.get(14)?,
        category_name: r.get(15)?,
        parent_name: r.get(16)?,
        status: status_parse(&r.get::<_, String>(17)?),
        source: r.get(18)?,
        raw_block: r.get(19)?,
    })
}
