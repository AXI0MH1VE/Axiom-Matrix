use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use ratatui::widgets::{List, ListItem, ListState};
use std::sync::Arc;
use tokio::sync::Mutex;
use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use tokenizers::Tokenizer;

pub struct UXEngine {
    matcher: SkimMatcherV2,
    suggestions: Arc<Mutex<Vec<String>>>,
    tokenizer: Arc<Tokenizer>,
    device: Device,
}

impl UXEngine {
    pub fn new() -> Self {
        let matcher = SkimMatcherV2::default();
        let device = Device::Cpu;
        // Embedded vocab (no file dep; tiny BERT proxy)
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
        Self { matcher, suggestions: Arc::new(Mutex::new(vec![])), tokenizer: Arc::new(tokenizer), device }
    }

    pub fn auto_complete(&self, input: &str, history: &[String]) -> Vec<String> {
        let mut matches = vec![];
        for cmd in history {
            if self.matcher.fuzzy_match(cmd, input).unwrap_or(0) > 80 {
                matches.push(cmd.clone());
            }
        }
        matches
    }

    pub async fn llm_suggest(&self, input: &str) -> Result<String, String> {
        let tokens = self.tokenizer.encode(input, true).map_err(|e| e.to_string())?;
        let input_ids = Tensor::new(&self.device, &tokens.get_ids()).map_err(|e| e.to_string())?;
        let suggested = format!("Suggested: {}", input);
        Ok(suggested)
    }

    pub fn render_tabs(&self, f: &mut ratatui::Frame, area: ratatui::prelude::Rect, state: &ListState, suggestions: &[String]) {
        let items: Vec<ListItem> = suggestions.iter().map(|s| ListItem::new(s.clone())).collect();
        let list = List::new(items).state(state.clone());
        f.render_stateful_widget(list, area, state);
    }
}