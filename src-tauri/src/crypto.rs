use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::{rngs::OsRng, RngCore};
use secrecy::{ExposeSecret, SecretBox};
use zeroize::Zeroize;

use crate::errors::CryptoError;

pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Copy)]
pub struct KdfParams {
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            m_cost_kib: 65_536,
            t_cost: 3,
            p_cost: 4,
        }
    }
}

pub type SecretKey = SecretBox<[u8; KEY_LEN]>;

pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

pub fn derive_key(
    master_password: &str,
    salt: &[u8],
    params: &KdfParams,
) -> Result<SecretKey, CryptoError> {
    let argon_params =
        Params::new(params.m_cost_kib, params.t_cost, params.p_cost, Some(KEY_LEN))
            .map_err(|_| CryptoError::InvalidParams)?;

    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

    let mut key_bytes = [0u8; KEY_LEN];
    argon
        .hash_password_into(master_password.as_bytes(), salt, &mut key_bytes)
        .map_err(|_| CryptoError::Kdf)?;

    let secret = SecretBox::new(Box::new(key_bytes));
    key_bytes.zeroize();
    Ok(secret)
}

pub fn encrypt(key: &SecretKey, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key.expose_secret().as_slice())
        .map_err(|_| CryptoError::InvalidKey)?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::Encrypt)?;

    Ok((nonce_bytes.to_vec(), ciphertext))
}

pub fn decrypt(key: &SecretKey, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if nonce.len() != NONCE_LEN {
        return Err(CryptoError::InvalidNonce);
    }

    let cipher = Aes256Gcm::new_from_slice(key.expose_secret().as_slice())
        .map_err(|_| CryptoError::InvalidKey)?;

    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Слабые параметры чтобы тесты не ждали секундами на KDF
    fn fast() -> KdfParams {
        KdfParams {
            m_cost_kib: 4096,
            t_cost: 1,
            p_cost: 1,
        }
    }

    #[test]
    fn derive_is_deterministic() {
        let salt = [7u8; SALT_LEN];
        let a = derive_key("hunter2", &salt, &fast()).unwrap();
        let b = derive_key("hunter2", &salt, &fast()).unwrap();
        assert_eq!(a.expose_secret(), b.expose_secret());
    }

    #[test]
    fn derive_depends_on_salt() {
        let a = derive_key("hunter2", &[1u8; SALT_LEN], &fast()).unwrap();
        let b = derive_key("hunter2", &[2u8; SALT_LEN], &fast()).unwrap();
        assert_ne!(a.expose_secret(), b.expose_secret());
    }

    #[test]
    fn derive_depends_on_password() {
        let salt = [3u8; SALT_LEN];
        let a = derive_key("hunter2", &salt, &fast()).unwrap();
        let b = derive_key("hunter3", &salt, &fast()).unwrap();
        assert_ne!(a.expose_secret(), b.expose_secret());
    }

    #[test]
    fn roundtrip() {
        let salt = generate_salt();
        let key = derive_key("correct horse battery staple", &salt, &fast()).unwrap();
        let msg = b"the answer is 42";
        let (nonce, ct) = encrypt(&key, msg).unwrap();
        let pt = decrypt(&key, &nonce, &ct).unwrap();
        assert_eq!(pt, msg);
    }

    #[test]
    fn wrong_key_fails() {
        let salt = generate_salt();
        let k1 = derive_key("a", &salt, &fast()).unwrap();
        let k2 = derive_key("b", &salt, &fast()).unwrap();
        let (nonce, ct) = encrypt(&k1, b"data").unwrap();
        assert!(decrypt(&k2, &nonce, &ct).is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let salt = generate_salt();
        let key = derive_key("pass", &salt, &fast()).unwrap();
        let (nonce, mut ct) = encrypt(&key, b"data").unwrap();
        if let Some(byte) = ct.last_mut() {
            *byte ^= 0x01;
        }
        assert!(decrypt(&key, &nonce, &ct).is_err());
    }

    #[test]
    fn tampered_nonce_fails() {
        let salt = generate_salt();
        let key = derive_key("pass", &salt, &fast()).unwrap();
        let (mut nonce, ct) = encrypt(&key, b"data").unwrap();
        nonce[0] ^= 0x01;
        assert!(decrypt(&key, &nonce, &ct).is_err());
    }

    #[test]
    fn invalid_nonce_length_fails() {
        let salt = generate_salt();
        let key = derive_key("pass", &salt, &fast()).unwrap();
        let (_nonce, ct) = encrypt(&key, b"data").unwrap();
        assert!(decrypt(&key, &[0u8; 4], &ct).is_err());
    }

    #[test]
    fn nonces_are_unique() {
        let salt = generate_salt();
        let key = derive_key("pass", &salt, &fast()).unwrap();
        let (n1, _) = encrypt(&key, b"x").unwrap();
        let (n2, _) = encrypt(&key, b"x").unwrap();
        assert_ne!(n1, n2);
    }
}