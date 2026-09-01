//! Generic key/value settings (schema.sql `settings` table). Only
//! `check_updates` (spec A14) lives here today: absent means opted out, so
//! every existing database keeps its current opt-out behaviour untouched.
use crate::{Result, Store};

const CHECK_UPDATES_KEY: &str = "check_updates";

impl Store {
    pub fn get_check_updates(&self) -> Result<bool> {
        let value: Option<String> = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [CHECK_UPDATES_KEY], |r| r.get(0))
            .ok();
        Ok(value.as_deref() == Some("1"))
    }

    pub fn set_check_updates(&mut self, on: bool) -> Result<()> {
        self.set_setting(CHECK_UPDATES_KEY, if on { "1" } else { "0" })
    }

    pub(crate) fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self.conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).ok())
    }

    pub(crate) fn set_setting(&mut self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_off() {
        let store = Store::open_in_memory().unwrap();
        assert!(!store.get_check_updates().unwrap(), "an absent setting must read as opted out");
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut store = Store::open_in_memory().unwrap();
        store.set_check_updates(true).unwrap();
        assert!(store.get_check_updates().unwrap());
        store.set_check_updates(false).unwrap();
        assert!(!store.get_check_updates().unwrap());
    }
}
