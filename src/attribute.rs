//! Process-to-agent attribution engine.

use crate::model::{AgentKind, ResourceSample};
use std::collections::HashMap;

/// The result of attributing a process to an agent.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribution {
    pub agent_id: String,
    pub kind: AgentKind,
    pub confidence: f32,
}

fn contains_any(cmd: &str, needles: &[&str]) -> Option<String> {
    let c = cmd.to_lowercase();
    needles
        .iter()
        .find(|n| c.contains(*n))
        .map(|s| (*s).to_string())
}

const RENDER: &[&str] = &["comfyui", "sd-webui", "a1111"];
const TRAIN: &[&str] = &["kohya", "accelerate", "train"];
const INFER: &[&str] = &["ollama", "vllm", "llama", "whisper"];
const CODING: &[&str] = &["claude", "codex", "cursor", "aider", "goose"];

/// Infer the kind of work from a command line.
pub fn kind_from_cmd(cmd: &str) -> AgentKind {
    if contains_any(cmd, RENDER).is_some() {
        AgentKind::Render
    } else if contains_any(cmd, TRAIN).is_some() {
        AgentKind::Train
    } else if contains_any(cmd, INFER).is_some() {
        AgentKind::Infer
    } else if contains_any(cmd, CODING).is_some() {
        AgentKind::Coding
    } else {
        AgentKind::Unknown
    }
}

/// Derive a stable short agent id from a command line and its kind.
fn id_from_cmd(cmd: &str, kind: AgentKind) -> String {
    let needles = match kind {
        AgentKind::Render => RENDER,
        AgentKind::Train => TRAIN,
        AgentKind::Infer => INFER,
        AgentKind::Coding => CODING,
        AgentKind::Unknown => return "unassigned".into(),
    };
    contains_any(cmd, needles).unwrap_or_else(|| "unassigned".into())
}

/// Attribute a single process to an agent using layered heuristics:
/// explicit env tag (1.0) > cmdline signature (0.8) > parent-tree
/// inheritance (0.6) > unassigned (0.0). Port-map attribution is a later
/// refinement and is not yet wired into this signature.
pub fn attribute(
    sample: &ResourceSample,
    by_pid: &HashMap<u32, ResourceSample>,
    env_tag: Option<&str>,
) -> Attribution {
    // 1. Explicit env tag wins.
    if let Some(tag) = env_tag
        && !tag.is_empty()
    {
        return Attribution {
            agent_id: tag.to_string(),
            kind: kind_from_cmd(&sample.cmd),
            confidence: 1.0,
        };
    }

    // 2. Cmdline signature.
    let kind = kind_from_cmd(&sample.cmd);
    if kind != AgentKind::Unknown {
        return Attribution {
            agent_id: id_from_cmd(&sample.cmd, kind),
            kind,
            confidence: 0.8,
        };
    }

    // 3. Parent-tree inheritance: walk up to an attributable ancestor.
    let mut cur = sample.ppid;
    let mut hops = 0;
    while cur != 0 && hops < 32 {
        match by_pid.get(&cur) {
            Some(parent) => {
                let pk = kind_from_cmd(&parent.cmd);
                if pk != AgentKind::Unknown {
                    return Attribution {
                        agent_id: id_from_cmd(&parent.cmd, pk),
                        kind: pk,
                        confidence: 0.6,
                    };
                }
                cur = parent.ppid;
                hops += 1;
            }
            None => break,
        }
    }

    // 4. Nothing matched.
    Attribution {
        agent_id: "unassigned".into(),
        kind: AgentKind::Unknown,
        confidence: 0.0,
    }
}
