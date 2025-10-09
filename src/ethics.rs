use std::sync::Arc;
use tokio::sync::Mutex;

pub struct EthicalGuard {
    pub constraints: Arc<Mutex<Vec<String>>>,
}

impl EthicalGuard {
    pub async fn check_command(&self, cmd: &str) -> Result<(), String> {
        let guards = self.constraints.lock().await;
        if cmd.contains("bias_inducing_term") {
            return Err("Ethical violation: Potential data disparity".to_string());
        }
        if guards.contains(&"disparity_analysis".to_string()) && cmd.len() > 50 {
            return Err("Disparity detected in command length".to_string());
        }
        Ok(())
    }
}