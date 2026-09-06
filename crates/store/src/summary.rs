//! Money-rules aggregates (spec 4.3, D4, D6), CSV export and the checksum
//! banner data (A3).
use crate::query::{where_clause, TxFilter};
use crate::{Result, Store};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatTotal { pub category_id: i64, pub name: String, pub parent_name: Option<String>, pub cents: i64 }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthRow { pub month: String, pub income_cents: i64, pub expense_cents: i64 }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthCat { pub month: String, pub category_id: i64, pub name: String, pub cents: i64 }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantRow { pub merchant: String, pub cents: i64, pub count: i64 }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub income_cents: i64,
    pub expense_cents: i64,
    pub transfer_cents: i64,
    pub net_cents: i64,
    pub unassigned_count: i64,
    pub suggested_count: i64,
    pub by_category: Vec<CatTotal>,
    pub by_month: Vec<MonthRow>,
    pub by_month_category: Vec<MonthCat>,
    pub top_merchants: Vec<MerchantRow>,
}

/// Statements whose closing balance does not reconcile (spec A3), for the
/// checksum banner. `TxFilter.statement_id` is the drill-down key from here
/// into `list_transactions`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BadChecksum { pub statement_id: i64, pub number: i64, pub account_label: String, pub off_by_cents: i64 }

/// SQL fragments shared by every aggregate: income and expense in cents, per
/// the money rules in the task header. Transfer rows never count. A refund
/// reduces the expense of its category because a categorized refund still
/// takes the `c.kind = 'expense'` branch with a positive amount.
const INCOME: &str = "CASE WHEN t.status <> 'transfer' AND t.amount_cents > 0 AND t.kind <> 'refund' AND (c.kind = 'income' OR c.kind IS NULL) THEN t.amount_cents ELSE 0 END";
const EXPENSE: &str = "CASE WHEN t.status <> 'transfer' AND (c.kind = 'expense' OR (c.kind IS NULL AND (t.amount_cents < 0 OR t.kind = 'refund'))) THEN -t.amount_cents ELSE 0 END";
const FROM: &str = "FROM transactions t LEFT JOIN categories c ON c.id = t.category_id LEFT JOIN categories p ON p.id = c.parent_id";

impl Store {
    pub fn summary(&self, from: Option<NaiveDate>, to: Option<NaiveDate>, account_id: Option<i64>) -> Result<Summary> {
        self.summary_filtered(from, to, account_id, None)
    }

    /// A17/F3: the Prehľad filters by account KIND, which covers every account
    /// of that kind. Picking one account per kind (what the screen used to do,
    /// because the store offered nothing else) dropped a second personal or
    /// business account out of the totals. `account_id` still narrows to a
    /// single account when a caller wants exactly that.
    /// Each aggregate is its own method (`by_category_rows`, `by_month_rows`,
    /// ...) purely to keep this function's cyclomatic complexity under the
    /// project's Lizard budget: every `?` here is a branch under that
    /// analyzer, and five inline aggregates in one function measured 20.
    pub fn summary_filtered(&self, from: Option<NaiveDate>, to: Option<NaiveDate>, account_id: Option<i64>, account_kind: Option<parser::AccountKind>) -> Result<Summary> {
        let f = TxFilter { from, to, account_id, account_kind, ..Default::default() };
        let (w, params) = where_clause(&f);
        let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let totals = self.summary_totals(&w, &refs)?;
        let by_category = self.by_category_rows(&w, &refs)?;
        let by_month = self.by_month_rows(&w, &refs)?;
        let by_month_category = self.by_month_category_rows(&w, &refs)?;
        let top_merchants = self.top_merchant_rows(&w, &refs)?;
        Ok(Summary { income_cents: totals.0, expense_cents: totals.1, transfer_cents: totals.2, net_cents: totals.0 - totals.1, unassigned_count: totals.3, suggested_count: totals.4, by_category, by_month, by_month_category, top_merchants })
    }

