use agdog::app::build_agents;
use agdog::model::{AgentKind, ResourceSample};
use std::collections::HashMap;

fn proc(pid: u32, ppid: u32, exe: &str, cmd: &str, cpu: f32, rss: u64) -> ResourceSample {
    ResourceSample {
        pid,
        ppid,
        cpu_pct: cpu,
        rss_bytes: rss,
        exe_name: exe.to_string(),
        cmd: cmd.to_string(),
        ..Default::default()
    }
}

#[test]
fn child_processes_roll_into_their_agent_root() {
    let samples = vec![
        proc(
            100,
            1,
            "claude",
            "claude --enable-auto-mode",
            30.0,
            1_000_000_000,
        ),
        proc(101, 100, "node", "node mcp-server.cjs", 5.0, 300_000_000), // child of claude
        proc(102, 101, "python", "python chroma-mcp", 2.0, 200_000_000), // grandchild
        proc(
            900,
            1,
            "ollama",
            "/opt/homebrew/opt/ollama/bin/ollama serve",
            1.0,
            500_000_000,
        ),
        proc(950, 1, "gamed", "/usr/libexec/gamed", 1.0, 100_000_000), // unrelated
    ];
    let agents = build_agents(&samples, &[], &HashMap::new(), 1, 0.0);

    let claude = agents
        .iter()
        .find(|a| a.id == "claude")
        .expect("claude agent");
    assert_eq!(claude.kind, AgentKind::Coding);
    assert!(claude.pids.contains(&100));
    assert!(claude.pids.contains(&101));
    assert!(claude.pids.contains(&102));
    assert!((claude.cpu_pct - 37.0).abs() < 1e-3);

    assert!(agents.iter().any(|a| a.id == "ollama"));
    assert!(
        agents
            .iter()
            .any(|a| a.id == "unassigned" && a.pids.contains(&950))
    );
}

#[test]
fn parallel_sessions_get_distinct_ids_by_cwd() {
    let mut a = proc(100, 1, "claude", "claude", 10.0, 0);
    a.cwd = Some("/Users/x/dev/agdog".to_string());
    let mut b = proc(200, 1, "claude", "claude", 10.0, 0);
    b.cwd = Some("/Users/x/dev/site".to_string());
    let agents = build_agents(&[a, b], &[], &HashMap::new(), 1, 0.0);

    assert!(agents.iter().any(|x| x.id == "claude:agdog"));
    assert!(agents.iter().any(|x| x.id == "claude:site"));
    assert!(agents.iter().all(|x| x.id != "claude"));
}

#[test]
fn child_agent_process_nests_as_subagent() {
    let mut parent = proc(100, 1, "claude", "claude", 5.0, 1_000_000);
    parent.cwd = Some("/x/dev/main".to_string());
    let mut child = proc(200, 100, "claude", "claude -p sub", 3.0, 500_000);
    child.cwd = Some("/x/dev/worker".to_string());

    let agents = build_agents(&[parent, child], &[], &HashMap::new(), 1, 0.0);

    let p = agents
        .iter()
        .find(|a| a.id == "claude:main")
        .expect("parent agent");
    assert_eq!(p.subagents.len(), 1);
    assert_eq!(p.subagents[0].name, "claude:worker");
    // The child is nested, not a separate top-level row.
    assert!(agents.iter().all(|a| a.id != "claude:worker"));
}

#[test]
fn gui_and_system_processes_never_become_agents() {
    let samples = vec![
        proc(
            100,
            1,
            "Claude",
            "/Applications/Claude.app/Contents/MacOS/Claude",
            5.0,
            0,
        ),
        proc(
            101,
            100,
            "Claude Helper",
            "/Applications/Claude.app/Contents/Frameworks/Claude Helper.app/Contents/MacOS/Claude Helper --type=renderer",
            5.0,
            0,
        ),
    ];
    let agents = build_agents(&samples, &[], &HashMap::new(), 1, 0.0);
    assert!(agents.iter().all(|a| a.id != "claude"));
    assert!(agents.iter().any(|a| a.id == "unassigned"));
}
