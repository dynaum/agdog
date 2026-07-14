//! Application state and the tick/event loop.

use crate::attribute::attribute;
use crate::classify::classify;
use crate::collect::gpu::{GpuCollector, GpuSample, default_gpu_collector};
use crate::collect::system::SystemCollector;
use crate::model::{Agent, AgentKind, AgentState, Event, EventKind, ResourceSample};
use crate::socket::{EventServer, socket_path};
use crate::ui;
use anyhow::Result;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::collections::{HashMap, HashSet};
use std::io::stdout;
use std::time::{Duration, Instant};

/// Aggregate counts and totals shown in the summary strip.
#[derive(Debug, Clone, Default)]
pub struct Summary {
    pub working: usize,
    pub idle: usize,
    pub stuck: usize,
    pub runaway_crashed: usize,
    pub total_cpu: f32,
    pub total_gpu: f32,
    pub used_mem: u64,
    pub total_mem: u64,
}

/// Column the agents table is sorted by; cycled with `s`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    #[default]
    Gpu,
    Cpu,
    Mem,
    Cost,
    Name,
}

impl SortKey {
    /// The next key in the cycle.
    pub fn next(self) -> Self {
        match self {
            SortKey::Gpu => SortKey::Cpu,
            SortKey::Cpu => SortKey::Mem,
            SortKey::Mem => SortKey::Cost,
            SortKey::Cost => SortKey::Name,
            SortKey::Name => SortKey::Gpu,
        }
    }

    /// Short label for the footer.
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Gpu => "gpu",
            SortKey::Cpu => "cpu",
            SortKey::Mem => "mem",
            SortKey::Cost => "cost",
            SortKey::Name => "name",
        }
    }
}

/// Cost of `gpu_secs` seconds of GPU time at `rate_per_hour` dollars/hour.
pub fn cost_for(gpu_secs: u64, rate_per_hour: f64) -> f64 {
    gpu_secs as f64 / 3600.0 * rate_per_hour
}

/// Sort agents in place by the given key (descending for metrics, A-Z for name).
pub fn sort_agents(agents: &mut [Agent], key: SortKey) {
    use std::cmp::Ordering;
    agents.sort_by(|a, b| match key {
        SortKey::Gpu => b.gpu_pct.partial_cmp(&a.gpu_pct).unwrap_or(Ordering::Equal),
        SortKey::Cpu => b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap_or(Ordering::Equal),
        SortKey::Mem => b.mem_bytes.cmp(&a.mem_bytes),
        SortKey::Cost => b
            .cost_usd
            .partial_cmp(&a.cost_usd)
            .unwrap_or(Ordering::Equal),
        SortKey::Name => a.id.cmp(&b.id),
    });
}

/// Top-level application state.
pub struct App {
    /// Filtered, sorted view shown in the table and used for selection.
    pub agents: Vec<Agent>,
    /// Full unfiltered agent set (summary and events are computed over this).
    pub all_agents: Vec<Agent>,
    pub selected: usize,
    pub quit: bool,
    pub summary: Summary,
    pub sort: SortKey,
    pub filter: String,
    pub filtering: bool,
    /// Latest per-process samples from the system collector.
    pub samples: Vec<ResourceSample>,
    /// Latest per-device GPU samples.
    pub gpus: Vec<GpuSample>,
    /// Seconds per tick (from `--interval`).
    pub interval: u64,
    /// GPU cost rate in dollars per hour (from `--gpu-hourly`).
    pub rate_per_hour: f64,
    env_tags: HashMap<u32, String>,
    gpu: Box<dyn GpuCollector>,
    system: SystemCollector,
    /// Event broadcaster; None in tests and until `run` starts it.
    pub server: Option<EventServer>,
    tick_count: u64,
}

