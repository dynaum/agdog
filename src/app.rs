//! Application state and the tick/event loop.

use crate::collect::gpu::{GpuCollector, default_gpu_collector};
use crate::collect::system::SystemCollector;
use crate::model::{Agent, AgentKind, AgentState, ResourceSample};
use crate::ui;
use anyhow::Result;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::stdout;
use std::time::{Duration, Instant};

/// Top-level application state.
pub struct App {
    pub agents: Vec<Agent>,
    pub selected: usize,
    pub quit: bool,
    /// Latest per-process samples from the system collector (grouped in Task 8).
    pub samples: Vec<ResourceSample>,
    gpu: Box<dyn GpuCollector>,
    system: SystemCollector,
    tick_count: u64,
}

impl App {
    pub fn new() -> Self {
        Self {
            agents: seed_agents(),
            selected: 0,
            quit: false,
            samples: Vec::new(),
            gpu: default_gpu_collector(),
            system: SystemCollector::new(),
            tick_count: 0,
        }
    }

    /// Advance one tick: refresh real process samples and live GPU values.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.samples = self.system.sample();
        let samples = self.gpu.sample();
        for (a, s) in self.agents.iter_mut().zip(samples.iter().cycle()) {
            a.gpu_pct = s.util_pct;
            a.vram_bytes = s.mem_used;
            a.history.push(s.util_pct);
            if a.history.len() > 60 {
                a.history.remove(0);
            }
        }
    }

    /// Handle one key press.
    pub fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Down => {
                if self.selected + 1 < self.agents.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            _ => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Fabricate a handful of agents so the skeleton renders something live.
/// Replaced by real attribution in Task 8.
fn seed_agents() -> Vec<Agent> {
    let defs = [
        (
            "comfyui-flux",
            AgentKind::Render,
            AgentState::Working,
            "flux1-dev 1024 batch 8/16",
        ),
        (
            "kohya-lora",
            AgentKind::Train,
            AgentState::Working,
            "sdxl-lora ep3 step 148/210",
        ),
        (
            "ollama-llama70b",
            AgentKind::Infer,
            AgentState::Idle,
            "llama3.3-70b serving",
        ),
        (
            "claude-code#3",
            AgentKind::Coding,
            AgentState::Stuck,
            "waiting on git lock 8m12s",
        ),
        (
            "render-batch",
            AgentKind::Render,
            AgentState::Runaway,
            "99% for 22m",
        ),
    ];
    defs.iter()
        .enumerate()
        .map(|(i, (id, kind, state, task))| Agent {
            id: (*id).to_string(),
            kind: *kind,
            state: *state,
            pids: vec![20000 + i as u32],
            cpu_pct: 20.0 + i as f32 * 8.0,
            mem_bytes: (2 + i as u64) * 1_000_000_000,
            gpu_pct: 0.0,
            vram_bytes: 0,
            cost_usd: 0.5 * (i as f64 + 1.0),
            since_secs: 3600 * (i as u64 + 1),
            task: (*task).to_string(),
            history: Vec::new(),
        })
        .collect()
}

/// Run the terminal UI loop until the user quits. Restores the terminal on exit.
pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();

    let tick_rate = Duration::from_secs(1);
    let mut last_tick = Instant::now();

    let res = (|| -> Result<()> {
        loop {
            terminal.draw(|f| ui::render(f, &app))?;
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let CEvent::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press {
                        app.on_key(k.code);
                    }
                }
            }
            if last_tick.elapsed() >= tick_rate {
                app.tick();
                last_tick = Instant::now();
            }
            if app.quit {
                break;
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_sets_quit() {
        let mut app = App::new();
        app.on_key(KeyCode::Char('q'));
        assert!(app.quit);
    }

    #[test]
    fn down_moves_selection_and_saturates() {
        let mut app = App::new();
        let n = app.agents.len();
        for _ in 0..n + 3 {
            app.on_key(KeyCode::Down);
        }
        assert_eq!(app.selected, n - 1);
    }

    #[test]
    fn tick_populates_gpu_history() {
        let mut app = App::new();
        app.tick();
        assert!(app.agents.iter().all(|a| !a.history.is_empty()));
    }

    #[test]
    fn tick_collects_real_process_samples() {
        let mut app = App::new();
        app.tick();
        assert!(!app.samples.is_empty());
        assert!(app.samples.iter().any(|s| s.pid == std::process::id()));
    }
}
