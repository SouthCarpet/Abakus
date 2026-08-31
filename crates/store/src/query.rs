//! Minimal transaction listing for Task 9; Task 10 replaces this with real
//! filtering (`where_clause` over `TxFilter`).
use crate::import::status_parse;
use crate::{Result, Store};
use rules::Status;

#[derive(Debug, Clone, Default)]
pub struct TxFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct TxRow { pub id: i64, pub merchant_raw: String, pub status: Status, pub category_name: Option<String> }

impl Store {
    /// Every transaction, unfiltered (`_filter` is ignored until Task 10).
    pub fn list_transactions(&self, _filter: &TxFilter) -> Result<Vec<TxRow>> {
        let mut st = self.conn.prepare("SELECT t.id, t.merchant_raw, t.status, c.name FROM transactions t LEFT JOIN categories c ON c.id = t.category_id ORDER BY t.id")?;
        let rows = st.query_map([], |r| Ok(TxRow { id: r.get(0)?, merchant_raw: r.get(1)?, status: status_parse(&r.get::<_, String>(2)?), category_name: r.get(3)? }))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}
