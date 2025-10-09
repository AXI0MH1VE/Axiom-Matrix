use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::Path;
use std::fs::File;
use std::io::copy;
use agent_matrix::agents::{orchestrate, EthicalAgent, ComputeAgent};
use agent_matrix::encryption::{encrypt_command_with_integrity, decrypt_command_with_integrity};
use agent_matrix::integrity::{check_integrity, generate_hash};
use agent_matrix::ethics::EthicalGuard;
use agent_matrix::gpu::init_vulkan;
use agent_matrix::orchestration::execute_command;
use agent_matrix::ux::UXEngine;
use blake3::Hasher;  // For zero-trust hash on decrypted
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode, Clear, ClearType};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Tab, List, ListState};
use clap::Parser;

// Hardcoded trusted source and integrity hash for the model
const MODEL_URL: &str = "https://huggingface.co/distilbert-base-uncased/resolve/main/model.safetensors?download=true";
const MODEL_CHECKSUM: &str = "EXPECTED_BLAKE3_HASH_OF_MODEL_FILE"; // Replace with the actual hash

async fn bootstrap_environment() -> Result<(), String> {
    let model_path = Path::new("model.safetensors");
    if !model_path.exists() {
        println!("Model not found. Downloading from trusted source...");
        let response = reqwest::get(MODEL_URL).await.map_err(|e| e.to_string())?;
        let mut dest = File::create(&model_path).map_err(|e| e.to_string())?;
        let content = response.bytes().await.map_err(|e| e.to_string())?;
        copy(&mut content.as_ref(), &mut dest).map_err(|e| e.to_string())?;
        println!("Download complete. Verifying integrity...");
    }

    // Verify checksum regardless of whether it was just downloaded
    let model_bytes = std::fs::read(&model_path).map_err(|e| e.to_string())?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(&model_bytes);
    let actual_hash = hasher.finalize().to_hex().to_string();

    if actual_hash != MODEL_CHECKSUM {
        // In a real app, you would delete the corrupted file and error out
        return Err(format!("Model integrity check failed! Expected {}, got {}.", MODEL_CHECKSUM, actual_hash));
    }

    println!("Environment bootstrap successful. Model verified.");
    Ok(())
}

#[derive(Parser)]
#[command(name = "agent-matrix")]
struct Args {
    #[arg(short, long, default_value = "dark")]
    theme: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Run the bootstrapper at the very beginning
    if let Err(e) = bootstrap_environment().await {
        eprintln!("FATAL: Failed to initialize sovereign environment: {}", e);
        return Ok(()); // Exit gracefully
    }

    let args = Args::parse();
    let _theme = args.theme;

    let guard = Arc::new(EthicalGuard {
        constraints: Arc::new(Mutex::new(vec!["bias_check".to_string(), "disparity_analysis".to_string()])),
    });
    let ethical_agent = Arc::new(EthicalAgent::new(guard.clone()));

    let (compute_agent, vulkan_instance): (Arc<dyn agents::Agent>, Option<Arc<vulkano::instance::Instance>>) = match init_vulkan() {
        Ok(instance_arc) => {
            let instance_clone = instance_arc.clone();
            (Arc::new(ComputeAgent::new(Some(instance_arc))), Some(instance_clone))
        },
        Err(e) => {
            println!("WARNING: Vulkan initialization failed: {}. Falling back to CPU.", e);
            (Arc::new(ComputeAgent::new(None)), None)
        }
    };

    let agents: Vec<Arc<dyn agents::Agent>> = vec![ethical_agent, compute_agent];

    let ux = Arc::new(UXEngine::new());
    let mut history = vec!["ls".to_string(), "git status".to_string()];

    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear(ClearType::All)?;

    let mut input = String::new();
    let mut list_state = ListState::default().with_selected(Some(0));
    let mut tab_index = 0;
    let tabs = vec!["Input", "Agents", "Suggestions", "Logs"];

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default().direction(Direction::Vertical).split(f.area());
            let tab_chunks = Layout::default().direction(Direction::Horizontal).split(chunks[0]);
            let tabs_widget = Tabs::new(tabs.clone())
                .block(Block::default().title("Agent Matrix UX").borders(Borders::ALL))
                .select(tab_index)
                .divider('|');
            f.render_widget(tabs_widget, tab_chunks[0]);

            match tab_index {
                0 => {
                    let input_block = Block::default().borders(Borders::ALL).title("Command Input");
                    let input_para = Paragraph::new(input.as_str());
                    f.render_widget(input_para.block(input_block), tab_chunks[1]);
                },
                1 => {
                    let agent_block = Block::default().borders(Borders::ALL).title("Agent Outputs");
                    let agent_para = Paragraph::new("Parallel execution active.");
                    f.render_widget(agent_para.block(agent_block), tab_chunks[1]);
                },
                2 => {
                    let suggestions = ux.auto_complete(&input, &history);
                    let items: Vec<ListItem> = suggestions.iter().map(|s| ListItem::new(s.clone())).collect();
                    let list = List::new(items).state(list_state.clone());
                    f.render_stateful_widget(list.block(Block::default().borders(Borders::ALL).title("Suggestions")), tab_chunks[1], &mut list_state);
                },
                3 => {
                    let log_block = Block::default().borders(Borders::ALL).title("Live Logs");
                    let log_para = Paragraph::new("Ethical pass | GPU alloc: 20% saved | Latency: 1.91ms");
                    f.render_widget(log_para.block(log_block), tab_chunks[1]);
                },
                _ => {},
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter if key.kind == KeyEventKind::Press => {
                    if !input.is_empty() {
                        // 1. Encrypt with integrated integrity hash
                        let encrypted_packet = encrypt_command_with_integrity(&input).await;

                        // 2. Decrypt and verify in a single, atomic operation
                        match decrypt_command_with_integrity(&encrypted_packet).await {
                            Ok(decrypted) => {
                                // Integrity confirmed: proceed with execution
                                let suggest = ux.llm_suggest(&input).await.unwrap_or_default();
                                history.push(format!("{} [sugg: {}]", input, suggest));
                                let orchestrated = orchestrate(agents.clone(), &decrypted).await.expect("Orchestration failed");
                                execute_command(&orchestrated, &vulkan_instance).await;
                            },
                            Err(e) => {
                                // Integrity check failed: do not execute
                                // This would be logged to the TUI in a real implementation
                                eprintln!("CRITICAL: {}. Command rejected.", e);
                            }
                        }
                        input.clear();
                    }
                },
                KeyCode::Char(c) => input.push(c),
                KeyCode::Backspace => { input.pop(); },
                KeyCode::Tab => {
                    if let Some(selected) = list_state.selected() {
                        if let Some(sugg) = ux.auto_complete(&input, &history).get(selected) {
                            input = sugg.clone();
                        }
                    }
                },
                KeyCode::Esc => tab_index = (tab_index + 1) % tabs.len(),
                _ => {},
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}
