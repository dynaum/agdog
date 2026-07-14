//! Core data model: agents, resource samples, and events.

use serde::{Deserialize, Serialize};

/// What kind of work an agent is doing, inferred from its command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Render,
    Train,
    Infer,
    Coding,
    Unknown,
}

impl Default for AgentKind {
    fn default() -> Self {
        AgentKind::Unknown
    }
}

/// Semantic state of an agent, derived by the classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    Working,
    #[default]
    Idle,
    Stuck,
    Runaway,
    Crashed,
}

/// A single process observation for one tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceSample {
    pub pid: u32,
    pub ppid: u32,
    pub cpu_pct: f32,
    pub rss_bytes: u64,
    pub gpu_pct: f32,
    pub vram_bytes: u64,
    pub gpu_index: Option<u32>,
    pub cmd: String,
}

/// An agent: one or more processes grouped by attribution, with folded metrics.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub kind: AgentKind,
    pub state: AgentState,
    pub pids: Vec<u32>,
    pub cpu_pct: f32,
    pub mem_bytes: u64,
    pub gpu_pct: f32,
    pub vram_bytes: u64,
    pub cost_usd: f64,
    pub since_secs: u64,
    pub task: String,
    /// Recent GPU-util samples for the sparkline (oldest first).
    pub history: Vec<f32>,
}

/// The kind of lifecycle event emitted over the socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Started,
    StateChanged,
    Exited,
    PressureWarning,
}

/// A state-change event broadcast to socket subscribers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub kind: EventKind,
    pub agent_id: String,
    pub from: Option<AgentState>,
    pub to: AgentState,
    pub ts_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_kind_serializes_lowercase() {
        let j = serde_json::to_string(&AgentKind::Train).unwrap();
        assert_eq!(j, "\"train\"");
    }

    #[test]
    fn state_default_is_idle() {
        assert_eq!(AgentState::default(), AgentState::Idle);
    }

    #[test]
    fn event_roundtrips_through_json() {
        let ev = Event {
            kind: EventKind::StateChanged,
            agent_id: "kohya-lora".into(),
            from: Some(AgentState::Working),
            to: AgentState::Stuck,
            ts_secs: 42,
        };
        let j = serde_json::to_string(&ev).unwrap();
        let back: Event = serde_json::from_str(&j).unwrap();
        assert_eq!(ev, back);
    }
}
