//! Shared Tauri app state: one SQLite store per process, guarded by a mutex.
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use store::Store;

pub struct AppState {
    pub store: Mutex<Store>,
    pub data_dir: std::path::PathBuf,
    /// 091/B10: the live database file `store` currently has open. Needed
    /// separately from `data_dir` because a restore has to rename this exact
    /// path, not just "some file under the data directory".
    pub db_path: std::path::PathBuf,
    /// 091/B10: set for the duration of any import command, so a restore can
    /// refuse immediately with a clear reason instead of silently blocking
    /// on the store mutex until the import finishes.
    pub import_in_progress: AtomicBool,
}

impl AppState {
    pub fn open(dir: std::path::PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let db_path = dir.join("abakus.db");
        let store = Store::open(&db_path).map_err(|e| e.to_string())?;
        Ok(Self { store: Mutex::new(store), data_dir: dir, db_path, import_in_progress: AtomicBool::new(false) })
    }
}
