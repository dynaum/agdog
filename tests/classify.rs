use agdog::classify::classify;
use agdog::model::{Agent, AgentState};

/// A live agent with one pid and otherwise-default metrics.
fn agent() -> Agent {
    Agent {
        pids: vec![1],
        ..Default::default()
    }
}

#[test]
fn no_pids_is_crashed() {
    let a = Agent {
        pids: vec![],
        ..Default::default()
    };
    assert_eq!(classify(None, &a, 10), AgentState::Crashed);
}

#[test]
fn active_util_is_working() {
    let a = Agent {
        gpu_pct: 40.0,
        ..agent()
    };
    assert_eq!(classify(None, &a, 10), AgentState::Working);
}

#[test]
fn held_memory_no_activity_is_stuck() {
    let a = Agent {
        gpu_pct: 0.0,
        cpu_pct: 1.0,
        vram_bytes: 8_000_000_000,
        ..agent()
    };
    assert_eq!(classify(None, &a, 90), AgentState::Stuck);
}

#[test]
fn pegged_and_sustained_is_runaway() {
    let prev = Agent {
        gpu_pct: 99.0,
        ..agent()
    };
    let cur = Agent {
        gpu_pct: 99.0,
        ..agent()
    };
    assert_eq!(classify(Some(&prev), &cur, 400), AgentState::Runaway);
}

#[test]
fn pegged_but_brief_is_working_not_runaway() {
    let prev = Agent {
        gpu_pct: 99.0,
        ..agent()
    };
    let cur = Agent {
        gpu_pct: 99.0,
        ..agent()
    };
    assert_eq!(classify(Some(&prev), &cur, 100), AgentState::Working);
}

#[test]
fn idle_when_quiet_and_no_memory() {
    let a = Agent {
        gpu_pct: 0.0,
        cpu_pct: 0.0,
        vram_bytes: 0,
        ..agent()
    };
    assert_eq!(classify(None, &a, 120), AgentState::Idle);
}
