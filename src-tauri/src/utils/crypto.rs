use crate::utils::error::Result;
use ring::{aead, rand};
use ring::rand::SecureRandom;

const NONCE_LEN: usize = 12;

/// Encrypt data using AES-GCM
pub fn encrypt(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| crate::utils::error::UvcadError::InvalidConfig("Invalid encryption key".to_string()))?;
    let sealing_key = aead::LessSafeKey::new(unbound_key);

    let rng = rand::SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| crate::utils::error::UvcadError::InvalidConfig("Random generation failed".to_string()))?;

    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);

    let mut in_out = data.to_vec();
    sealing_key.seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut in_out)
        .map_err(|_| crate::utils::error::UvcadError::InvalidConfig("Encryption failed".to_string()))?;

    // Prepend nonce to ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&in_out);

    Ok(result)
}

/// Decrypt data using AES-GCM
pub fn decrypt(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if encrypted.len() < NONCE_LEN {
        return Err(crate::utils::error::UvcadError::InvalidConfig("Invalid encrypted data".to_string()));
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_LEN);
    let nonce = aead::Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| crate::utils::error::UvcadError::InvalidConfig("Invalid nonce".to_string()))?;

    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| crate::utils::error::UvcadError::InvalidConfig("Invalid encryption key".to_string()))?;
    let opening_key = aead::LessSafeKey::new(unbound_key);

    let mut in_out = ciphertext.to_vec();
    let plaintext = opening_key.open_in_place(nonce, aead::Aad::empty(), &mut in_out)
        .map_err(|_| crate::utils::error::UvcadError::InvalidConfig("Decryption failed".to_string()))?;

    Ok(plaintext.to_vec())
}

/// Generate a random encryption key
pub fn generate_key() -> Result<[u8; 32]> {
    let rng = rand::SystemRandom::new();
    let mut key = [0u8; 32];
    rng.fill(&mut key)
        .map_err(|_| crate::utils::error::UvcadError::InvalidConfig("Key generation failed".to_string()))?;
    Ok(key)
}
