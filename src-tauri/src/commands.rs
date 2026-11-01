use serde::Serialize;
use std::thread;
use std::time::Duration;
use secrecy::{ExposeSecret, SecretString};
use tauri::State;

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
    password: SecretString,
) -> Result<EntryMeta, AppError> {
    state.with_session_mut(|session| {
        let entry = session.add_entry(title, password.expose_secret().to_string())?;
        Ok(EntryMeta::from(&entry))
    })
}

#[tauri::command]
pub fn update_entry(
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    password: Option<SecretString>,
) -> Result<EntryMeta, AppError> {
    state.with_session_mut(|session| {
        let entry = session.update_entry(
            &id,
            title,
            password.map(|p| p.expose_secret().to_string()),
        )?;
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
    let mut clipboard = arboard::Clipboard::new().map_err(|e| AppError::InvalidInput(e.to_string()))?;
    clipboard.set_text(text.clone()).map_err(|e| AppError::InvalidInput(e.to_string()))?;

    if clear_after_secs > 0 {
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(clear_after_secs));
            if let Ok(mut cb) = arboard::Clipboard::new() {
                if let Ok(current) = cb.get_text() {
                    // Очищаем только если в буфере всё ещё наш пароль
                    if current == text {
                        let _ = cb.set_text("");
                    }
                }
            }
        });
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