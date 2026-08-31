//! Shared Tauri app state: one SQLite store per process, guarded by a mutex.
use std::sync::Mutex;
use store::Store;

pub struct AppState {
    pub store: Mutex<Store>,
    pub data_dir: std::path::PathBuf,
}

impl AppState {
    pub fn open(dir: std::path::PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let store = Store::open(&dir.join("abakus.db")).map_err(|e| e.to_string())?;
        Ok(Self { store: Mutex::new(store), data_dir: dir })
    }
}
