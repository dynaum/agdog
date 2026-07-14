//! Semantic state classifier (working/idle/stuck/runaway/crashed).

use crate::model::{Agent, AgentState};

/// Classify an agent's semantic state from its current metrics, how long it has
/// held them (`held_secs`), and its previous snapshot.
///
/// Precedence: crashed (no live pids) > runaway (pegged and sustained) >
/// working (active) > stuck (memory held with no activity) > idle.
pub fn classify(prev: Option<&Agent>, cur: &Agent, held_secs: u64) -> AgentState {
    if cur.pids.is_empty() {
        return AgentState::Crashed;
    }
    let util = cur.gpu_pct.max(cur.cpu_pct);
    let prev_util = prev.map(|p| p.gpu_pct.max(p.cpu_pct)).unwrap_or(util);

    if util >= 98.0 && prev_util >= 98.0 && held_secs >= 300 {
        return AgentState::Runaway;
    }
    if util > 5.0 {
        return AgentState::Working;
    }
    if cur.vram_bytes > 0 && held_secs >= 60 {
        return AgentState::Stuck;
    }
    AgentState::Idle
}
