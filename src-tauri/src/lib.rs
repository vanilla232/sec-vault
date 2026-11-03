pub mod commands;
pub mod crypto;
pub mod errors;
pub mod generator;
pub mod state;
pub mod vault;

use tauri::Manager;
#[cfg(windows)]
use tauri::Emitter;

#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::System::Diagnostics::Debug::{
    SetErrorMode, SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX,
};
#[cfg(windows)]
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_READOBJECTS,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    unsafe {
        // Запрещает создание дампов памяти и окон с ошибкой при краше
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
    }

    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let vault_path = data_dir.join("vault.enc");
            app.manage(state::AppState::new(vault_path));

            #[cfg(windows)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    if let Ok(hwnd) = window.hwnd() {
                        unsafe {
                            let _ = SetWindowDisplayAffinity(
                                HWND(hwnd.0 as *mut _),
                                WDA_EXCLUDEFROMCAPTURE,
                            );
                        }
                    }
                }

                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    let mut was_locked = false;
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        
                        let hdesk = unsafe { 
                            OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) 
                        };
                        let is_locked = hdesk.is_err();
                        
                        if let Ok(h) = hdesk {
                            let _ = unsafe { CloseDesktop(h) };
                        }

                        if is_locked && !was_locked {
                            was_locked = true;
                            if let Some(app_state) = app_handle.try_state::<state::AppState>() {
                                if app_state.is_unlocked() {
                                    let _ = app_state.take_session();
                                    let _ = app_handle.emit("vault-locked", ());
                                }
                            }
                        } else if !is_locked {
                            was_locked = false;
                        }
                    }
                });
            }

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
            commands::set_window_content_protection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}