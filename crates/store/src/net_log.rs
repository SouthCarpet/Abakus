//! Outbound network audit log (spec A14b): every request `net::audited_get`
//! makes, plus every non-localhost endpoint `net::sample_connections`
//! observes, land here so Nastavenia can show exactly what left the machine.
use crate::{Result, Store};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetLogRow {
    pub id: i64,
    pub started_at: DateTime<Utc>,
    pub url: String,
    pub status: String,
    pub duration_ms: i64,
    pub bytes_in: i64,
}

/// The log is an audit trail, not a growing record: past this many rows the
/// oldest are dropped so it can never become an unbounded local dataset.
const MAX_ROWS: i64 = 1000;

impl Store {
    pub fn append_net_log(&mut self, started_at: DateTime<Utc>, url: &str, status: &str, duration_ms: i64, bytes_in: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO net_log (started_at, url, status, duration_ms, bytes_in) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![started_at.to_rfc3339(), url, status, duration_ms, bytes_in],
        )?;
        tx.execute("DELETE FROM net_log WHERE id NOT IN (SELECT id FROM net_log ORDER BY id DESC LIMIT ?1)", [MAX_ROWS])?;
        tx.commit()?;
        Ok(())
    }

    /// Newest first, capped at `limit` rows (Nastavenia shows the recent tail).
    pub fn net_log(&self, limit: usize) -> Result<Vec<NetLogRow>> {
        let mut st = self.conn.prepare("SELECT id, started_at, url, status, duration_ms, bytes_in FROM net_log ORDER BY id DESC LIMIT ?1")?;
        let rows = st.query_map([limit as i64], row_to_net_log)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

fn row_to_net_log(r: &rusqlite::Row) -> rusqlite::Result<NetLogRow> {
    let started_at: String = r.get(1)?;
    Ok(NetLogRow {
        id: r.get(0)?,
        started_at: DateTime::parse_from_rfc3339(&started_at)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e)))?,
        url: r.get(2)?,
        status: r.get(3)?,
        duration_ms: r.get(4)?,
        bytes_in: r.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_log_reads_back_empty() {
        let store = Store::open_in_memory().unwrap();
        assert!(store.net_log(10).unwrap().is_empty());
    }

    #[test]
    fn append_then_read_round_trips_newest_first() {
        let mut store = Store::open_in_memory().unwrap();
        let t1 = DateTime::parse_from_rfc3339("2026-08-31T10:00:00Z").unwrap().with_timezone(&Utc);
        let t2 = DateTime::parse_from_rfc3339("2026-08-31T10:00:05Z").unwrap().with_timezone(&Utc);
        store.append_net_log(t1, "https://api.github.com/repos/SouthCarpet/Abakus/releases/latest", "200", 120, 512).unwrap();
        store.append_net_log(t2, "observed: 1.2.3.4:443", "observed", 0, 0).unwrap();

        let rows = store.net_log(10).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].url, "observed: 1.2.3.4:443");
        assert_eq!(rows[0].status, "observed");
        assert_eq!(rows[1].status, "200");
        assert_eq!(rows[1].bytes_in, 512);
        assert_eq!(rows[1].started_at, t1);
    }

    #[test]
    fn limit_caps_the_returned_rows() {
        let mut store = Store::open_in_memory().unwrap();
        let t = DateTime::parse_from_rfc3339("2026-08-31T10:00:00Z").unwrap().with_timezone(&Utc);
        for i in 0..5 { store.append_net_log(t, &format!("url{i}"), "200", 1, 1).unwrap(); }
        assert_eq!(store.net_log(2).unwrap().len(), 2);
    }

    #[test]
    fn append_trims_the_table_to_the_newest_max_rows() {
        let mut store = Store::open_in_memory().unwrap();
        let t = DateTime::parse_from_rfc3339("2026-08-31T10:00:00Z").unwrap().with_timezone(&Utc);
        for i in 0..(MAX_ROWS + 5) { store.append_net_log(t, &format!("url{i}"), "200", 1, 1).unwrap(); }

        let rows = store.net_log((MAX_ROWS + 5) as usize).unwrap();
        assert_eq!(rows.len() as i64, MAX_ROWS, "the table must never hold more than the cap");
        assert_eq!(rows[0].url, format!("url{}", MAX_ROWS + 4), "the newest row must survive the trim");
        assert!(!rows.iter().any(|r| r.url == "url0"), "the oldest rows must be the ones dropped");
    }
}
