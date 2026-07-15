//! Unix-socket event server and JSON line protocol.
//!
//! Outbound: agdog broadcasts one JSON `Event` line per subscriber. Inbound
//! (Tier 3 subagents): a connected agent may send `Report` lines to tell agdog
//! about its subagents, e.g. `{"agent_id":"claude:dev","subagents":[...]}`.

use crate::model::{Event, SubAgent};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Default socket path: `$XDG_RUNTIME_DIR/agdog.sock`, else the temp dir.
pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join("agdog.sock")
}

/// An inbound subagent report from an agent.
#[derive(Debug, Deserialize)]
struct Report {
    agent_id: String,
    #[serde(default)]
    subagents: Vec<SubAgent>,
}

type Reported = Arc<Mutex<HashMap<String, Vec<SubAgent>>>>;

/// Broadcasts events to subscribers and collects inbound subagent reports.
pub struct EventServer {
    subscribers: Arc<Mutex<Vec<UnixStream>>>,
    reported: Reported,
    path: PathBuf,
}

impl EventServer {
    /// Bind the socket at `path` and accept subscribers on a background thread.
    pub fn start(path: PathBuf) -> std::io::Result<Self> {
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        let subscribers: Arc<Mutex<Vec<UnixStream>>> = Arc::new(Mutex::new(Vec::new()));
        let reported: Reported = Arc::new(Mutex::new(HashMap::new()));
        let subs = Arc::clone(&subscribers);
        let rep = Arc::clone(&reported);
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        let _ = s.set_write_timeout(Some(Duration::from_millis(200)));
                        // Read inbound reports on a clone so writes and reads don't collide.
                        if let Ok(rs) = s.try_clone() {
                            let rep2 = Arc::clone(&rep);
                            thread::spawn(move || read_reports(rs, rep2));
                        }
                        if let Ok(mut guard) = subs.lock() {
                            guard.push(s);
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            subscribers,
            reported,
            path,
        })
    }

    /// Write one JSON line per subscriber. Drops any that error or time out,
    /// so a slow or dead client never blocks the caller.
    pub fn broadcast(&self, ev: &Event) {
        let mut line = match serde_json::to_string(ev) {
            Ok(l) => l,
            Err(_) => return,
        };
        line.push('\n');
        if let Ok(mut guard) = self.subscribers.lock() {
            guard.retain_mut(|s| s.write_all(line.as_bytes()).and_then(|_| s.flush()).is_ok());
        }
    }

    /// Number of currently connected subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Subagents reported by agents over the socket, keyed by agent id.
    pub fn reported_subagents(&self) -> HashMap<String, Vec<SubAgent>> {
        self.reported.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl Drop for EventServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn read_reports(stream: UnixStream, reported: Reported) {
    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(stream);
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(rep) = serde_json::from_str::<Report>(&line)
            && let Ok(mut g) = reported.lock()
        {
            g.insert(rep.agent_id, rep.subagents);
        }
    }
}

/// Connect to the socket and print each event line to stdout (`agdog watch`).
pub fn watch(path: PathBuf) -> std::io::Result<()> {
    use std::io::{BufRead, BufReader};
    let stream = UnixStream::connect(path)?;
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        println!("{}", line?);
    }
    Ok(())
}
