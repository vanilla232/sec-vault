use serde::Serialize;
use std::thread;
use std::time::Duration;
use secrecy::{ExposeSecret, SecretString};
use tauri::State;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{HANDLE, HWND};
#[cfg(windows)]
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
#[cfg(windows)]
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
};

use crate::crypto::KdfParams;
use crate::errors::AppError;
use crate::generator::{self, GenOptions};
use crate::state::AppState;
use crate::vault::{Entry, Session};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub exists: bool,
    pub unlocked: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntryMeta {
    pub id: String,
    pub title: String,
    pub username: String,
    pub url: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Entry> for EntryMeta {
    fn from(e: &Entry) -> Self {
        Self {
            id: e.id.clone(),
            title: e.title.clone(),
            username: e.username.clone(),
            url: e.url.clone(),
            note: e.note.clone(),
            created_at: e.created_at.clone(),
            updated_at: e.updated_at.clone(),
        }
    }
}

#[tauri::command]
pub fn vault_status(state: State<'_, AppState>) -> VaultStatus {
    VaultStatus {
        exists: state.has_vault(),
        unlocked: state.is_unlocked(),
    }
}

#[tauri::command]
pub fn initialize_vault(
    state: State<'_, AppState>,
    master_password: SecretString,
) -> Result<(), AppError> {
    validate_master_password(master_password.expose_secret())?;
    if state.has_vault() {
        return Err(AppError::InvalidInput("vault already exists".into()));
    }
    let session = Session::create(
        state.vault_path.clone(),
        master_password.expose_secret(),
        KdfParams::default(),
    )?;
    state.set_session(session)?;
    Ok(())
}

#[tauri::command]
pub fn unlock_vault(
    state: State<'_, AppState>,
    master_password: SecretString,
) -> Result<(), AppError> {
    if !state.has_vault() {
        return Err(AppError::NotInitialized);
    }
    if state.is_unlocked() {
        return Err(AppError::AlreadyUnlocked);
    }
    let session = Session::unlock(state.vault_path.clone(), master_password.expose_secret())?;
    state.set_session(session)?;
    Ok(())
}

#[tauri::command]
pub fn lock_vault(state: State<'_, AppState>) -> Result<(), AppError> {
    let _ = state.take_session();
    Ok(())
}

#[tauri::command]
pub fn list_entries(state: State<'_, AppState>) -> Result<Vec<EntryMeta>, AppError> {
    state.with_session(|session| {
        Ok(session.list_entries().iter().map(EntryMeta::from).collect())
    })
}

#[tauri::command]
pub fn create_entry(
    state: State<'_, AppState>,
    title: String,
    username: String,
    password: SecretString,
    url: String,
    note: String,
) -> Result<EntryMeta, AppError> {
    state.with_session_mut(|session| {
        let entry = session.add_entry(title, username, password, url, note)?;
        Ok(EntryMeta::from(&entry))
    })
}

#[tauri::command]
pub fn update_entry(
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    username: Option<String>,
    password: Option<SecretString>,
    url: Option<String>,
    note: Option<String>,
) -> Result<EntryMeta, AppError> {
    state.with_session_mut(|session| {
        let entry = session.update_entry(&id, title, username, password, url, note)?;
        Ok(EntryMeta::from(&entry))
    })
}

#[tauri::command]
pub fn delete_entry(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    state.with_session_mut(|session| session.delete_entry(&id).map_err(AppError::from))
}

#[tauri::command]
pub fn reveal_entry_password(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, AppError> {
    state.with_session(|session| Ok(session.reveal_password(&id)?.to_string()))
}

#[tauri::command]
pub fn generate_password(opts: GenOptions) -> Result<String, AppError> {
    generator::generate(&opts).map_err(AppError::InvalidInput)
}

#[tauri::command]
pub fn copy_to_clipboard(text: String, clear_after_secs: u64) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        write_clipboard_private(&text)?;
    }

    #[cfg(not(windows))]
    {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| AppError::InvalidInput(e.to_string()))?;
        clipboard
            .set_text(text.clone())
            .map_err(|e| AppError::InvalidInput(e.to_string()))?;
    }

    if clear_after_secs > 0 {
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(clear_after_secs));
            if let Ok(mut cb) = arboard::Clipboard::new() {
                if let Ok(current) = cb.get_text() {
                    if current == text {
                        let _ = cb.set_text("");
                    }
                }
            }
        });
    }

    Ok(())
}

