use agdog::app::build_agents;
use agdog::model::{AgentKind, AgentState, ResourceSample};
use std::collections::HashMap;

fn s(pid: u32, ppid: u32, cmd: &str, cpu: f32, rss: u64) -> ResourceSample {
    ResourceSample {
        pid,
        ppid,
        cpu_pct: cpu,
        rss_bytes: rss,
        gpu_pct: 0.0,
        vram_bytes: 0,
        gpu_index: None,
        cmd: cmd.to_string(),
    }
}

#[test]
fn groups_by_agent_id_and_sums_metrics() {
    let samples = vec![
        s(10, 1, "python comfyui/main.py", 30.0, 1_000_000_000),
        s(11, 10, "python comfyui worker", 10.0, 500_000_000),
        s(20, 1, "some random daemon", 1.0, 100_000_000),
    ];
    let agents = build_agents(&samples, &[], &HashMap::new(), 1);

    let comfy = agents
        .iter()
        .find(|a| a.id == "comfyui")
        .expect("comfyui group");
    assert_eq!(comfy.kind, AgentKind::Render);
    assert!((comfy.cpu_pct - 40.0).abs() < 1e-3);
    assert_eq!(comfy.mem_bytes, 1_500_000_000);
    assert!(comfy.pids.contains(&10) && comfy.pids.contains(&11));

    assert!(agents.iter().any(|a| a.id == "unassigned"));
}

#[test]
fn active_group_classifies_working() {
    let samples = vec![s(10, 1, "python comfyui/main.py", 50.0, 1_000_000_000)];
    let agents = build_agents(&samples, &[], &HashMap::new(), 1);
    let comfy = agents.iter().find(|a| a.id == "comfyui").unwrap();
    assert_eq!(comfy.state, AgentState::Working);
}

#[test]
fn state_persists_across_ticks_and_accumulates_time() {
    let samples = vec![s(10, 1, "python comfyui/main.py", 50.0, 1_000_000_000)];
    let first = build_agents(&samples, &[], &HashMap::new(), 1);
    let second = build_agents(&samples, &first, &HashMap::new(), 1);
    let comfy = second.iter().find(|a| a.id == "comfyui").unwrap();
    assert_eq!(comfy.state, AgentState::Working);
    assert_eq!(comfy.since_secs, 1); // one tick of continuous Working
}
