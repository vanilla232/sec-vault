use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

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
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Entry> for EntryMeta {
    fn from(e: &Entry) -> Self {
        Self {
            id: e.id.clone(),
            title: e.title.clone(),
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
    master_password: String,
) -> Result<(), AppError> {
    validate_master_password(&master_password)?;
    if state.has_vault() {
        return Err(AppError::InvalidInput("vault already exists".into()));
    }
    let session =
        Session::create(state.vault_path.clone(), &master_password, KdfParams::default())?;
    state.set_session(session)?;
    Ok(())
}

#[tauri::command]
pub fn unlock_vault(
    state: State<'_, AppState>,
    master_password: String,
) -> Result<(), AppError> {
    if !state.has_vault() {
        return Err(AppError::NotInitialized);
    }
    if state.is_unlocked() {
        return Err(AppError::AlreadyUnlocked);
    }
    let session = Session::unlock(state.vault_path.clone(), &master_password)?;
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
    password: String,
) -> Result<EntryMeta, AppError> {
    state.with_session_mut(|session| {
        let entry = session.add_entry(title, password)?;
        Ok(EntryMeta::from(&entry))
    })
}

#[tauri::command]
pub fn update_entry(
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    password: Option<String>,
) -> Result<EntryMeta, AppError> {
    state.with_session_mut(|session| {
        let entry = session.update_entry(&id, title, password)?;
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
pub fn copy_to_clipboard(
    app: AppHandle,
    text: String,
    clear_after_secs: u64,
) -> Result<(), AppError> {
    app.clipboard()
        .write_text(text.clone())
        .map_err(|e| AppError::InvalidInput(e.to_string()))?;

    if clear_after_secs > 0 {
        let app_handle = app.clone();
        let original_text = text.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(clear_after_secs));
            // Очищаем, только если в буфере всё ещё наш пароль
            if let Ok(current) = app_handle.clipboard().read_text() {
                if current == original_text {
                    let _ = app_handle.clipboard().write_text("".to_string());
                }
            }
        });
    }
    Ok(())
}

#[tauri::command]
pub fn destroy_vault(
    state: State<'_, AppState>,
    master_password: String,
) -> Result<(), AppError> {
    if !state.has_vault() {
        return Err(AppError::NotInitialized);
    }

    // Проверяем мастер-пароль. Если он неверный, файлы останутся нетронутыми.
    let _ = Session::unlock(state.vault_path.clone(), &master_password)?;

    // Сбрасываем текущую активную сессию (уничтожаем ключ в памяти)
    let _ = state.take_session();

    // Полностью удаляем файл хранилища и его бэкап
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