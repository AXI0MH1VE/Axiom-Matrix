use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};
use rand::Rng;
use tokio::task;

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