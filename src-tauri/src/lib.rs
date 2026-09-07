//! Abakus Tauri shell.
pub mod commands;
pub mod import_flow;
pub mod net;
pub mod secrets;
pub mod state;
pub mod update;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Spec D9 promises %LOCALAPPDATA%\Abakus, not the identifier-derived Tauri path (review fix 2026-08-30).
            let base = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from).ok_or("LOCALAPPDATA unset")?;
            let state = state::AppState::open(base.join("Abakus"))?;
            // Spec A14b: the leak sampler runs once at app start regardless of
            // the update-check opt-in, since it is a hard local-only guarantee,
            // not part of the opt-in feature.
            // A17/F7: the sampler's own failure is kept too, for the same
            // reason its rows are: an audit step that can fail in silence is
            // not an audit step.
            match state.store.lock() {
                Ok(mut s) => {
                    if let Err(e) = net::sample_connections(&mut s) {
                        net::record_audit_failure("startup: sample_connections", e);
                    }
                }
                Err(_) => net::record_audit_failure("startup: sample_connections", "store lock poisoned".to_string()),
            }
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_statements,
            commands::import_with_password,
            commands::list_accounts,
            commands::save_account,
            commands::update_account,
            commands::clear_password,
            commands::account_delete_preview,
            commands::delete_account,
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
            commands::recent_statements,
            commands::statement_history,
            commands::save_transaction_note,
            commands::backup_database,
            commands::statement_delete_preview,
            commands::delete_statement,
            commands::get_check_updates,
            commands::set_check_updates,
            commands::check_update_now,
            commands::net_log,
            commands::net_audit_failures,
            commands::run_net_audit,
        ])
        .run(tauri::generate_context!())
        .expect("abakus failed to start");
}