    fn summary_totals(&self, w: &str, refs: &[&dyn rusqlite::ToSql]) -> Result<(i64, i64, i64, i64, i64)> {
        let sql = format!("SELECT COALESCE(SUM({INCOME}),0), COALESCE(SUM({EXPENSE}),0), COALESCE(SUM(CASE WHEN t.status = 'transfer' AND t.amount_cents < 0 THEN -t.amount_cents ELSE 0 END),0), COALESCE(SUM(t.status = 'unassigned'),0), COALESCE(SUM(t.status = 'suggested'),0) {FROM}{w}");
        Ok(self.conn.query_row(&sql, refs, |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?)
    }

    fn by_category_rows(&self, w: &str, refs: &[&dyn rusqlite::ToSql]) -> Result<Vec<CatTotal>> {
        self.rows(&format!("SELECT c.id, c.name, p.name, SUM({EXPENSE}) AS cents {FROM}{w} {} GROUP BY c.id HAVING cents <> 0 ORDER BY cents DESC", and(w, "c.id IS NOT NULL")), refs, |r| Ok(CatTotal { category_id: r.get(0)?, name: r.get(1)?, parent_name: r.get(2)?, cents: r.get(3)? }))
    }

    fn by_month_rows(&self, w: &str, refs: &[&dyn rusqlite::ToSql]) -> Result<Vec<MonthRow>> {
        self.rows(&format!("SELECT substr(t.tx_date,1,7) AS m, SUM({INCOME}), SUM({EXPENSE}) {FROM}{w} GROUP BY m ORDER BY m"), refs, |r| Ok(MonthRow { month: r.get(0)?, income_cents: r.get(1)?, expense_cents: r.get(2)? }))
    }

    fn by_month_category_rows(&self, w: &str, refs: &[&dyn rusqlite::ToSql]) -> Result<Vec<MonthCat>> {
        self.rows(&format!("SELECT substr(t.tx_date,1,7) AS m, COALESCE(p.id, c.id), COALESCE(p.name, c.name), SUM({EXPENSE}) AS cents {FROM}{w} {} GROUP BY m, COALESCE(p.id, c.id) HAVING cents <> 0 ORDER BY m", and(w, "c.id IS NOT NULL")), refs, |r| Ok(MonthCat { month: r.get(0)?, category_id: r.get(1)?, name: r.get(2)?, cents: r.get(3)? }))
    }

    fn top_merchant_rows(&self, w: &str, refs: &[&dyn rusqlite::ToSql]) -> Result<Vec<MerchantRow>> {
        self.rows(&format!("SELECT t.merchant_raw, SUM({EXPENSE}) AS cents, COUNT(*) {FROM}{w} {} GROUP BY t.merchant_norm HAVING cents > 0 ORDER BY cents DESC LIMIT 10", and(w, "t.status <> 'transfer'")), refs, |r| Ok(MerchantRow { merchant: r.get(0)?, cents: r.get(1)?, count: r.get(2)? }))
    }

    fn rows<T>(&self, sql: &str, refs: &[&dyn rusqlite::ToSql], map: impl FnMut(&rusqlite::Row) -> rusqlite::Result<T>) -> Result<Vec<T>> {
        let mut st = self.conn.prepare(sql)?;
        let it = st.query_map(refs, map)?;
        Ok(it.collect::<std::result::Result<_, _>>()?)
    }

    pub fn export_csv(&self, f: &TxFilter) -> Result<String> {
        let mut out = String::from("datum;ucet;obchodnik;miesto;suma_eur;kategoria;podkategoria;stav\n");
        for r in self.list_transactions(f)? {
            out.push_str(&csv_line(&r));
        }
        Ok(out)
    }

    pub fn statements_with_bad_checksum(&self) -> Result<Vec<BadChecksum>> {
        self.rows(
            "SELECT s.id, s.number, a.label, s.checksum_off_by FROM statements s JOIN accounts a ON a.id = s.account_id WHERE s.checksum_status = 'off'",
            &[],
            |r| Ok(BadChecksum { statement_id: r.get(0)?, number: r.get(1)?, account_label: r.get(2)?, off_by_cents: r.get(3)? }),
        )
    }
}

/// A leading `= + - @`, a leading tab or carriage return, and the full-width
/// Unicode look-alikes of `= + - @` (used to slip past ASCII-only filters,
/// then normalized back to their ASCII meaning by some spreadsheets) all
/// open a formula in Excel/LibreOffice/Sheets when the file is opened (OWASP
/// CSV Injection, https://owasp.org/www-community/attacks/CSV_Injection,
/// read 2026-09-06). `merchant_raw`, `place` and `counterparty_name` come
/// from parsed PDF text, not from this app, so a crafted statement line
/// could carry one through unexamined. A leading `'` defuses every case
/// covered here by forcing the cell to a literal.
///
/// Limits, stated honestly rather than as a universal guarantee: OWASP's
/// list also names a leading line feed, which cannot reach this function (
/// `merchant_raw`/`place`/`counterparty_name` are single-line parsed
/// fields; the multiline `raw_block` is never passed through `csv_quote`).
/// And this defusing prefix is not proof against every spreadsheet forever:
/// a save/reopen cycle in some tools can drop the escaping it relies on.
/// This function only covers the leading-character injection vector; it
/// makes no claim about anything downstream of the user's own spreadsheet
/// software.
fn csv_quote(s: &str) -> String {
    const DANGEROUS_LEADING: [char; 10] = ['=', '+', '-', '@', '\t', '\r', '\u{FF1D}', '\u{FF0B}', '\u{FF0D}', '\u{FF20}'];
    let defused = match s.chars().next() {
        Some(c) if DANGEROUS_LEADING.contains(&c) => format!("'{s}"),
        _ => s.to_string(),
    };
    format!("\"{}\"", defused.replace('"', "\"\""))
}

fn csv_line(r: &crate::TxRow) -> String {
    let (cat, sub) = match (&r.parent_name, &r.category_name) {
        (Some(p), Some(c)) => (p.clone(), c.clone()),
        (None, Some(c)) => (c.clone(), String::new()),
        _ => (String::new(), String::new()),
    };
    format!(
        "{};{};{};{};{};{};{};{}\n",
        r.tx_date,
        if r.account_kind == parser::AccountKind::Business { "firemny" } else { "osobny" },
        csv_quote(&r.merchant_raw),
        csv_quote(r.place.as_deref().unwrap_or("")),
        parser::money::format_cents(r.amount_cents).replace(' ', ""),
        csv_quote(&cat),
        csv_quote(&sub),
        crate::import::status_str(r.status),
    )
}

/// `w` already starts with ` WHERE ...` when non-empty, so `and` appends
/// `AND extra`; when `w` is empty it opens with `WHERE extra`.
fn and(w: &str, extra: &str) -> String { if w.is_empty() { format!("WHERE {extra}") } else { format!("AND {extra}") } }

#[cfg(test)]
mod csv_quote_tests {
    use super::csv_quote;

