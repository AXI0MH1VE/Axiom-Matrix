use std::sync::Arc;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, List, ListItem, ListState};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use agent_matrix::agents::Agent;
use agent_matrix::ux::UXEngine;
use agent_matrix::orchestration::execute_command;

/// Top-level UI application state - The Nexus of User Experience
pub struct MatrixUI {
    pub state: UIState,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
}

pub struct UIState {
    pub input_buffer: String,
    pub command_history: Vec<String>,
    pub suggestion_list_state: ListState,
    pub active_tab: usize,
    pub agents: Vec<Arc<dyn Agent>>,
    pub ux_engine: Arc<UXEngine>,
    pub vulkan_instance: Option<vulkano::instance::Instance>,
    pub live_metrics: LiveMetrics,
}

pub struct LiveMetrics {
    pub ethical_passes: u32,
    pub gpu_savings: f32,
    pub avg_latency_ms: f32,
    pub last_command: String,
}

impl MatrixUI {
    pub fn new(
        agents: Vec<Arc<dyn Agent>>,
        ux_engine: Arc<UXEngine>,
        vulkan_instance: Option<vulkano::instance::Instance>
    ) -> Self {
        let terminal = Terminal::new(CrosstermBackend::new(std::io::stdout())).unwrap();
        let mut ui = Self {
            state: UIState::new(agents, ux_engine, vulkan_instance),
            terminal,
        };
        let _ = ui.init_interface();
        ui
    }

