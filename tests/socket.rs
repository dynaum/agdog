use agdog::model::{AgentState, Event, EventKind};
use agdog::socket::EventServer;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::Duration;

#[test]
fn broadcast_reaches_subscriber() {
    let path = std::env::temp_dir().join(format!("agdog-test-{}.sock", std::process::id()));
    let server = EventServer::start(path.clone()).unwrap();

    let client = UnixStream::connect(&path).unwrap();
    let mut reader = BufReader::new(client);

    // Wait for the accept thread to register the subscriber.
    for _ in 0..100 {
        if server.subscriber_count() >= 1 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(server.subscriber_count(), 1);

    let ev = Event {
        kind: EventKind::StateChanged,
        agent_id: "kohya".into(),
        from: Some(AgentState::Working),
        to: AgentState::Stuck,
        ts_secs: 7,
    };
    server.broadcast(&ev);

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let got: Event = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(got, ev);
}
