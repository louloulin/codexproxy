//!
//! AES-256-GCM Symmetric Encryption
//! 
//! Rust implementation aligned with mimo2codex security.encryption.test.ts
//! 
//! Features:
//! - encrypt_string: Encrypt plaintext with AES-256-GCM
//! - decrypt_string: Decrypt sealed secret with AES-256-GCM
//! - SealedSecret struct for nonce, ciphertext, and auth tag

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

/// Sealed secret containing encrypted data
#[derive(Debug, Clone, PartialEq)]
pub struct SealedSecret {
    /// Base64 encoded ciphertext
    pub ciphertext: String,
    /// Base64 encoded nonce
    pub nonce: String,
    /// Base64 encoded auth tag
    pub auth_tag: String,
}

/// Encrypt a string with AES-256-GCM
/// 
/// # Arguments
/// * `plaintext` - The string to encrypt
/// * `key` - 32-byte encryption key
/// 
/// # Returns
/// SealedSecret containing base64-encoded ciphertext, nonce, and auth tag
pub fn encrypt_string(plaintext: &str, key: &[u8; 32]) -> SealedSecret {
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("encryption should not fail");
    
    // GCM auth tag is appended to ciphertext, extract it
    let auth_tag = ciphertext[ciphertext.len() - 16..].to_vec();
    let ct_without_tag = &ciphertext[..ciphertext.len() - 16];
    
    SealedSecret {
        ciphertext: base64_encode(ct_without_tag),
        nonce: base64_encode(&nonce_bytes),
        auth_tag: base64_encode(&auth_tag),
    }
}

/// Decrypt a sealed secret with AES-256-GCM
/// 
/// # Arguments
/// * `sealed` - The sealed secret to decrypt
/// * `key` - 32-byte encryption key
/// 
/// # Returns
/// Decrypted plaintext string
pub fn decrypt_string(sealed: &SealedSecret, key: &[u8; 32]) -> String {
    let cipher = Aes256Gcm::new(key.into());
    let nonce_bytes = base64_decode(&sealed.nonce);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let auth_tag = base64_decode(&sealed.auth_tag);
    let mut ciphertext = base64_decode(&sealed.ciphertext);
    
    // Append auth tag for decryption
    ciphertext.extend_from_slice(&auth_tag);
    
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .expect("decryption failed - data may be corrupted or wrong key");
    
    String::from_utf8(plaintext).expect("decrypted data should be valid utf8")
}

// Simple base64 encoding using standard alphabet
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let mut result = String::new();
    let mut i = 0;
    
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
        
        result.push(ALPHABET[(b0 >> 2) as usize] as char);
        
        let idx1 = ((b0 & 0x03) << 4) | (b1 >> 4);
        result.push(ALPHABET[idx1 as usize] as char);
        
        if i + 1 < data.len() {
            let idx2 = ((b1 & 0x0F) << 2) | (b2 >> 6);
            result.push(ALPHABET[idx2 as usize] as char);
        } else {
            result.push('=');
        }
        
        if i + 2 < data.len() {
            let idx3 = b2 & 0x3F;
            result.push(ALPHABET[idx3 as usize] as char);
        } else {
            result.push('=');
        }
        
        i += 3;
    }
    
    result
}