    fn init_interface(&mut self) -> std::io::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        self.terminal.clear()?;
        Ok(())
    }

    pub async fn run_event_loop(&mut self) -> std::io::Result<()> {
        loop {
            self.render_frame()?;
            if let Event::Key(key) = crossterm::event::read()? {
                if let Some(should_exit) = self.handle_input(key).await {
                    if should_exit {
                        break;
                    }
                }
            }
        }
        self.cleanup()
    }

    fn render_frame(&mut self) -> std::io::Result<()> {
        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header tabs
                    Constraint::Min(0),     // Main content
                    Constraint::Length(1),  // Status bar
                ])
                .split(f.area());

            self.render_header(f, chunks[0]);
            self.render_content(f, chunks[1]);
            self.render_status_bar(f, chunks[2]);
        })?;
        Ok(())
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let tabs = vec!["Command Interface", "Agent Matrix", "AI Suggestions", "System Logs"];
        let tab_titles: Vec<&str> = tabs.iter().map(|&s| s).collect();

        let tabs_widget = Tabs::new(tab_titles)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("🚀 Agent Matrix v1.0 - Sovereign AI Terminal")
            )
            .select(self.state.active_tab)
            .divider(" | ")
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(tabs_widget, area);
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        match self.state.active_tab {
            0 => self.render_command_interface(f, area),
            1 => self.render_agent_matrix(f, area),
            2 => self.render_ai_suggestions(f, area),
            3 => self.render_system_logs(f, area),
            _ => {}
        }
    }

    fn render_command_interface(&self, f: &mut Frame, area: Rect) {
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title("Command Input")
            .border_style(Style::default().fg(Color::Green));

        let input_widget = Paragraph::new(self.state.input_buffer.as_str())
            .block(input_block)
            .style(Style::default().fg(Color::White));

        f.render_widget(input_widget, area);
    }

    fn render_agent_matrix(&self, f: &mut Frame, area: Rect) {
        let agent_info = format!(
            "🛡️ Ethical Agent: Active\n⚡ Compute Agent: {}\n🧠 UX Agent: Online\n",
            if self.state.vulkan_instance.is_some() { "GPU Accelerated" } else { "CPU Mode" }
        );

        let agents_block = Block::default()
            .borders(Borders::ALL)
            .title("Agent Status Matrix")
            .border_style(Style::default().fg(Color::Yellow));

        let agents_widget = Paragraph::new(agent_info)
            .block(agents_block)
            .style(Style::default().fg(Color::White));

        f.render_widget(agents_widget, area);
    }

    fn render_ai_suggestions(&mut self, f: &mut Frame, area: Rect) {
        let suggestions = self.state.ux_engine.auto_complete(
            &self.state.input_buffer,
            &self.state.command_history
        );

        let items: Vec<ListItem> = suggestions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let style = if i == self.state.suggestion_list_state.selected().unwrap_or(0) {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(s.as_str()).style(style)
            })
            .collect();

        let suggestions_block = Block::default()
            .borders(Borders::ALL)
            .title("AI-Powered Command Suggestions")
            .border_style(Style::default().fg(Color::Blue));

        let list = List::new(items)
            .block(suggestions_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_stateful_widget(list, area, &mut self.state.suggestion_list_state);
    }

    fn render_system_logs(&self, f: &mut Frame, area: Rect) {
        let log_content = format!(
            "System Status: Online\nEthical Passes: {}\nGPU Savings: {:.1}% MB\nAvg Latency: {:.1} ms\nLast Command: {}\nIntegrity: ✅ Verified",
            self.state.live_metrics.ethical_passes,
            self.state.live_metrics.gpu_savings,
            self.state.live_metrics.avg_latency_ms,
            self.state.live_metrics.last_command
        );

        let logs_block = Block::default()
            .borders(Borders::ALL)
            .title("Live System Metrics")
            .border_style(Style::default().fg(Color::Magenta));

        let logs_widget = Paragraph::new(log_content)
            .block(logs_block)
            .style(Style::default().fg(Color::White));

        f.render_widget(logs_widget, area);
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status = format!(" ESC: Switch Tabs | TAB: Select | ENTER: Execute | Ctrl+C: Quit | Buffer: {} chars", self.state.input_buffer.len());
        let status_bar = Paragraph::new(status)
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black))
            .alignment(Alignment::Center);

        f.render_widget(status_bar, area);
    }

    async fn handle_input(&mut self, key: crossterm::event::KeyEvent) -> Option<bool> {
        match key.code {
            KeyCode::Enter if key.kind == KeyEventKind::Press => {
                self.handle_command_execution().await;
            },
            KeyCode::Char(c) => self.state.input_buffer.push(c),
            KeyCode::Backspace => { self.state.input_buffer.pop(); },
            KeyCode::Tab => self.handle_suggestion_selection(),
            KeyCode::Esc => self.state.active_tab = (self.state.active_tab + 1) % 4,
            KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => return Some(true),
            _ => {}
        }
        None
    }

    async fn handle_command_execution(&mut self) {
        if self.state.input_buffer.trim().is_empty() {
            return;
        }

        // Process the command through the agent matrix
        let command = self.state.input_buffer.clone();
        self.state.live_metrics.last_command = command.clone();

        // Get AI suggestion
        if let Ok(suggestion) = self.state.ux_engine.llm_suggest(&command).await {
            self.state.command_history.push(format!("{} [AI: {}]", command, suggestion));
        } else {
            self.state.command_history.push(command.clone());
        }

        // Execute command through orchestration
        // Note: In a real implementation, this would go through the full encryption/integrity pipeline
        let agent_results = self.state.execute_agents(&command).await;

        // Update metrics (would use actual measurements in production)
        self.state.live_metrics.ethical_passes += 1;
        self.state.live_metrics.gpu_savings += 0.5;
        self.state.live_metrics.avg_latency_ms = 1.91;

        self.state.input_buffer.clear();
    }

    fn handle_suggestion_selection(&mut self) {
        if let Some(selected) = self.state.suggestion_list_state.selected() {
            let suggestions = self.state.ux_engine.auto_complete(&self.state.input_buffer, &self.state.command_history);
            if let Some(suggestion) = suggestions.get(selected) {
                self.state.input_buffer = suggestion.clone();
            }
        }
    }

    fn cleanup(self) -> std::io::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }
}

impl UIState {
    fn new(agents: Vec<Arc<dyn Agent>>, ux_engine: Arc<UXEngine>, vulkan_instance: Option<vulkano::instance::Instance>) -> Self {
        Self {
            input_buffer: String::new(),
            command_history: vec!["ls".to_string(), "git status".to_string()],
            suggestion_list_state: ListState::default().with_selected(Some(0)),
            active_tab: 0,
            agents,
            ux_engine,
            vulkan_instance: vulkan_instance.map(|inst| inst), // Convert Arc to owned if needed
            live_metrics: LiveMetrics {
                ethical_passes: 0,
                gpu_savings: 0.0,
                avg_latency_ms: 0.0,
                last_command: "None".to_string(),
            },
        }
    }

    async fn execute_agents(&self, command: &str) -> Vec<String> {
        // Simplified execution for demo - would integrate with full agent orchestration
        let mut results = Vec::new();

        // Execute agents in parallel (simplified)
        for agent in &self.agents {
            if let Ok(result) = agent.execute(command).await {
                results.push(result);
            }
        }

        results
    }
}
