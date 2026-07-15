use agdog::attribute::{agent_root, cli_signature, is_excluded};
use agdog::model::{AgentKind, ResourceSample};

fn proc(pid: u32, ppid: u32, exe: &str, cmd: &str) -> ResourceSample {
    ResourceSample {
        pid,
        ppid,
        exe_name: exe.to_string(),
        cmd: cmd.to_string(),
        ..Default::default()
    }
}

#[test]
fn claude_cli_is_a_coding_root() {
    let s = proc(100, 1, "claude", "claude --enable-auto-mode");
    let (kind, tool) = agent_root(&s, None).unwrap();
    assert_eq!(kind, AgentKind::Coding);
    assert_eq!(tool, "claude");
}

#[test]
fn ollama_is_an_infer_root() {
    let s = proc(
        100,
        1,
        "ollama",
        "/opt/homebrew/opt/ollama/bin/ollama serve",
    );
    assert_eq!(agent_root(&s, None).unwrap().0, AgentKind::Infer);
}

#[test]
fn gui_app_binary_is_not_a_root() {
    // Claude Desktop lives in a .app bundle and must not be an agent.
    let s = proc(
        100,
        1,
        "Claude",
        "/Applications/Claude.app/Contents/MacOS/Claude",
    );
    assert!(is_excluded(&s));
    assert!(agent_root(&s, None).is_none());
}

#[test]
fn system_cursor_service_is_not_a_root() {
    let s = proc(
        812,
        1,
        "CursorUIViewService",
        "/System/Library/PrivateFrameworks/TextInputUIMacHelper.framework/XPCServices/CursorUIViewService.xpc/Contents/MacOS/CursorUIViewService",
    );
    assert!(is_excluded(&s));
    assert!(agent_root(&s, None).is_none());
}

#[test]
fn claude_config_path_is_not_a_root() {
    // A node process whose path merely contains ".claude" must not match.
    let s = proc(
        200,
        1,
        "node",
        "/Users/x/.hermes/node/bin/node /Users/x/.claude/plugins/mcp-server.cjs",
    );
    assert!(agent_root(&s, None).is_none());
}

#[test]
fn env_tag_forces_a_root() {
    let s = proc(100, 1, "python", "python worker.py");
    let (_, tool) = agent_root(&s, Some("render-batch")).unwrap();
    assert_eq!(tool, "render-batch");
}

#[test]
fn python_comfyui_is_a_render_root() {
    let s = proc(100, 1, "python3.11", "python comfyui/main.py");
    assert_eq!(agent_root(&s, None).unwrap().0, AgentKind::Render);
}

#[test]
fn cli_signature_maps_known_tools_only() {
    assert_eq!(cli_signature("ollama").unwrap().0, AgentKind::Infer);
    assert_eq!(cli_signature("claude").unwrap().0, AgentKind::Coding);
    assert!(cli_signature("bun").is_none());
    assert!(cli_signature("node").is_none());
}
