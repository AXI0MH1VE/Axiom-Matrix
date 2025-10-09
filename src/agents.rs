use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use candle_core::{Device, Tensor};
use candle_transformers::models::bert::BertModel;
use candle_nn::VarBuilder;
use tokenizers::Tokenizer;
use vulkano::instance::Instance as VulkanInstance;

#[async_trait]
pub trait Agent: Send + Sync {
    async fn execute(&self, task: &str) -> Result<String, String>;
}

pub struct EthicalAgent {
    guard: Arc<EthicalGuard>,
    llm_model: Arc<BertModel>,
    tokenizer: Arc<Tokenizer>,
    device: Device,
}

impl EthicalAgent {
    pub fn new(guard: Arc<EthicalGuard>) -> Self {
        let device = Device::Cpu;
        let vb = VarBuilder::zeros(device.clone());
        let model = BertModel::load(&vb, "bert-base-uncased").expect("Model load");
        // Embedded vocab for EthicalAgent
        let vocab_str = r#"{
  "pad_token": "[PAD]",
  "unk_token": "[UNK]",
  "bos_token": "[CLS]",
  "eos_token": "[SEP]",
  "unk_id": 100,
  "sep_token": "[SEP]",
  "cls_token": "[CLS]",
  "pad_token_id": 0,
  "sep_token_id": 102,
  "cls_token_id": 101,
  "mask_token": "[MASK]",
  "mask_token_id": 103
}"#;
        let tokenizer = Tokenizer::from_str(vocab_str).expect("Embedded vocab load");
        Self { guard, llm_model: Arc::new(model), tokenizer: Arc::new(tokenizer), device }
    }
}

#[async_trait]
impl Agent for EthicalAgent {
    async fn execute(&self, task: &str) -> Result<String, String> {
        self.guard.check_command(task).await?;
        let tokens = self.tokenizer.encode(task, true).map_err(|e| e.to_string())?;
        let input_ids = Tensor::new(&self.device, &tokens.get_ids()).map_err(|e| e.to_string())?;
        let outputs = self.llm_model.forward(&input_ids.unsqueeze(0)).map_err(|e| e.to_string())?;
        let bias_score = outputs.last_hidden_state().mean(1).to_scalar::<f32>().map_err(|e| e.to_string())?;
        if bias_score > 0.5 { return Err("LLM-detected bias".to_string()); }
        Ok(format!("Ethically approved (score: {:.2}): {}", bias_score, task))
    }
}

pub struct ComputeAgent {
    vulkan: Option<Arc<VulkanInstance>>,
}

impl ComputeAgent {
    pub fn new(vulkan: Option<Arc<VulkanInstance>>) -> Self {
        Self { vulkan }
    }
}

#[async_trait]
impl Agent for ComputeAgent {
    async fn execute(&self, task: &str) -> Result<String, String> {
        match &self.vulkan {
            Some(inst) => {
                let processed = super::gpu::run_compute_shader(inst, task).await;
                Ok(format!("GPU processed: {}", processed))
            },
            None => Ok(format!("CPU fallback processed: {}", task)),
        }
    }
}

pub async fn orchestrate(agents: Vec<Arc<dyn Agent>>, task: &str) -> Result<String, String> {
    let mut handles = vec![];
    for agent in agents {
        let t = task.to_string();
        handles.push(tokio::spawn(async move { agent.execute(&t).await }));
    }
    let mut results = vec![];
    for handle in handles {
        match handle.await {
            Ok(Ok(r)) => results.push(r),
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err("Agent panic".to_string()),
        }
    }
    Ok(results.join(" | "))
}