impl App {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            all_agents: Vec::new(),
            selected: 0,
            quit: false,
            summary: Summary::default(),
            sort: SortKey::default(),
            filter: String::new(),
            filtering: false,
            samples: Vec::new(),
            gpus: Vec::new(),
            interval: 1,
            rate_per_hour: 0.0,
            env_tags: HashMap::new(),
            gpu: default_gpu_collector(),
            system: SystemCollector::new(),
            server: None,
            tick_count: 0,
        }
    }

    /// Rebuild the visible `agents` from `all_agents` applying the current
    /// filter and sort. Cheap; called on tick and on key changes.
    fn refresh_view(&mut self) {
        let mut v: Vec<Agent> = if self.filter.is_empty() {
            self.all_agents.clone()
        } else {
            let f = self.filter.to_lowercase();
            self.all_agents
                .iter()
                .filter(|a| a.id.to_lowercase().contains(&f))
                .cloned()
                .collect()
        };
        sort_agents(&mut v, self.sort);
        self.agents = v;
        if !self.agents.is_empty() && self.selected >= self.agents.len() {
            self.selected = self.agents.len() - 1;
        }
    }

    /// Diff the previous and current full agent sets and broadcast events.
    fn emit_events_over(&self, prev: &[Agent]) {
        let Some(server) = &self.server else {
            return;
        };
        let prev_states: HashMap<&str, AgentState> =
            prev.iter().map(|a| (a.id.as_str(), a.state)).collect();
        for a in &self.all_agents {
            match prev_states.get(a.id.as_str()) {
                None => server.broadcast(&Event {
                    kind: EventKind::Started,
                    agent_id: a.id.clone(),
                    from: None,
                    to: a.state,
                    ts_secs: self.tick_count,
                }),
                Some(&ps) if ps != a.state => server.broadcast(&Event {
                    kind: EventKind::StateChanged,
                    agent_id: a.id.clone(),
                    from: Some(ps),
                    to: a.state,
                    ts_secs: self.tick_count,
                }),
                _ => {}
            }
        }
        let now_ids: HashSet<&str> = self.all_agents.iter().map(|a| a.id.as_str()).collect();
        for p in prev {
            if !now_ids.contains(p.id.as_str()) {
                server.broadcast(&Event {
                    kind: EventKind::Exited,
                    agent_id: p.id.clone(),
                    from: Some(p.state),
                    to: AgentState::Crashed,
                    ts_secs: self.tick_count,
                });
            }
        }
    }

    /// Advance one tick: sample real processes and the GPU, fold into agents,
    /// classify, and recompute the summary.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        let gpu_samples = self.gpu.sample();
        let mut samples = self.system.sample();

        // Merge any per-pid GPU data (real backends) into the process samples.
        let mut vram_by_pid: HashMap<u32, (u64, f32)> = HashMap::new();
        for g in &gpu_samples {
            for (pid, vram, util) in &g.per_pid {
                let e = vram_by_pid.entry(*pid).or_insert((0, 0.0));
                e.0 += *vram;
                e.1 = e.1.max(*util);
            }
        }
        for s in samples.iter_mut() {
            if let Some((vram, util)) = vram_by_pid.get(&s.pid) {
                s.vram_bytes = *vram;
                s.gpu_pct = *util;
            }
        }

        let prev = std::mem::take(&mut self.all_agents);
        self.all_agents = build_agents(
            &samples,
            &prev,
            &self.env_tags,
            self.interval,
            self.rate_per_hour,
        );
        self.summary = summarize(
            &self.all_agents,
            self.system.total_cpu_pct(),
            self.system.total_mem(),
            &gpu_samples,
        );
        self.emit_events_over(&prev);
        self.samples = samples;
        self.gpus = gpu_samples;
        self.refresh_view();
    }

    /// Handle one key press.
    pub fn on_key(&mut self, key: KeyCode) {
        if self.filtering {
            match key {
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.refresh_view();
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.refresh_view();
                }
                KeyCode::Enter | KeyCode::Esc => self.filtering = false,
                _ => {}
            }
            return;
        }
        match key {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('s') => {
                self.sort = self.sort.next();
                self.refresh_view();
            }
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter.clear();
                self.refresh_view();
            }
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

