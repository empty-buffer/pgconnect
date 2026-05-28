use crate::error::{Error, Result};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{password_hash::SaltString, Argon2, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;

const NONCE_SIZE: usize = 12;

/// Derives a 256-bit encryption key from a master password and salt using Argon2id
pub fn derive_key(master_password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let argon2 = Argon2::default();
    let mut key = [0u8; 32];

    argon2
        .hash_password_into(master_password.as_bytes(), salt, &mut key)
        .map_err(|e| Error::core(format!("Key derivation failed: {}", e)))?;

    Ok(key)
}

/// Generates a random salt for key derivation
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    use rand::RngCore;
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Generates a random nonce for AES-GCM encryption
pub fn generate_nonce() -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    use rand::RngCore;
    OsRng.fill_bytes(&mut nonce);
    nonce
}

/// Encrypts a password using AES-256-GCM
/// Returns (ciphertext, nonce)
pub fn encrypt_password(password: &str, key: &[u8; 32]) -> Result<(Vec<u8>, [u8; NONCE_SIZE])> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::core(format!("Failed to create cipher: {}", e)))?;

    let nonce_bytes = generate_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, password.as_bytes())
        .map_err(|e| Error::core(format!("Encryption failed: {}", e)))?;

    Ok((ciphertext, nonce_bytes))
}

/// Decrypts a password using AES-256-GCM
pub fn decrypt_password(ciphertext: &[u8], nonce: &[u8], key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::core(format!("Failed to create cipher: {}", e)))?;

    let nonce = Nonce::from_slice(nonce);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| Error::core("Decryption failed - incorrect master password?"))?;

    String::from_utf8(plaintext)
        .map_err(|e| Error::core(format!("Invalid UTF-8 in decrypted password: {}", e)))
}

/// Hashes the master password for verification (stored in DB)
pub fn hash_master_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::core(format!("Password hashing failed: {}", e)))?;

    Ok(hash.to_string())
}

/// Verifies the master password against stored hash
pub fn verify_master_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = argon2::PasswordHash::new(hash)
        .map_err(|e| Error::core(format!("Invalid password hash: {}", e)))?;

    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let password = "my_secret_password";
        let salt = generate_salt();
        let key = derive_key("master_password", &salt).unwrap();

        let (ciphertext, nonce) = encrypt_password(password, &key).unwrap();
        let decrypted = decrypt_password(&ciphertext, &nonce, &key).unwrap();

        assert_eq!(password, decrypted);
    }

    #[test]
    fn test_master_password_verification() {
        let password = "master_password";
        let hash = hash_master_password(password).unwrap();

        assert!(verify_master_password(password, &hash).unwrap());
        assert!(!verify_master_password("wrong_password", &hash).unwrap());
    }
}
