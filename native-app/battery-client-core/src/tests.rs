use super::*;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const START: u64 = 10_000_000_000;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    directory: PathBuf,
    listener: Option<UnixListener>,
    client: Client,
}
impl Fixture {
    fn new() -> Self {
        assert_eq!(
            unsafe { libc::geteuid() },
            0,
            "actual trusted peer tests require UID0"
        );
        assert_eq!(
            unsafe { libc::getegid() },
            0,
            "actual trusted peer tests require GID0"
        );
        let directory = std::env::temp_dir().join(format!(
            "battery-client-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("snapshot.sock");
        let listener = UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        Self {
            directory,
            listener: Some(listener),
            client: Client::with_path(path),
        }
    }
    fn accept(&mut self, now: u64) -> UnixStream {
        let cutoff = Instant::now() + Duration::from_secs(1);
        loop {
            self.client.tick_at(now);
            match self.listener.as_ref().unwrap().accept() {
                Ok((peer, _)) => return peer,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < cutoff,
                        "client never connected to the actual Unix listener"
                    );
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("accept: {error}"),
            }
        }
    }
    fn reply(&mut self, now: u64, bytes: &[u8]) {
        let mut peer = self.accept(now);
        peer.write_all(bytes).unwrap();
        peer.shutdown(std::net::Shutdown::Write).unwrap();
        self.client.tick_at(now);
        drop(peer);
    }
    fn seed(&mut self) {
        self.reply(START, &wire(START + 2_000_000_000, 57, "discharging"));
        assert_eq!(
            self.client.view_at(START),
            View {
                available: 1,
                capacity_percent: 57,
                status: 3,
                revision: 7
            }
        );
    }
    fn assert_clear(&mut self, now: u64) {
        let view = self.client.view_at(now);
        assert_eq!(view.available, 0);
        assert_eq!(
            view.capacity_percent, -1,
            "failure must clear the previous capacity"
        );
        assert_eq!(view.status, 0, "failure must clear the previous status");
        assert_eq!(view.revision, 0, "failure must clear the previous revision");
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.listener.take();
        std::fs::remove_file(self.directory.join("snapshot.sock")).ok();
        std::fs::remove_dir(&self.directory).unwrap();
    }
}
fn wire(until: u64, capacity: u8, status: &str) -> Vec<u8> {
    format!("{{\"schema\":1,\"revision\":7,\"until_ns\":\"{until}\",\"available\":true,\"capacity_percent\":{capacity},\"status\":\"{status}\"}}\n").into_bytes()
}

#[test]
fn actual_unix_root_peer_valid_then_malformed_clears() {
    let mut f = Fixture::new();
    f.seed();
    f.reply(START + POLL_NS, b"not-json\n");
    f.assert_clear(START + POLL_NS);
}
#[test]
fn actual_unix_absent_and_partial_eof_clear() {
    let mut f = Fixture::new();
    f.seed();
    f.reply(START + POLL_NS, b"{\"schema\":1");
    f.assert_clear(START + POLL_NS);
    f.reply(
        START + 2 * POLL_NS,
        &wire(START + 2_000_000_000, 57, "discharging"),
    );
    assert_eq!(f.client.view_at(START + 2 * POLL_NS).available, 1);
    f.listener.take();
    std::fs::remove_file(f.directory.join("snapshot.sock")).unwrap();
    f.client.tick_at(START + 3 * POLL_NS);
    f.assert_clear(START + 3 * POLL_NS);
}
#[test]
fn view_expires_at_boottime_deadline_without_timer_delivery() {
    let mut f = Fixture::new();
    f.reply(START, &wire(START + 50_000_000, 100, "full"));
    assert_eq!(f.client.view_at(START + 49_999_999).available, 1);
    f.assert_clear(START + 50_000_000);
}
#[test]
fn held_reply_is_nonblocking_and_partial_bytes_never_renew_original_deadline() {
    let mut f = Fixture::new();
    f.seed();
    let request_start = START + POLL_NS;
    let mut peer = f.accept(request_start);
    let complete = wire(START + 2_000_000_000, 84, "charging");
    let split = complete.len() / 2;
    peer.write_all(&complete[..split]).unwrap();
    let wall = Instant::now();
    f.client.tick_at(request_start + REQUEST_NS - 1);
    assert!(
        wall.elapsed() < Duration::from_millis(50),
        "held read blocked the caller"
    );
    peer.write_all(&complete[split..]).unwrap();
    peer.shutdown(std::net::Shutdown::Write).unwrap();
    f.client.tick_at(request_start + REQUEST_NS);
    f.assert_clear(request_start + REQUEST_NS);
}
#[test]
fn unavailable_wire_clears_every_field() {
    let mut f = Fixture::new();
    f.seed();
    let bytes = format!("{{\"schema\":1,\"revision\":8,\"until_ns\":\"{}\",\"available\":false,\"capacity_percent\":null,\"status\":null}}\n", START + 2_000_000_000);
    f.reply(START + POLL_NS, bytes.as_bytes());
    f.assert_clear(START + POLL_NS);
}
#[test]
fn strict_json_shapes_and_two_second_horizon_clear_previous_publication() {
    let valid = String::from_utf8(wire(START + 2_000_000_000, 57, "discharging")).unwrap();
    let bad = [
        valid.replace("\"schema\":1", "\"schema\":1,\"schema\":1"),
        valid.replace("\"schema\":1", "\"schema\":2"),
        valid.replace("\"schema\":1", "\"schema\":1,\"extra\":0"),
        valid.replace("\"capacity_percent\":57", "\"capacity_percent\":101"),
        valid.replace("\"capacity_percent\":57", "\"capacity_percent\":null"),
        valid.replace("\"status\":\"discharging\"", "\"status\":\"invented\""),
        valid.replace("\"available\":true", "\"available\":false"),
        valid.replace("\"revision\":7", "\"revision\":-1"),
        valid.replace("\"until_ns\":\"12000000000\"", "\"until_ns\":12000000000"),
        valid.replace(
            "\"until_ns\":\"12000000000\"",
            "\"until_ns\":\"18446744073709551616\"",
        ),
        String::from_utf8(wire(START + POLL_NS, 57, "discharging")).unwrap(),
        String::from_utf8(wire(START + POLL_NS + 2_000_000_001, 57, "discharging")).unwrap(),
        format!("{valid}{valid}"),
        format!("{valid}garbage"),
        "x".repeat(4097),
    ];
    for bytes in bad {
        let mut f = Fixture::new();
        f.seed();
        f.reply(START + POLL_NS, bytes.as_bytes());
        f.assert_clear(START + POLL_NS);
    }
}
#[test]
fn all_wire_statuses_are_copied_without_transport_requests() {
    for (status, number) in [
        ("unknown", 1),
        ("charging", 2),
        ("discharging", 3),
        ("not_charging", 4),
        ("full", 5),
    ] {
        let mut f = Fixture::new();
        let mut peer = f.accept(START);
        peer.set_nonblocking(true).unwrap();
        let mut probe = [0u8; 1];
        use std::io::Read;
        assert_eq!(
            peer.read(&mut probe).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "read-only client sent a request"
        );
        peer.write_all(&wire(START + 2_000_000_000, 0, status))
            .unwrap();
        peer.shutdown(std::net::Shutdown::Write).unwrap();
        f.client.tick_at(START);
        let view = f.client.view_at(START);
        assert_eq!(
            (view.available, view.capacity_percent, view.status),
            (1, 0, number)
        );
    }
}
