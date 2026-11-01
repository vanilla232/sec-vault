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
    
    // Гарантируем зануление исходного массива до его удаления из стека
    key_bytes.zeroize();
    
    Ok(secret)
}

pub fn encrypt(key: &SecretKey, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key.expose_secret().as_slice())
        .map_err(|_| CryptoError::InvalidKey)?;

    // Критично: генерируем уникальный nonce на каждое шифрование (GCM не прощает повторного использования)
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

    fn fast_kdf() -> KdfParams {
        KdfParams {
            m_cost_kib: 4096,
            t_cost: 1,
            p_cost: 1,
        }
    }

    #[test]
    fn derive_key_is_deterministic() {
        let salt = [7u8; SALT_LEN];
        let a = derive_key("hunter2", &salt, &fast_kdf()).unwrap();
        let b = derive_key("hunter2", &salt, &fast_kdf()).unwrap();
        assert_eq!(a.expose_secret(), b.expose_secret());
    }

    #[test]
    fn derive_key_depends_on_salt() {
        let a = derive_key("hunter2", &[1u8; SALT_LEN], &fast_kdf()).unwrap();
        let b = derive_key("hunter2", &[2u8; SALT_LEN], &fast_kdf()).unwrap();
        assert_ne!(a.expose_secret(), b.expose_secret());
    }

    #[test]
    fn derive_key_depends_on_password() {
        let salt = [3u8; SALT_LEN];
        let a = derive_key("hunter2", &salt, &fast_kdf()).unwrap();
        let b = derive_key("hunter3", &salt, &fast_kdf()).unwrap();
        assert_ne!(a.expose_secret(), b.expose_secret());
    }

    #[test]
    fn derive_key_handles_unicode_and_long_passwords() {
        let salt = [4u8; SALT_LEN];
        let long_pw = "a".repeat(10000);
        
        assert!(derive_key("ПарольСЮникодом🔒", &salt, &fast_kdf()).is_ok());
        assert!(derive_key(&long_pw, &salt, &fast_kdf()).is_ok());
    }

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let salt = generate_salt();
        let key = derive_key("correct horse battery staple", &salt, &fast_kdf()).unwrap();
        
        let test_cases = vec![
            b"the answer is 42".to_vec(),
            b"".to_vec(), // Пустой массив
            "Юникод текст 🔒".as_bytes().to_vec(),
        ];

        for msg in test_cases {
            let (nonce, ct) = encrypt(&key, &msg).unwrap();
            let pt = decrypt(&key, &nonce, &ct).unwrap();
            assert_eq!(pt, msg);
        }
    }

    #[test]
    fn wrong_key_fails() {
        let salt = generate_salt();
        let k1 = derive_key("a", &salt, &fast_kdf()).unwrap();
        let k2 = derive_key("b", &salt, &fast_kdf()).unwrap();
        let (nonce, ct) = encrypt(&k1, b"data").unwrap();
        
        assert!(matches!(decrypt(&k2, &nonce, &ct), Err(CryptoError::Decrypt)));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let salt = generate_salt();
        let key = derive_key("pass", &salt, &fast_kdf()).unwrap();
        let (nonce, mut ct) = encrypt(&key, b"data").unwrap();
        
        if let Some(byte) = ct.last_mut() {
            *byte ^= 0x01;
        }
        
        assert!(matches!(decrypt(&key, &nonce, &ct), Err(CryptoError::Decrypt)));
    }

    #[test]
    fn tampered_nonce_fails() {
        let salt = generate_salt();
        let key = derive_key("pass", &salt, &fast_kdf()).unwrap();
        let (mut nonce, ct) = encrypt(&key, b"data").unwrap();
        
        nonce[0] ^= 0x01;
        
        assert!(matches!(decrypt(&key, &nonce, &ct), Err(CryptoError::Decrypt)));
    }

    #[test]
    fn nonce_is_unique() {
        let salt = generate_salt();
        let key = derive_key("pass", &salt, &fast_kdf()).unwrap();
        let (n1, ct1) = encrypt(&key, b"x").unwrap();
        let (n2, ct2) = encrypt(&key, b"x").unwrap();
        
        assert_ne!(n1, n2);
        assert_ne!(ct1, ct2);
    }

        #[test]
    fn known_answer_test_nist_aes256gcm() {
        // Вектор из NIST SP 800-38D (AES-256-GCM, Test Case 2, нулевой ключ/IV/данные)
        let key_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let nonce_hex = "000000000000000000000000";
        let pt_hex = "00000000000000000000000000000000";
        let expected_ct_hex = "cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919";

        let mut key_bytes = [0u8; KEY_LEN];
        key_bytes.copy_from_slice(&hex::decode(key_hex).unwrap());
        let key = SecretBox::new(Box::new(key_bytes));

        let nonce = hex::decode(nonce_hex).unwrap();
        let pt = hex::decode(pt_hex).unwrap();
        let expected_ct = hex::decode(expected_ct_hex).unwrap();

        // Проверяем, что наш модуль корректно расшифровывает эталонный шифротекст NIST
        let decrypted = decrypt(&key, &nonce, &expected_ct).unwrap();
        assert_eq!(decrypted, pt);
    }
}