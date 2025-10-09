use blake3::Hasher;
use tokio::task;

pub async fn check_integrity(data: &[u8], original: &str) -> bool {
    let mut hasher = Hasher::new();
    hasher.update(original.as_bytes());
    let expected_hash = hasher.finalize();

    let mut hasher = Hasher::new();
    hasher.update(data);
    let actual_hash = hasher.finalize();

    task::spawn_blocking(move || expected_hash == actual_hash).await.expect("Task failed")
}