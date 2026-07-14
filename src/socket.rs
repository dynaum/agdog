//! Unix-socket event server and JSON line protocol.

use crate::model::Event;
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

/// Broadcasts events to any connected subscribers over a Unix socket.
pub struct EventServer {
    subscribers: Arc<Mutex<Vec<UnixStream>>>,
    path: PathBuf,
}

impl EventServer {
    /// Bind the socket at `path` and accept subscribers on a background thread.
    pub fn start(path: PathBuf) -> std::io::Result<Self> {
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        let subscribers: Arc<Mutex<Vec<UnixStream>>> = Arc::new(Mutex::new(Vec::new()));
        let subs = Arc::clone(&subscribers);
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        let _ = s.set_write_timeout(Some(Duration::from_millis(200)));
                        if let Ok(mut guard) = subs.lock() {
                            guard.push(s);
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self { subscribers, path })
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
}

impl Drop for EventServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
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
