//! Demo mode: curated mock agents (and the mock GPU) for screenshots, the
//! README GIF, and a first run on a machine with no agents. Enabled with
//! `AGDOG_DEMO=1`; that env var also forces the mock GPU backend (see `gpu.rs`).

use crate::model::{Agent, AgentKind, AgentState, SubAgent, SubSource};

/// System totals shown in the summary strip while in demo mode.
pub const DEMO_CPU: f32 = 57.0;
pub const DEMO_USED_MEM: u64 = 185_000_000_000;
pub const DEMO_TOTAL_MEM: u64 = 512_000_000_000;

/// Whether demo mode is on.
pub fn enabled() -> bool {
    std::env::var_os("AGDOG_DEMO").is_some()
}

/// A believable per-core CPU heatmap that varies each tick.
pub fn demo_cpu_cores(tick: u64) -> Vec<f32> {
    (0..16u64)
        .map(|i| {
            let mix = tick.wrapping_add(i).wrapping_mul(2654435761);
            (mix % 100) as f32
        })
        .collect()
}

fn sub(name: &str, state: AgentState) -> SubAgent {
    SubAgent {
        name: name.to_string(),
        state,
        source: SubSource::Transcript,
        cpu_pct: 0.0,
        mem_bytes: 0,
        task: String::new(),
    }
}

/// The curated agent set, jittered per tick and carrying sparkline history.
pub fn demo_agents(tick: u64, prev: &[Agent]) -> Vec<Agent> {
    let jitter = |base: f32, seed: u64| -> f32 {
        let mix = tick.wrapping_add(seed).wrapping_mul(2654435761) % 8;
        (base + mix as f32 - 4.0).clamp(0.0, 100.0)
    };

    let mut agents = vec![
        make(
            "comfyui:flux",
            AgentKind::Render,
            AgentState::Working,
            jitter(40.0, 1),
            4_100_000_000,
            jitter(94.0, 2),
            14_200_000_000,
            1.20,
            9012,
            "flux1-dev 1024 · batch 8/16",
            vec![],
        ),
        make(
            "kohya:sdxl-lora",
            AgentKind::Train,
            AgentState::Working,
            jitter(60.0, 3),
            6_800_000_000,
            jitter(88.0, 4),
            9_100_000_000,
            3.10,
            13342,
            "sdxl-lora · ep3 step 148/210",
            vec![],
        ),
        make(
            "claude:agdog",
            AgentKind::Coding,
            AgentState::Working,
            jitter(30.0, 5),
            700_000_000,
            0.0,
            0,
            0.0,
            4521,
            "claude --enable-auto-mode",
            vec![
                sub("explore", AgentState::Working),
                sub("code-review", AgentState::Working),
            ],
        ),
        make(
            "vllm:llama-70b",
            AgentKind::Infer,
            AgentState::Runaway,
            jitter(20.0, 6),
            3_000_000_000,
            99.0,
            71_100_000_000,
            2.66,
            22770,
            "99% for 22m · 0 tokens emitted",
            vec![],
        ),
        make(
            "ollama:mixtral",
            AgentKind::Infer,
            AgentState::Idle,
            1.0,
            0,
            2.0,
            0,
            0.0,
            1087,
            "serving on :11434",
            vec![],
        ),
        make(
            "whisper:batch",
            AgentKind::Coding,
            AgentState::Stuck,
            1.0,
            200_000_000,
            0.0,
            12_000_000_000,
            0.90,
            20344,
            "vram held · 0% util · 4m",
            vec![],
        ),
    ];

    // Carry the GPU-util sparkline history across ticks.
    for a in &mut agents {
        if let Some(p) = prev.iter().find(|p| p.id == a.id) {
            a.history = p.history.clone();
        }
        a.history.push(a.gpu_pct);
        if a.history.len() > 60 {
            a.history.remove(0);
        }
    }
    agents
}

#[allow(clippy::too_many_arguments)]
fn make(
    id: &str,
    kind: AgentKind,
    state: AgentState,
    cpu: f32,
    mem: u64,
    gpu: f32,
    vram: u64,
    cost: f64,
    pid: u32,
    task: &str,
    subagents: Vec<SubAgent>,
) -> Agent {
    Agent {
        id: id.to_string(),
        kind,
        state,
        pids: vec![pid],
        cpu_pct: cpu,
        mem_bytes: mem,
        gpu_pct: gpu,
        vram_bytes: vram,
        cost_usd: cost,
        since_secs: 3600,
        task: task.to_string(),
        subagents,
        ..Default::default()
    }
}
