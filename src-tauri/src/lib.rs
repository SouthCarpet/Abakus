//! Abakus Tauri shell.
pub mod commands;
pub mod import_flow;
pub mod secrets;
pub mod state;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Spec D9 promises %LOCALAPPDATA%\Abakus, not the identifier-derived Tauri path (review fix 2026-08-30).
            let base = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from).ok_or("LOCALAPPDATA unset")?;
            app.manage(state::AppState::open(base.join("Abakus"))?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_statements,
            commands::import_with_password,
            commands::list_accounts,
            commands::save_account,
            commands::clear_password,
            commands::list_categories,
            commands::save_category,
            commands::archive_category,
            commands::list_rules,
            commands::delete_rule,
            commands::list_transactions,
            commands::assign,
            commands::confirm,
            commands::summary,
            commands::export_csv,
            commands::bad_checksums,
            commands::data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("abakus failed to start");
}
