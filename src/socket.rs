//! Event socket and JSON line protocol.
//!
//! agdog broadcasts one JSON `Event` line per subscriber, and connected agents
//! may send `Report` lines back to declare their subagents (Tier 3).
//!
//! The transport is whatever the platform's local IPC primitive is: a
//! Unix-domain socket at `$XDG_RUNTIME_DIR/agdog.sock`, or a named pipe at
//! `\\.\pipe\agdog` on Windows. Both are access-controlled by the OS and
//! neither opens a network port. The protocol is identical, so a client only
//! has to know which name to connect to.

pub use imp::*;

#[cfg(unix)]
mod imp {
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

        /// Write one JSON line per subscriber, dropping any that error or time out.
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
}

#[cfg(windows)]
mod imp {
    use crate::model::{Event, SubAgent};
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Write;
    use std::os::windows::io::FromRawHandle;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows::Win32::Storage::FileSystem::{FILE_FLAGS_AND_ATTRIBUTES, PIPE_ACCESS_DUPLEX};
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
        PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };
    use windows::core::PCWSTR;

    /// The pipe agdog listens on. Windows has no filesystem socket, so this is
    /// a name in the pipe namespace rather than a path on disk.
    ///
    /// A named pipe's default security descriptor grants access to the creating
    /// user and to administrators, and denies anonymous remote clients. That
    /// matches the intent of the Unix socket without opening a TCP port.
    pub const PIPE_NAME: &str = r"\\.\pipe\agdog";

    /// Where clients connect. Kept as a `PathBuf` so the API matches Unix.
    pub fn socket_path() -> PathBuf {
        PathBuf::from(PIPE_NAME)
    }

    /// An inbound subagent report from an agent.
    #[derive(Debug, Deserialize)]
    struct Report {
        agent_id: String,
        #[serde(default)]
        subagents: Vec<SubAgent>,
    }

    type Reported = Arc<Mutex<HashMap<String, Vec<SubAgent>>>>;

    /// Encode a Rust string as a NUL-terminated UTF-16 buffer for the Win32 API.
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// A pipe handle being handed to the accept thread.
    ///
    /// `HANDLE` wraps a raw pointer so it is not `Send` by default. Ownership
    /// moves to exactly one thread and is never shared, which makes this safe.
    struct SendHandle(HANDLE);
    unsafe impl Send for SendHandle {}

    /// Broadcasts events to subscribers and collects inbound subagent reports.
    pub struct EventServer {
        subscribers: Arc<Mutex<Vec<File>>>,
        reported: Reported,
    }

    impl EventServer {
        /// Listen on the named pipe, accepting subscribers on a background thread.
        ///
        /// Windows creates one pipe *instance* per client, so the accept loop
        /// makes a fresh instance, waits for a connection on it, hands it off,
        /// then repeats. `path` is accepted for symmetry with Unix; the pipe
        /// name is fixed.
        pub fn start(_path: PathBuf) -> std::io::Result<Self> {
            let subscribers: Arc<Mutex<Vec<File>>> = Arc::new(Mutex::new(Vec::new()));
            let reported: Reported = Arc::new(Mutex::new(HashMap::new()));

            // Create the first instance up front so a client connecting right
            // after `start` returns does not race the accept thread.
            let first = create_instance()?;

            let subs = Arc::clone(&subscribers);
            let rep = Arc::clone(&reported);
            thread::spawn(move || {
                let mut pending = first;
                loop {
                    // Blocks until a client connects to this instance.
                    if unsafe { ConnectNamedPipe(pending.0, None) }.is_err() {
                        // ERROR_PIPE_CONNECTED (535) means a client attached
                        // between creation and this call, which is a success
                        // for our purpose. Anything else ends the loop.
                        let code = unsafe { windows::Win32::Foundation::GetLastError() };
                        if code.0 != 535 {
                            unsafe {
                                let _ = CloseHandle(pending.0);
                            }
                            break;
                        }
                    }

                    // Taking the handle as a File gives normal Read/Write.
                    let file = unsafe { File::from_raw_handle(pending.0.0) };
                    if let Ok(read_half) = file.try_clone() {
                        let rep2 = Arc::clone(&rep);
                        thread::spawn(move || read_reports(read_half, rep2));
                    }
                    if let Ok(mut guard) = subs.lock() {
                        guard.push(file);
                    }

                    // Every further client needs its own instance.
                    match create_instance() {
                        Ok(h) => pending = h,
                        Err(_) => break,
                    }
                }
            });

            Ok(Self {
                subscribers,
                reported,
            })
        }

        /// Write one JSON line per subscriber, dropping any that error.
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

    /// Create one instance of the named pipe, ready for a client to connect.
    fn create_instance() -> std::io::Result<SendHandle> {
        let name = wide(PIPE_NAME);
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_DUPLEX.0),
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                64 * 1024,
                64 * 1024,
                0,
                None,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        Ok(SendHandle(handle))
    }

    /// Drain inbound `Report` lines without ever blocking in `ReadFile`.
    ///
    /// Windows serializes I/O on a synchronous file object, so a blocking read
    /// parked on this pipe also stalls `broadcast`'s write to the very same
    /// pipe, and neither side makes progress. Peeking first means the read is
    /// only issued when bytes are already buffered, so it returns at once and
    /// leaves the write path free.
    fn read_reports(stream: File, reported: Reported) {
        use std::io::Read;
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::System::Pipes::PeekNamedPipe;

        let handle = HANDLE(stream.as_raw_handle());
        let mut stream = stream;
        let mut pending = String::new();
        let mut buf = [0u8; 8192];

        loop {
            let mut avail: u32 = 0;
            let ok = unsafe { PeekNamedPipe(handle, None, 0, None, Some(&mut avail), None) };
            if ok.is_err() {
                return; // client disconnected
            }
            if avail == 0 {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let want = (avail as usize).min(buf.len());
            match stream.read(&mut buf[..want]) {
                Ok(0) => return,
                Ok(n) => pending.push_str(&String::from_utf8_lossy(&buf[..n])),
                Err(_) => return,
            }

            // Consume whole lines, keeping any partial tail for the next read.
            while let Some(nl) = pending.find('\n') {
                let line: String = pending.drain(..=nl).collect();
                if let Ok(rep) = serde_json::from_str::<Report>(line.trim())
                    && let Ok(mut g) = reported.lock()
                {
                    g.insert(rep.agent_id, rep.subagents);
                }
            }
        }
    }

    /// Connect to the pipe and print each event line to stdout (`agdog watch`).
    pub fn watch(_path: PathBuf) -> std::io::Result<()> {
        use std::io::{BufRead, BufReader};
        // A named pipe client is just a file open on the pipe name.
        let stream = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(PIPE_NAME)?;
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            println!("{}", line?);
        }
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
mod imp {
    use crate::model::{Event, SubAgent};
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Placeholder socket path (the socket is not active on this platform).
    pub fn socket_path() -> PathBuf {
        std::env::temp_dir().join("agdog.sock")
    }

    /// No-op event server: no socket transport on this platform.
    pub struct EventServer;

    impl EventServer {
        pub fn start(_path: PathBuf) -> std::io::Result<Self> {
            Ok(EventServer)
        }
        pub fn broadcast(&self, _ev: &Event) {}
        pub fn subscriber_count(&self) -> usize {
            0
        }
        pub fn reported_subagents(&self) -> HashMap<String, Vec<SubAgent>> {
            HashMap::new()
        }
    }

    pub fn watch(_path: PathBuf) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "the agdog event socket is not available on this platform",
        ))
    }
}