    /// Direct unit coverage on the string logic itself, not routed through a
    /// PDF fixture: `merchant_raw`/`place` are trimmed by the parser before
    /// `csv_quote` ever sees them, so a leading tab or carriage return could
    /// never survive that path in a real import. Testing here is the only
    /// way to pin the defense for a future field source that isn't trimmed.
    #[test]
    fn defuses_every_ascii_marker_in_owasps_csv_injection_list() {
        for c in ['=', '+', '-', '@', '\t', '\r'] {
            let s = format!("{c}cmd");
            assert_eq!(csv_quote(&s), format!("\"'{s}\""), "must defuse a leading {c:?}");
        }
    }

    #[test]
    fn defuses_the_full_width_look_alikes_used_to_bypass_ascii_only_filters() {
        for c in ['\u{FF1D}', '\u{FF0B}', '\u{FF0D}', '\u{FF20}'] {
            let s = format!("{c}cmd");
            assert_eq!(csv_quote(&s), format!("\"'{s}\""), "must defuse a leading full-width {c:?}");
        }
    }

    #[test]
    fn an_ordinary_value_is_only_quoted_not_altered() {
        assert_eq!(csv_quote("BAUHAUS"), "\"BAUHAUS\"");
        assert_eq!(csv_quote(""), "\"\"");
        // A merchant that legitimately starts with a hyphen (e.g. a business
        // name) is still defused: OWASP's mitigation is a deliberate,
        // documented false positive, not a narrowing bug.
        assert_eq!(csv_quote("-Store"), "\"'-Store\"");
    }

    #[test]
    fn an_embedded_quote_is_still_doubled_for_valid_csv() {
        assert_eq!(csv_quote("Bob\"s Diner"), "\"Bob\"\"s Diner\"");
    }
}
