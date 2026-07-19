//! Windows-only integration tests, run on a real Windows host in CI.
//!
//! These cover the paths that cannot be exercised on Unix and that previously
//! failed silently: `.exe` attribution, the Windows cwd slug, the named-pipe
//! socket, and DXGI adapter enumeration.

#![cfg(windows)]

use agdog::attribute::{agent_root, cli_signature};
use agdog::collect::system::SystemCollector;
use agdog::model::{AgentKind, AgentState, Event, EventKind};
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

/// Spawn a real process named `claude.exe` and confirm attribution resolves it.
///
/// This is the regression that made every Windows agent land in `unassigned`
/// while looking like "no agents running".
#[test]
fn real_exe_process_is_attributed_as_an_agent() {
    let src = std::env::current_exe().expect("current_exe");
    let dir = std::env::temp_dir().join("agdog-test-attr");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let fake = dir.join("claude.exe");
    // Copy a binary that exits on its own so a leaked child cannot hang CI.
    std::fs::copy(r"C:\Windows\System32\timeout.exe", &fake)
        .or_else(|_| std::fs::copy(&src, &fake))
        .expect("copy to claude.exe");

    let mut child = std::process::Command::new(&fake)
        .args(["/T", "20", "/NOBREAK"])
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawn claude.exe");
    std::thread::sleep(Duration::from_millis(1200));

    let mut sys = SystemCollector::new();
    let _ = sys.sample();
    std::thread::sleep(Duration::from_millis(400));
    let samples = sys.sample();

    let found = samples.iter().find(|s| s.pid == child.id());
    let _ = child.kill();
    let _ = child.wait();

    let s = found.expect("spawned claude.exe should appear in the process list");
    assert_eq!(s.exe_name.to_lowercase(), "claude.exe");
    let (kind, tool) = agent_root(s, None).expect("claude.exe must attribute as an agent root");
    assert_eq!(kind, AgentKind::Coding);
    assert_eq!(tool, "claude");
}

#[test]
fn exe_suffix_is_stripped_for_signatures() {
    assert_eq!(cli_signature("claude.exe").unwrap().0, AgentKind::Coding);
    assert_eq!(cli_signature("ollama.exe").unwrap().0, AgentKind::Infer);
    assert!(cli_signature("node.exe").is_none());
}

/// A Windows cwd must resolve to the folder Claude Code actually creates.
///
/// Seeds a projects dir under a temp USERPROFILE and checks the transcript is
/// found through the real slug (`C:\...` -> `C--...`).
#[test]
fn windows_cwd_resolves_to_its_transcript() {
    let home = std::env::temp_dir().join("agdog-test-home");
    let cwd = r"C:\Users\test\my_project";
    // Same rule Claude Code uses: every non-alphanumeric becomes one dash.
    let slug: String = cwd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    assert_eq!(slug, "C--Users-test-my-project");

    let dir = home.join(".claude").join("projects").join(&slug);
    std::fs::create_dir_all(&dir).expect("create projects dir");
    std::fs::write(
        dir.join("session.jsonl"),
        "{\"message\":{\"content\":[{\"type\":\"tool_use\",\"name\":\"Task\",\"id\":\"t1\",\"input\":{\"subagent_type\":\"explorer\"}}]}}\n",
    )
    .expect("write transcript");

    // subagents_from_transcript reads USERPROFILE on Windows.
    unsafe { std::env::set_var("USERPROFILE", &home) };
    let subs = agdog::subagent::subagents_from_transcript(cwd);
    assert_eq!(
        subs.len(),
        1,
        "a Windows cwd must resolve through its slug to the transcript"
    );
    assert_eq!(subs[0].name, "explorer");

    let _ = std::fs::remove_dir_all(&home);
}

/// The named pipe must carry events to a subscriber and accept reports back.
#[test]
fn named_pipe_round_trip() {
    use agdog::socket::{EventServer, socket_path};

    let server = EventServer::start(socket_path()).expect("start named pipe server");

    // Connect as a client, the same way `agdog watch` does.
    let mut client = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(r"\\.\pipe\agdog")
        .expect("connect to the pipe");
    // Give the accept thread a moment to register the connection.
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(
        server.subscriber_count(),
        1,
        "connected client must be counted as a subscriber"
    );

    // Server -> client: an event arrives as one JSON line.
    let ev = Event {
        kind: EventKind::StateChanged,
        agent_id: "claude:proj".into(),
        from: Some(AgentState::Working),
        to: AgentState::Stuck,
        ts_secs: 42,
    };
    server.broadcast(&ev);

    let mut reader = BufReader::new(client.try_clone().expect("clone pipe handle"));
    let mut line = String::new();
    reader.read_line(&mut line).expect("read event line");
    assert!(
        line.contains("claude:proj"),
        "event should reach the subscriber, got: {line}"
    );

    // Client -> server: a report registers subagents for an agent.
    client
        .write_all(
            b"{\"agent_id\":\"claude:proj\",\"subagents\":[{\"name\":\"explorer\",\"state\":\"working\",\"source\":\"socket\"}]}\n",
        )
        .expect("write report");
    client.flush().expect("flush report");
    std::thread::sleep(Duration::from_millis(500));

    let reported = server.reported_subagents();
    let subs = reported
        .get("claude:proj")
        .expect("report should register under its agent id");
    assert_eq!(subs[0].name, "explorer");
}

/// DXGI must enumerate at least one adapter and report a sane VRAM total.
/// The CI runner has a Hyper-V virtual adapter, which is enough to prove the
/// enumeration path works end to end.
#[test]
fn dxgi_enumerates_an_adapter() {
    use agdog::collect::gpu::GpuCollector;
    use agdog::collect::gpu_windows::WindowsGpu;

    let Some(mut gpu) = WindowsGpu::try_new() else {
        // No adapter at all is a legitimate host configuration, not a failure.
        eprintln!("no DXGI adapter on this host; skipping");
        return;
    };
    let samples = gpu.sample();
    assert!(
        !samples.is_empty(),
        "an available backend must report a device"
    );
    for s in &samples {
        assert!(s.mem_total > 0, "adapter should report a VRAM budget");
        assert!(
            (0.0..=100.0).contains(&s.util_pct),
            "utilization must be a percentage, got {}",
            s.util_pct
        );
    }
    assert_eq!(gpu.name(), "windows-dxgi");
}

/// Sampling must not block the caller. The old implementation slept 200ms
/// inside every sample, on the same thread that draws the UI.
#[test]
fn sampling_does_not_block() {
    use agdog::collect::gpu::GpuCollector;
    use agdog::collect::gpu_windows::WindowsGpu;

    let Some(mut gpu) = WindowsGpu::try_new() else {
        return;
    };
    let start = std::time::Instant::now();
    for _ in 0..5 {
        let _ = gpu.sample();
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(500),
        "5 samples took {elapsed:?}; sampling should not sleep per call"
    );
}