#[cfg(windows)]
fn write_clipboard_private(text: &str) -> Result<(), AppError> {
    let mut utf16: Vec<u16> = text.encode_utf16().collect();
    utf16.push(0);

    unsafe {
        OpenClipboard(None).map_err(|e| AppError::InvalidInput(format!("OpenClipboard: {e}")))?;
        
        // Оборачиваем в замыкание, чтобы гарантировать закрытие буфера при любой ошибке
        let result = (|| -> Result<(), AppError> {
            let _ = EmptyClipboard();

            let text_mem = GlobalAlloc(GMEM_MOVEABLE, utf16.len() * std::mem::size_of::<u16>())
                .map_err(|e| AppError::InvalidInput(format!("GlobalAlloc text: {e}")))?;
            let dst = GlobalLock(text_mem);
            if dst.is_null() {
                return Err(AppError::InvalidInput("GlobalLock text".into()));
            }
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), dst as *mut u16, utf16.len());
            let _ = GlobalUnlock(text_mem);

            if SetClipboardData(13, HANDLE(text_mem.0)).is_err() {
                return Err(AppError::InvalidInput("SetClipboardData CF_UNICODETEXT".into()));
            }

            // Запрет сохранения в историю Windows (Win+V) и облачную синхронизацию.
            for name in [
                "ExcludeClipboardContentFromMonitorProcessing",
                "CanIncludeInClipboardHistory",
                "CanUploadToCloudClipboard",
            ] {
                let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let fmt = RegisterClipboardFormatW(PCWSTR(wide.as_ptr()));
                if fmt == 0 {
                    return Err(AppError::InvalidInput(format!("RegisterClipboardFormatW {name}")));
                }

                let mem = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of::<u32>())
                    .map_err(|e| AppError::InvalidInput(format!("GlobalAlloc {name}: {e}")))?;
                let ptr = GlobalLock(mem);
                if ptr.is_null() {
                    return Err(AppError::InvalidInput(format!("GlobalLock {name}")));
                }
                // Значение DWORD 0 отключает обработку формата.
                std::ptr::write(ptr as *mut u32, 0u32);
                let _ = GlobalUnlock(mem);

                if SetClipboardData(fmt, HANDLE(mem.0)).is_err() {
                    return Err(AppError::InvalidInput(format!("SetClipboardData {name}")));
                }
            }

            Ok(())
        })();

        let _ = CloseClipboard();
        result?;
    }

    Ok(())
}

#[tauri::command]
pub fn set_window_content_protection(
    window: tauri::Window,
    enabled: bool,
) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        let tauri_hwnd = window.hwnd().map_err(|e| AppError::InvalidInput(e.to_string()))?;
        let hwnd = HWND(tauri_hwnd.0 as *mut _);
        
        let affinity = if enabled {
            WDA_EXCLUDEFROMCAPTURE
        } else {
            WDA_NONE
        };

        unsafe {
            SetWindowDisplayAffinity(hwnd, affinity)
                .map_err(|e| AppError::InvalidInput(format!("SetWindowDisplayAffinity: {e}")))?;
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (window, enabled);
    }

    Ok(())
}

#[tauri::command]
pub fn destroy_vault(
    state: State<'_, AppState>,
    master_password: SecretString,
) -> Result<(), AppError> {
    if !state.has_vault() {
        return Err(AppError::NotInitialized);
    }

    let _ = Session::unlock(state.vault_path.clone(), master_password.expose_secret())?;
    let _ = state.take_session();

    let _ = std::fs::remove_file(&state.vault_path);
    let bak_path = state.vault_path.with_extension("enc.bak");
    let _ = std::fs::remove_file(bak_path);

    Ok(())
}

fn validate_master_password(pw: &str) -> Result<(), AppError> {
    if pw.chars().count() < 12 {
        return Err(AppError::InvalidInput(
            "master password must be at least 12 characters".into(),
        ));
    }
    Ok(())
}