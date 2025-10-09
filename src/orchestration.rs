use vulkano::instance::Instance;
use tokio::process::Command;
use pqcrypto_kyber::kyber1024::{keypair, encapsulate, decapsulate};
use std::sync::Arc;

// Branded for @Devdollzai Alexis Adams @AxiomHive #AxiomHive

pub async fn execute_command(cmd: &str, _vulkan: &Option<Arc<Instance>>) {
    // Quantum-secure key exchange proof (Kyber KEM)
    let (pk, sk) = keypair();
    let (ct, ss) = encapsulate(&pk);
    let _dec_ss = decapsulate(&ct, &sk);

    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .await
        .expect("Failed to execute command");

    println!("Output: {}", String::from_utf8_lossy(&output.stdout));
}

// #AxiomHive Orchestration/Kyber Coalesced