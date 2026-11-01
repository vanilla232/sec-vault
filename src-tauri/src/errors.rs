use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid KDF params")]
    InvalidParams,

    #[error("KDF derivation failed")]
    Kdf,

    #[error("encryption failed")]
    Encrypt,

    #[error("decryption failed")]
    Decrypt,

    #[error("invalid nonce length")]
    InvalidNonce,

    #[error("invalid key length")]
    InvalidKey,
}

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("vault already exists at this path")]
    AlreadyExists,

    #[error("entry not found")]
    EntryNotFound,

    #[error("unsupported vault version: {0}")]
    UnsupportedVersion(u32),

    #[error("unsupported KDF algorithm: {0}")]
    UnsupportedKdf(String),

    #[error("corrupted vault: {0}")]
    Corrupted(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Vault(#[from] VaultError),

    #[error("vault is locked")]
    Locked,

    #[error("vault is not initialized")]
    NotInitialized,

    #[error("vault is already unlocked")]
    AlreadyUnlocked,

    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}