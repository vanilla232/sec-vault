pub mod commands;
pub mod crypto;
pub mod errors;
pub mod generator;
pub mod state;
pub mod vault;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let vault_path = data_dir.join("vault.enc");
            app.manage(state::AppState::new(vault_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::initialize_vault,
            commands::unlock_vault,
            commands::lock_vault,
            commands::list_entries,
            commands::create_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::reveal_entry_password,
            commands::generate_password,
            commands::copy_to_clipboard,
            commands::destroy_vault,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}