/// Fold per-process samples into agents grouped by attribution, classify each
/// against its previous snapshot, and sort by GPU usage descending.
pub fn build_agents(
    samples: &[ResourceSample],
    prev: &[Agent],
    env_tags: &HashMap<u32, String>,
    tick_secs: u64,
    rate_per_hour: f64,
) -> Vec<Agent> {
    let by_pid: HashMap<u32, ResourceSample> = samples.iter().map(|s| (s.pid, s.clone())).collect();
    let prev_by_id: HashMap<&str, &Agent> = prev.iter().map(|a| (a.id.as_str(), a)).collect();

    let mut groups: HashMap<String, Agent> = HashMap::new();
    for s in samples {
        let tag = env_tags.get(&s.pid).map(|x| x.as_str());
        let attr = attribute(s, &by_pid, tag);
        let entry = groups
            .entry(attr.agent_id.clone())
            .or_insert_with(|| Agent {
                id: attr.agent_id.clone(),
                kind: attr.kind,
                ..Default::default()
            });
        if entry.kind == AgentKind::Unknown && attr.kind != AgentKind::Unknown {
            entry.kind = attr.kind;
        }
        entry.pids.push(s.pid);
        entry.cpu_pct += s.cpu_pct;
        entry.mem_bytes += s.rss_bytes;
        entry.vram_bytes += s.vram_bytes;
        entry.gpu_pct = entry.gpu_pct.max(s.gpu_pct);
        if entry.task.is_empty() {
            entry.task = s.cmd.clone();
        }
    }

    let mut out: Vec<Agent> = groups
        .into_values()
        .map(|mut a| {
            let prev_agent = prev_by_id.get(a.id.as_str()).copied();
            let held = prev_agent.map(|p| p.since_secs).unwrap_or(0);
            let new_state = classify(prev_agent, &a, held);
            a.since_secs = match prev_agent {
                Some(p) if p.state == new_state => p.since_secs + tick_secs,
                _ => 0,
            };
            a.state = new_state;
            let prev_cost = prev_agent.map(|p| p.cost_usd).unwrap_or(0.0);
            a.cost_usd = prev_cost
                + if a.gpu_pct > 5.0 {
                    cost_for(tick_secs, rate_per_hour)
                } else {
                    0.0
                };
            if let Some(p) = prev_agent {
                a.history = p.history.clone();
            }
            a.history.push(a.gpu_pct);
            if a.history.len() > 60 {
                a.history.remove(0);
            }
            a
        })
        .collect();

    out.sort_by(|a, b| {
        b.gpu_pct
            .partial_cmp(&a.gpu_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.cpu_pct
                    .partial_cmp(&a.cpu_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    out
}

/// Compute the aggregate summary from the agent list and system totals.
pub fn summarize(agents: &[Agent], cpu: f32, mem: (u64, u64), gpus: &[GpuSample]) -> Summary {
    let mut s = Summary {
        total_cpu: cpu,
        used_mem: mem.0,
        total_mem: mem.1,
        ..Default::default()
    };
    for a in agents {
        match a.state {
            AgentState::Working => s.working += 1,
            AgentState::Idle => s.idle += 1,
            AgentState::Stuck => s.stuck += 1,
            AgentState::Runaway | AgentState::Crashed => s.runaway_crashed += 1,
        }
    }
    if !gpus.is_empty() {
        s.total_gpu = gpus.iter().map(|g| g.util_pct).sum::<f32>() / gpus.len() as f32;
    }
    s
}

/// Run the terminal UI loop until the user quits. Restores the terminal on exit.
pub fn run(interval_secs: u64, rate_per_hour: f64) -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();
    app.interval = interval_secs.max(1);
    app.rate_per_hour = rate_per_hour;
    app.server = EventServer::start(socket_path()).ok();
    app.tick();

    let tick_rate = Duration::from_secs(app.interval);
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
    fn tick_collects_real_process_samples() {
        let mut app = App::new();
        app.tick();
        assert!(!app.samples.is_empty());
        assert!(app.samples.iter().any(|s| s.pid == std::process::id()));
    }

    #[test]
    fn tick_builds_agents_and_summary() {
        let mut app = App::new();
        app.tick();
        assert!(!app.all_agents.is_empty());
        assert!(!app.agents.is_empty());
        assert!(app.summary.total_mem > 0);
    }

    #[test]
    fn cost_for_scales_with_time_and_rate() {
        assert!((cost_for(3600, 2.0) - 2.0).abs() < 1e-9);
        assert!((cost_for(1800, 2.0) - 1.0).abs() < 1e-9);
        assert_eq!(cost_for(1000, 0.0), 0.0);
    }

    #[test]
    fn sort_agents_orders_by_key() {
        let mut v = vec![
            Agent {
                id: "b".into(),
                cpu_pct: 10.0,
                gpu_pct: 5.0,
                ..Default::default()
            },
            Agent {
                id: "a".into(),
                cpu_pct: 90.0,
                gpu_pct: 1.0,
                ..Default::default()
            },
        ];
        sort_agents(&mut v, SortKey::Cpu);
        assert_eq!(v[0].id, "a");
        sort_agents(&mut v, SortKey::Name);
        assert_eq!(v[0].id, "a");
        sort_agents(&mut v, SortKey::Gpu);
        assert_eq!(v[0].id, "b");
    }
}
