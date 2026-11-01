use std::path::PathBuf;
use std::sync::Mutex;

use crate::errors::AppError;
use crate::vault::Session;

pub struct AppState {
    pub vault_path: PathBuf,
    inner: Mutex<Inner>,
}

struct Inner {
    session: Option<Session>,
}

impl AppState {
    pub fn new(vault_path: PathBuf) -> Self {
        Self {
            vault_path,
            inner: Mutex::new(Inner { session: None }),
        }
    }

    pub fn has_vault(&self) -> bool {
        self.vault_path.exists()
    }

    pub fn is_unlocked(&self) -> bool {
        self.inner.lock().unwrap().session.is_some()
    }

    pub fn set_session(&self, session: Session) -> Result<(), AppError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.session.is_some() {
            return Err(AppError::AlreadyUnlocked);
        }
        inner.session = Some(session);
        Ok(())
    }

    pub fn take_session(&self) -> Option<Session> {
        self.inner.lock().unwrap().session.take()
    }

    pub fn with_session<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Session) -> Result<T, AppError>,
    {
        let inner = self.inner.lock().unwrap();
        let session = inner.session.as_ref().ok_or(AppError::Locked)?;
        f(session)
    }

    pub fn with_session_mut<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&mut Session) -> Result<T, AppError>,
    {
        let mut inner = self.inner.lock().unwrap();
        let session = inner.session.as_mut().ok_or(AppError::Locked)?;
        f(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KdfParams;
    use tempfile::TempDir;

    fn fast_kdf() -> KdfParams {
        KdfParams { m_cost_kib: 4096, t_cost: 1, p_cost: 1 }
    }

    #[test]
    fn starts_locked_and_uninitialized() {
        let dir = TempDir::new().unwrap();
        let state = AppState::new(dir.path().join("vault.enc"));
        assert!(!state.has_vault());
        assert!(!state.is_unlocked());
    }

    #[test]
    fn set_session_then_lock_works() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.enc");
        let state = AppState::new(path.clone());

        let session = Session::create(path, "master123456", fast_kdf()).unwrap();
        state.set_session(session).unwrap();
        assert!(state.is_unlocked());
        assert!(state.has_vault());

        state.take_session();
        assert!(!state.is_unlocked());
    }

    #[test]
    fn with_session_errors_when_locked() {
        let dir = TempDir::new().unwrap();
        let state = AppState::new(dir.path().join("vault.enc"));
        let res: Result<(), AppError> = state.with_session(|_| Ok(()));
        assert!(matches!(res, Err(AppError::Locked)));
    }

    #[test]
    fn set_session_rejects_double_unlock() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.enc");
        let state = AppState::new(path.clone());

        let s1 = Session::create(path.clone(), "master123456", fast_kdf()).unwrap();
        state.set_session(s1).unwrap();

        let s2 = Session::unlock(path, "master123456").unwrap();
        assert!(matches!(
            state.set_session(s2),
            Err(AppError::AlreadyUnlocked)
        ));
    }
}