#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::{self, KdfParams, SecretKey, NONCE_LEN, SALT_LEN};
use crate::errors::VaultError;

const FORMAT_VERSION: u32 = 1;
const KDF_ALGO: &str = "argon2id";

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultFile {
    pub version: u32,
    pub kdf: KdfMetadata,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KdfMetadata {
    pub algo: String,
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
    pub salt: String,
}

impl KdfMetadata {
    fn to_params(&self) -> KdfParams {
        KdfParams {
            m_cost_kib: self.m_cost_kib,
            t_cost: self.t_cost,
            p_cost: self.p_cost,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct VaultPayload {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub password: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct Session {
    path: PathBuf,
    kdf: KdfMetadata,
    key: SecretKey,
    payload: VaultPayload,
}

impl Session {
    pub fn create(path: PathBuf, password: &str, params: KdfParams) -> Result<Self, VaultError> {
        if path.exists() {
            return Err(VaultError::AlreadyExists);
        }

        let salt = crypto::generate_salt();
        let key = crypto::derive_key(password, &salt, &params)?;
        let kdf = KdfMetadata {
            algo: KDF_ALGO.to_string(),
            m_cost_kib: params.m_cost_kib,
            t_cost: params.t_cost,
            p_cost: params.p_cost,
            salt: B64.encode(salt),
        };

        let session = Session {
            path,
            kdf,
            key,
            payload: VaultPayload::default(),
        };
        session.persist()?;
        Ok(session)
    }

    pub fn unlock(path: PathBuf, password: &str) -> Result<Self, VaultError> {
        let raw = fs::read(&path)?;
        let file: VaultFile = serde_json::from_slice(&raw)?;

        if file.version != FORMAT_VERSION {
            return Err(VaultError::UnsupportedVersion(file.version));
        }
        if file.kdf.algo != KDF_ALGO {
            return Err(VaultError::UnsupportedKdf(file.kdf.algo.clone()));
        }

        let salt = B64.decode(&file.kdf.salt)?;
        if salt.len() != SALT_LEN {
            return Err(VaultError::Corrupted("invalid salt length".into()));
        }

        let nonce = B64.decode(&file.nonce)?;
        if nonce.len() != NONCE_LEN {
            return Err(VaultError::Corrupted("invalid nonce length".into()));
        }

        let ciphertext = B64.decode(&file.ciphertext)?;
        let key = crypto::derive_key(password, &salt, &file.kdf.to_params())?;
        let plaintext = crypto::decrypt(&key, &nonce, &ciphertext)?;
        let payload: VaultPayload = serde_json::from_slice(&plaintext)?;

        Ok(Session {
            path,
            kdf: file.kdf,
            key,
            payload,
        })
    }

    fn persist(&self) -> Result<(), VaultError> {
        let plaintext = serde_json::to_vec(&self.payload)?;
        let (nonce, ciphertext) = crypto::encrypt(&self.key, &plaintext)?;

        let file = VaultFile {
            version: FORMAT_VERSION,
            kdf: self.kdf.clone(),
            nonce: B64.encode(&nonce),
            ciphertext: B64.encode(&ciphertext),
        };
        let bytes = serde_json::to_vec_pretty(&file)?;
        atomic_write(&self.path, &bytes)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn list_entries(&self) -> &[Entry] {
        &self.payload.entries
    }

    pub fn add_entry(&mut self, title: String, password: String) -> Result<Entry, VaultError> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(VaultError::InvalidInput("title is empty".into()));
        }
        if self.payload.entries.iter().any(|e| e.title == title) {
            return Err(VaultError::InvalidInput("title already exists".into()));
        }

        let now = Utc::now().to_rfc3339();
        let entry = Entry {
            id: Uuid::new_v4().to_string(),
            title,
            password,
            created_at: now.clone(),
            updated_at: now,
        };
        self.payload.entries.push(entry.clone());
        self.persist()?;
        Ok(entry)
    }

    pub fn update_entry(
        &mut self,
        id: &str,
        title: Option<String>,
        password: Option<String>,
    ) -> Result<Entry, VaultError> {
        let idx = self
            .payload
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or(VaultError::EntryNotFound)?;

        if let Some(new_title) = &title {
            let new_title = new_title.trim();
            if new_title.is_empty() {
                return Err(VaultError::InvalidInput("title is empty".into()));
            }
            if self
                .payload
                .entries
                .iter()
                .enumerate()
                .any(|(i, e)| i != idx && e.title == new_title)
            {
                return Err(VaultError::InvalidInput("title already exists".into()));
            }
        }

        let now = Utc::now().to_rfc3339();
        let entry = &mut self.payload.entries[idx];
        if let Some(new_title) = title {
            entry.title = new_title.trim().to_string();
        }
        if let Some(new_password) = password {
            entry.password = new_password;
        }
        entry.updated_at = now;

        let updated = entry.clone();
        self.persist()?;
        Ok(updated)
    }

    pub fn delete_entry(&mut self, id: &str) -> Result<(), VaultError> {
        let idx = self
            .payload
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or(VaultError::EntryNotFound)?;
        self.payload.entries.remove(idx);
        self.persist()?;
        Ok(())
    }

    pub fn reveal_password(&self, id: &str) -> Result<&str, VaultError> {
        self.payload
            .entries
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.password.as_str())
            .ok_or(VaultError::EntryNotFound)
    }

    /// Полностью удаляет все записи из хранилища.
    pub fn wipe_entries(&mut self) -> Result<(), VaultError> {
        self.payload.entries.clear();
        self.persist()?;
        Ok(())
    }
}

/// Атомарная запись с предварительным созданием бэкапа
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), VaultError> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent directory")
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file name"))?;

    // Создает бэкап перед перезаписью
    if path.exists() {
        let bak_path = path.with_extension("enc.bak");
        let _ = fs::copy(path, bak_path);
    }

    let tmp_name = format!(".{}.tmp", file_name.to_string_lossy());
    let tmp_path = parent.join(&tmp_name);

    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        VaultError::Io(e)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fast_kdf() -> KdfParams {
        KdfParams { m_cost_kib: 4096, t_cost: 1, p_cost: 1 }
    }

    fn new_path(dir: &TempDir) -> PathBuf {
        dir.path().join("vault.enc")
    }

    #[test]
    fn create_writes_file() {
        let dir = TempDir::new().unwrap();
        let path = new_path(&dir);
        let session = Session::create(path.clone(), "master123456", fast_kdf()).unwrap();
        assert!(path.exists());
        drop(session);
    }

    #[test]
    fn wipe_entries_removes_data() {
        let dir = TempDir::new().unwrap();
        let path = new_path(&dir);
        let mut session = Session::create(path.clone(), "master123456", fast_kdf()).unwrap();
        session.add_entry("test".into(), "pwd".into()).unwrap();
        assert_eq!(session.list_entries().len(), 1);
        session.wipe_entries().unwrap();
        assert_eq!(session.list_entries().len(), 0);
        
        let new_session = Session::unlock(path, "master123456").unwrap();
        assert_eq!(new_session.list_entries().len(), 0);
    }
}