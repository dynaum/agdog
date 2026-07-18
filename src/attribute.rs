//! Process-to-agent attribution.
//!
//! An *agent root* is a process that is itself an agent CLI (like `claude` or
//! `ollama`). Matching is on the program (executable) name, not on substrings of
//! the full path, so `~/.claude/...` plugin files and a `CursorUIViewService`
//! system helper no longer masquerade as agents. GUI-app helper processes and
//! system services are excluded from being roots. Non-root processes are
//! attributed to a root by walking their parent tree (done in `app::build_agents`).

use crate::model::{AgentKind, ResourceSample};

/// Known agent CLIs, matched by exact program (executable) name.
///
/// A trailing `.exe` is stripped first: on Windows the executable basename is
/// `claude.exe`, and matching it raw sends every agent to `unassigned`.
pub fn cli_signature(exe: &str) -> Option<(AgentKind, &'static str)> {
    let exe = exe.strip_suffix(".exe").unwrap_or(exe);
    match exe {
        "claude" => Some((AgentKind::Coding, "claude")),
        "aider" => Some((AgentKind::Coding, "aider")),
        "codex" => Some((AgentKind::Coding, "codex")),
        "goose" => Some((AgentKind::Coding, "goose")),
        "ollama" => Some((AgentKind::Infer, "ollama")),
        "vllm" => Some((AgentKind::Infer, "vllm")),
        "llama-server" | "llama-cli" => Some((AgentKind::Infer, "llama.cpp")),
        _ => None,
    }
}

/// True for processes that must never be an agent root: GUI-app helper
/// processes (Electron / `.app` bundles) and macOS system services.
pub fn is_excluded(sample: &ResourceSample) -> bool {
    let c = &sample.cmd;
    c.contains(".app/Contents/") || c.contains("/System/Library/")
}

/// If this process is the root of an agent, return its `(kind, tool)`.
///
/// An explicit `AGENT_ID` env tag forces attribution regardless of program name.
pub fn agent_root(sample: &ResourceSample, env_tag: Option<&str>) -> Option<(AgentKind, String)> {
    if let Some(tag) = env_tag
        && !tag.is_empty()
    {
        let kind = cli_signature(&sample.exe_name.to_lowercase())
            .map(|(k, _)| k)
            .unwrap_or(AgentKind::Unknown);
        return Some((kind, tag.to_string()));
    }

    if is_excluded(sample) {
        return None;
    }

    let exe = sample.exe_name.to_lowercase();
    if let Some((kind, tool)) = cli_signature(&exe) {
        return Some((kind, tool.to_string()));
    }

    // Python-launched frameworks: match on args, but only for python-ish exes so
    // an arbitrary process mentioning "comfyui" in a path is not caught.
    if exe.starts_with("python") || exe == "accelerate" || exe == "torchrun" {
        let cmd = sample.cmd.to_lowercase();
        if cmd.contains("comfyui") || cmd.contains("sd-webui") || cmd.contains("a1111") {
            return Some((AgentKind::Render, "comfyui".to_string()));
        }
        if cmd.contains("kohya") || cmd.contains("train_network") {
            return Some((AgentKind::Train, "kohya".to_string()));
        }
    }

    None
}
