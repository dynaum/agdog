//! Tier 2 subagents: read Claude Code Task sidechains from session transcripts.
//!
//! Claude Code writes a JSONL transcript per session under
//! `~/.claude/projects/<encoded-cwd>/<uuid>.jsonl`. Task subagents run in-process
//! (invisible to `ps`) but appear in the transcript as `Task` tool_use entries;
//! a matching `tool_result` marks completion. We surface the ones still running.

use crate::model::{AgentState, SubAgent, SubSource};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Longest slug Claude Code writes before truncating and appending a hash.
const MAX_SLUG: usize = 200;

/// Encode a working directory the way Claude Code names its project folder.
///
/// The rule is a whitelist, not a separator swap: every character outside
/// `[a-zA-Z0-9]` becomes one `-`. Verified against the 2.1.x bundle, which
/// does `path.replace(/[^a-zA-Z0-9]/g, "-")`.
///
/// This matters on two fronts. It is what makes Windows work at all, since
/// `C:\Users\me\proj` has to become `C--Users-me-proj`. It also fixes Unix
/// paths containing `_`, spaces, or any other non-alphanumeric, which the
/// previous `/`-and-`.` rule left untouched, so `~/dev/kohya_ss` never
/// resolved to its transcript.
///
/// The result is always ASCII, which lets callers slice it by byte safely.
fn slug(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The Claude projects dir for a working directory, if it exists.
fn transcript_dir(cwd: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let projects = PathBuf::from(home).join(".claude").join("projects");
    let slug = slug(cwd);

    let exact = projects.join(&slug);
    if exact.is_dir() {
        return Some(exact);
    }

    // Over the limit Claude truncates to 200 chars and appends `-<hash>`, so
    // match on the prefix rather than recomputing its hash. Slugs are ASCII,
    // so slicing on a byte index cannot split a character.
    if slug.len() > MAX_SLUG {
        let prefix = format!("{}-", &slug[..MAX_SLUG]);
        let found = fs::read_dir(&projects)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().starts_with(prefix.as_str()))?;
        let dir = found.path();
        return dir.is_dir().then_some(dir);
    }
    None
}

/// The most recently modified `.jsonl` transcript in a projects dir.
fn newest_transcript(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .max_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
}

/// In-flight Task subagents for the session whose working directory is `cwd`.
pub fn subagents_from_transcript(cwd: &str) -> Vec<SubAgent> {
    let Some(dir) = transcript_dir(cwd) else {
        return Vec::new();
    };
    let Some(file) = newest_transcript(&dir) else {
        return Vec::new();
    };
    match fs::read_to_string(&file) {
        Ok(text) => parse_inflight(&text),
        Err(_) => Vec::new(),
    }
}

/// Parse transcript JSONL for Task sidechains started but not yet returned.
/// Bounded to the tail so large transcripts stay cheap.
pub fn parse_inflight(text: &str) -> Vec<SubAgent> {
    let tail: Vec<&str> = {
        let mut v: Vec<&str> = text.lines().rev().take(4000).collect();
        v.reverse();
        v
    };
    let mut started: HashMap<String, String> = HashMap::new();
    let mut finished: HashSet<String> = HashSet::new();

    for line in tail {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(items) = v
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
        else {
            continue;
        };
        for it in items {
            match it.get("type").and_then(|t| t.as_str()) {
                Some("tool_use") if it.get("name").and_then(|n| n.as_str()) == Some("Task") => {
                    if let Some(id) = it.get("id").and_then(|x| x.as_str()) {
                        let sub = it
                            .get("input")
                            .and_then(|i| i.get("subagent_type"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("task");
                        started.insert(id.to_string(), sub.to_string());
                    }
                }
                Some("tool_result") => {
                    if let Some(id) = it.get("tool_use_id").and_then(|x| x.as_str()) {
                        finished.insert(id.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    let mut out: Vec<SubAgent> = started
        .into_iter()
        .filter(|(id, _)| !finished.contains(id))
        .map(|(_, name)| SubAgent {
            name,
            state: AgentState::Working,
            source: SubSource::Transcript,
            cpu_pct: 0.0,
            mem_bytes: 0,
            task: String::new(),
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflight_task_without_result_is_reported() {
        let text = r#"
{"message":{"content":[{"type":"tool_use","name":"Task","id":"t1","input":{"subagent_type":"explorer"}}]}}
{"message":{"content":[{"type":"text","text":"working"}]}}
"#;
        let subs = parse_inflight(text);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].name, "explorer");
        assert_eq!(subs[0].source, SubSource::Transcript);
    }

    #[test]
    fn slug_matches_claude_code_encoding() {
        // Unix, the case that already worked.
        assert_eq!(slug("/Users/me/dev/proj"), "-Users-me-dev-proj");
        // A dot directory yields a double dash. Confirmed against a real
        // local folder: `-Users-dynaum--claude-mem-observer-sessions`.
        assert_eq!(slug("/Users/me/.claude-mem/x"), "-Users-me--claude-mem-x");
        // Underscores map to `-` too. The old `/`-and-`.` rule got this
        // wrong, so a path like ~/dev/kohya_ss never found its transcript.
        assert_eq!(slug("/Users/me/dev/kohya_ss"), "-Users-me-dev-kohya-ss");
        assert_eq!(slug("/Users/me/my project"), "-Users-me-my-project");
        // Windows: drive colon and backslashes, and no leading dash.
        assert_eq!(slug(r"C:\Users\me\proj"), "C--Users-me-proj");
        assert_eq!(slug(r"E:\MatLab_HomeWork"), "E--MatLab-HomeWork");
        // UNC paths start with two dashes and keep a trailing one.
        assert_eq!(slug(r"\\diskstation\docker\"), "--diskstation-docker-");
        // Case is preserved.
        assert_eq!(slug("/Users/me/MoneyPrinter"), "-Users-me-MoneyPrinter");
        // Non-ASCII becomes one dash per character. This exact pair (cwd and
        // folder name) was observed on a real Windows 11 host.
        assert_eq!(slug(r"D:\桌面\资料\matlab综合"), "D--------matlab--");
        // Always ASCII, so byte slicing in `transcript_dir` is safe.
        assert!(slug(r"D:\桌面\资料").is_ascii());
    }

    #[test]
    fn completed_task_is_not_reported() {
        let text = r#"
{"message":{"content":[{"type":"tool_use","name":"Task","id":"t1","input":{"subagent_type":"explorer"}}]}}
{"message":{"content":[{"type":"tool_result","tool_use_id":"t1"}]}}
"#;
        assert!(parse_inflight(text).is_empty());
    }
}
