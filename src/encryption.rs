use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};
use rand::Rng;
use tokio::task;
use blake3::Hasher;

// HASH_SIZE is 32 bytes for BLAKE3
const HASH_SIZE: usize = 32;

pub fn generate_hash(data: &str) -> [u8; HASH_SIZE] {
    let mut hasher = Hasher::new();
    hasher.update(data.as_bytes());
    *hasher.finalize().as_bytes()
}

pub async fn encrypt_command_with_integrity(cmd: &str) -> Vec<u8> {
    // 1. Generate hash of original plaintext
    let original_hash = generate_hash(cmd);

    // 2. Encrypt the command
    let key = Key::from_slice(b"an example very very secret key.");
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = task::spawn_blocking(move || {
        cipher.encrypt(nonce, cmd.as_bytes().as_ref()).expect("Encryption failure")
    }).await.expect("Task failed");

    // 3. Prepend the original hash and nonce to the ciphertext
    [original_hash.to_vec(), nonce_bytes.to_vec(), ciphertext].concat()
}

pub async fn decrypt_command_with_integrity(data: &[u8]) -> Result<String, String> {
    // 1. Deconstruct the data packet
    let (original_hash_bytes, rest) = data.split_at(HASH_SIZE);
    let (nonce_bytes, ciphertext) = rest.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // 2. Decrypt the ciphertext
    let key = Key::from_slice(b"an example very very secret key.");
    let cipher = Aes256Gcm::new(key);
    let plaintext_bytes = task::spawn_blocking(move || {
        cipher.decrypt(nonce, ciphertext).map_err(|e| format!("Decryption failed: {:?}", e))
    }).await.map_err(|e| format!("Task failed: {:?}", e))??;

    let decrypted_cmd = String::from_utf8(plaintext_bytes).map_err(|e| format!("Invalid UTF-8: {:?}", e))?;

    // 3. Verify integrity
    let decrypted_hash_bytes = generate_hash(&decrypted_cmd);
    if original_hash_bytes == decrypted_hash_bytes {
        Ok(decrypted_cmd)
    } else {
        Err("Integrity check failed: Hashes do not match.".to_string())
    }
}

pub async fn encrypt_command(cmd: &str) -> Vec<u8> {
    let key = Key::from_slice(b"an example very very secret key.");
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cmd.as_bytes();
    let ciphertext = task::spawn_blocking(move || {
        cipher.encrypt(nonce, plaintext.as_ref()).expect("Encryption failure")
    }).await.expect("Task failed");

    [nonce_bytes.to_vec(), ciphertext].concat()
}

pub async fn decrypt_command(data: &[u8]) -> Result<String, String> {
    let key = Key::from_slice(b"an example very very secret key.");
    let cipher = Aes256Gcm::new(key);
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = task::spawn_blocking(move || {
        cipher.decrypt(nonce, ciphertext).map_err(|e| format!("Decryption failed: {:?}", e))
    }).await.map_err(|e| format!("Task failed: {:?}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8: {:?}", e))
}