// Simple base64 decoding
fn base64_decode(data: &str) -> Vec<u8> {
    const DECODE: [i8; 256] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1, -1, 63,
        52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1,
        -1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14,
        15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1, -1,
        -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
        41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    ];
    
    let clean: Vec<u8> = data
        .bytes()
        .filter(|&b| b != b'=')
        .collect();
    
    let mut result = Vec::new();
    let mut i = 0;
    
    while i < clean.len() {
        let c0 = clean[i] as usize;
        let c1 = if i + 1 < clean.len() { clean[i + 1] as usize } else { 0 };
        let c2 = if i + 2 < clean.len() { clean[i + 2] as usize } else { 0 };
        let c3 = if i + 3 < clean.len() { clean[i + 3] as usize } else { 0 };
        
        let b0 = (DECODE[c0] as u32) << 2 | (DECODE[c1] as u32) >> 4;
        result.push(b0 as u8);
        
        if i + 2 < clean.len() {
            let b1 = ((DECODE[c1] & 0x0F) as u32) << 4 | (DECODE[c2] as u32) >> 2;
            result.push(b1 as u8);
        }
        
        if i + 3 < clean.len() {
            let b2 = ((DECODE[c2] & 0x03) as u32) << 6 | DECODE[c3] as u32;
            result.push(b2 as u8);
        }
        
        i += 4;
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests aligned with mimo2codex security.encryption.test.ts

    #[test]
    fn test_round_trips_with_fresh_key() {
        let key = [0u8; 32];
        let sealed = encrypt_string("sk-supersecret", &key);
        let decrypted = decrypt_string(&sealed, &key);
        assert_eq!(decrypted, "sk-supersecret");
    }

    #[test]
    fn test_produces_unique_nonces() {
        let key = [0u8; 32];
        let a = encrypt_string("same", &key);
        let b = encrypt_string("same", &key);
        assert_ne!(a.nonce, b.nonce, "nonces should be unique");
        assert_ne!(a.ciphertext, b.ciphertext, "ciphertexts should be unique");
    }

    #[test]
    fn test_decryption_fails_when_ciphertext_tampered() {
        let key = [0u8; 32];
        let mut sealed = encrypt_string("plaintext", &key);
        
        // Tamper with first byte of ciphertext
        let mut ct_bytes = base64_decode(&sealed.ciphertext);
        ct_bytes[0] ^= 0x01;
        sealed.ciphertext = base64_encode(&ct_bytes);
        
        // Decryption should fail
        let result = std::panic::catch_unwind(|| decrypt_string(&sealed, &key));
        assert!(result.is_err(), "decryption should fail when ciphertext is tampered");
    }

    #[test]
    fn test_decryption_fails_when_auth_tag_tampered() {
        let key = [0u8; 32];
        let mut sealed = encrypt_string("plaintext", &key);
        
        // Tamper with first byte of auth tag
        let mut tag_bytes = base64_decode(&sealed.auth_tag);
        tag_bytes[0] ^= 0x01;
        sealed.auth_tag = base64_encode(&tag_bytes);
        
        // Decryption should fail
        let result = std::panic::catch_unwind(|| decrypt_string(&sealed, &key));
        assert!(result.is_err(), "decryption should fail when auth tag is tampered");
    }

    #[test]
    fn test_decryption_fails_under_different_key() {
        let key_a = [0u8; 32];
        let key_b: [u8; 32] = core::array::from_fn(|i| (i as u8).wrapping_add(1));
        
        let sealed = encrypt_string("plaintext", &key_a);
        
        // Decryption with different key should fail
        let result = std::panic::catch_unwind(|| decrypt_string(&sealed, &key_b));
        assert!(result.is_err(), "decryption should fail with different key");
    }

    #[test]
    fn test_sealed_secret_struct_fields() {
        let key = [0u8; 32];
        let sealed = encrypt_string("test", &key);
        
        assert!(!sealed.ciphertext.is_empty(), "ciphertext should not be empty");
        assert!(!sealed.nonce.is_empty(), "nonce should not be empty");
        assert!(!sealed.auth_tag.is_empty(), "auth_tag should not be empty");
    }

    #[test]
    fn test_base64_encoding_is_valid() {
        let key = [0u8; 32];
        let sealed = encrypt_string("hello world", &key);
        
        // Verify base64 is decodable
        let _ct = base64_decode(&sealed.ciphertext);
        let _nonce = base64_decode(&sealed.nonce);
        let _tag = base64_decode(&sealed.auth_tag);
    }
}
