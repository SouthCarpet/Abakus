//! Generic key/value settings (schema.sql `settings` table). Only
//! `check_updates` (spec A14) lives here today: absent means opted out, so
//! every existing database keeps its current opt-out behaviour untouched.
use crate::{Result, Store};
use rusqlite::OptionalExtension;

const CHECK_UPDATES_KEY: &str = "check_updates";

impl Store {
    pub fn get_check_updates(&self) -> Result<bool> {
        let value: Option<String> = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [CHECK_UPDATES_KEY], |r| r.get(0))
            .optional()?;
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

    /// Plan 091 point 5: a saved choice survives closing and reopening the same DB.
    #[test]
    fn saved_choices_survive_store_close_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preference.db");
        let mut store = Store::open(&path).unwrap();
        store.set_check_updates(true).unwrap();
        drop(store);
        let mut reopened = Store::open(&path).unwrap();
        assert!(reopened.get_check_updates().unwrap());
        reopened.set_check_updates(false).unwrap();
        drop(reopened);
        let reopened = Store::open(&path).unwrap();
        assert!(!reopened.get_check_updates().unwrap());
    }

    /// An unreadable setting is unknown, not a successful opt-out read.
    #[test]
    fn unreadable_settings_return_an_error_instead_of_false() {
        let store = Store::open_in_memory().unwrap();
        store.conn.execute_batch("DROP TABLE settings").unwrap();
        assert!(store.get_check_updates().is_err());
    }
}
