use agdog::attribute::{attribute, kind_from_cmd};
use agdog::model::{AgentKind, ResourceSample};
use std::collections::HashMap;

fn sample(pid: u32, ppid: u32, cmd: &str) -> ResourceSample {
    ResourceSample {
        pid,
        ppid,
        cpu_pct: 0.0,
        rss_bytes: 0,
        gpu_pct: 0.0,
        vram_bytes: 0,
        gpu_index: None,
        cmd: cmd.to_string(),
    }
}

#[test]
fn comfyui_is_render() {
    assert_eq!(kind_from_cmd("python comfyui/main.py"), AgentKind::Render);
}

#[test]
fn kohya_is_train() {
    assert_eq!(
        kind_from_cmd("python kohya_ss train_network.py"),
        AgentKind::Train
    );
}

#[test]
fn env_tag_wins_with_full_confidence() {
    let s = sample(100, 1, "python worker.py");
    let map = HashMap::new();
    let a = attribute(&s, &map, Some("render-batch"));
    assert_eq!(a.agent_id, "render-batch");
    assert_eq!(a.confidence, 1.0);
}

#[test]
fn cmdline_signature_attributes_infer() {
    let s = sample(100, 1, "python -m vllm.serve");
    let map = HashMap::new();
    let a = attribute(&s, &map, None);
    assert_eq!(a.kind, AgentKind::Infer);
    assert!((a.confidence - 0.8).abs() < 1e-6);
}

#[test]
fn parent_tree_inheritance() {
    let parent = sample(50, 1, "python comfyui/main.py");
    let child = sample(100, 50, "python worker_subprocess.py");
    let mut map = HashMap::new();
    map.insert(50, parent);
    let a = attribute(&child, &map, None);
    assert_eq!(a.kind, AgentKind::Render);
    assert!((a.confidence - 0.6).abs() < 1e-6);
}

#[test]
fn unattributed_is_unassigned() {
    let s = sample(100, 1, "some random daemon");
    let map = HashMap::new();
    let a = attribute(&s, &map, None);
    assert_eq!(a.agent_id, "unassigned");
    assert_eq!(a.confidence, 0.0);
}
