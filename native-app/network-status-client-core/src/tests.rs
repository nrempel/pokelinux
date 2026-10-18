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
        // SAFETY: these credential getters borrow no storage and never mutate it.
        assert_eq!(unsafe { libc::geteuid() }, 0, "actual root peer required");
        // SAFETY: same credential getter contract.
        assert_eq!(unsafe { libc::getegid() }, 0, "actual root group required");
        let directory = std::env::temp_dir().join(format!(
            "network-status-client-{}-{}",
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
        let until = Instant::now() + Duration::from_secs(1);
        loop {
            self.client.tick_at(now);
            match self.listener.as_ref().unwrap().accept() {
                Ok((peer, _)) => return peer,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < until, "actual client connection missing");
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => panic!("actual listener accept: {e}"),
            }
        }
    }
    fn reply(&mut self, now: u64, bytes: &[u8]) {
        let mut peer = self.accept(now);
        peer.write_all(bytes).unwrap();
        peer.shutdown(std::net::Shutdown::Write).unwrap();
        self.client.tick_at(now);
    }
    fn seed(&mut self) {
        self.reply(START, &wire(START + MAX_FUTURE_NS, 3, 7));
        assert_eq!(
            self.client.view_at(START),
            View {
                available: 1,
                local_interfaces: 3,
                address_records: 7,
                revision: 9,
            }
        );
    }
    fn clear(&mut self, now: u64) {
        assert_eq!(
            self.client.view_at(now),
            View::default(),
            "withdraw both counts and revision"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.listener.take();
        std::fs::remove_file(self.directory.join("snapshot.sock")).ok();
        std::fs::remove_dir(&self.directory).unwrap();
    }
}
fn wire(until: u64, interfaces: u16, addresses: u16) -> Vec<u8> {
    format!("{{\"schema\":1,\"revision\":9,\"until_ns\":\"{until}\",\"available\":true,\"local_interfaces\":{interfaces},\"address_records\":{addresses}}}\n").into_bytes()
}
#[test]
fn real_root_socket_healthy_empty_sends_no_request_bytes() {
    let mut f = Fixture::new();
    let mut peer = f.accept(START);
    peer.set_nonblocking(true).unwrap();
    let mut probe = [0u8; 1];
    assert_eq!(
        peer.read(&mut probe).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    peer.write_all(&wire(START + MAX_FUTURE_NS, 0, 0)).unwrap();
    peer.shutdown(std::net::Shutdown::Write).unwrap();
    f.client.tick_at(START);
    assert_eq!(
        f.client.view_at(START),
        View {
            available: 1,
            local_interfaces: 0,
            address_records: 0,
            revision: 9,
        }
    );
    assert_eq!(
        peer.read(&mut probe).unwrap(),
        0,
        "client wrote request bytes before closing"
    );
}
#[test]
fn real_socket_malformed_and_partial_eof_clear_previous_counts() {
    for bytes in [b"not-json\n".as_slice(), b"{\"schema\":1".as_slice()] {
        let mut f = Fixture::new();
        f.seed();
        f.reply(START + POLL_NS, bytes);
        f.clear(START + POLL_NS);
    }
}
#[test]
fn complete_line_without_eof_is_not_a_new_snapshot_and_cannot_renew_deadline() {
    let mut f = Fixture::new();
    f.seed();
    let begun = START + POLL_NS;
    let mut peer = f.accept(begun);
    peer.write_all(&wire(START + MAX_FUTURE_NS, 2, 4)).unwrap();
    let wall = Instant::now();
    f.client.tick_at(begun + REQUEST_NS - 1);
    assert!(
        wall.elapsed() < Duration::from_millis(50),
        "held EOF blocked tick"
    );
    assert_eq!(f.client.view_at(begun + REQUEST_NS - 1).local_interfaces, 3);
    peer.shutdown(std::net::Shutdown::Write).unwrap();
    f.client.tick_at(begun + REQUEST_NS);
    f.clear(begun + REQUEST_NS);
}
#[test]
fn getter_expires_at_boottime_boundary_without_a_tick() {
    let mut f = Fixture::new();
    f.reply(START, &wire(START + 50_000_000, 1, 16));
    assert_eq!(f.client.view_at(START + 49_999_999).address_records, 16);
    f.clear(START + 50_000_000);
}
#[test]
fn service_loss_clears_and_fresh_read_only_connection_recovers() {
    let mut f = Fixture::new();
    f.seed();
    f.listener.take();
    let path = f.directory.join("snapshot.sock");
    std::fs::remove_file(&path).unwrap();
    f.client.tick_at(START + POLL_NS);
    f.clear(START + POLL_NS);
    let listener = UnixListener::bind(path).unwrap();
    listener.set_nonblocking(true).unwrap();
    f.listener = Some(listener);
    f.reply(START + 2 * POLL_NS, &wire(START + MAX_FUTURE_NS, 0, 0));
    assert_eq!(f.client.view_at(START + 2 * POLL_NS).local_interfaces, 0);
}
#[test]
fn strict_six_fields_count_bounds_and_original_freshness_horizon() {
    let good = String::from_utf8(wire(START + MAX_FUTURE_NS, 3, 7)).unwrap();
    let absent = "{\"schema\":1,\"revision\":10,\"until_ns\":\"0\",\"available\":false,\"local_interfaces\":null,\"address_records\":null}\n";
    let bad = [
        good.replace("\"schema\":1", "\"schema\":1,\"schema\":1"),
        good.replace("\"schema\":1", "\"schema\":2"),
        good.replace("\"schema\":1", "\"schema\":1,\"extra\":0"),
        good.replace("\"local_interfaces\":3", "\"local_interfaces\":65"),
        good.replace("\"local_interfaces\":3", "\"local_interfaces\":-1"),
        good.replace("\"local_interfaces\":3", "\"local_interfaces\":0"),
        good.replace("\"address_records\":7", "\"address_records\":49"),
        good.replace("\"address_records\":7", "\"address_records\":null"),
        good.replace("\"revision\":9", "\"revision\":0"),
        good.replace(
            "\"until_ns\":\"12000000000\"",
            "\"until_ns\":\"012000000000\"",
        ),
        good.replace("\"until_ns\":\"12000000000\"", "\"until_ns\":12000000000"),
        good.replace(
            "\"until_ns\":\"12000000000\"",
            "\"until_ns\":\"18446744073709551616\"",
        ),
        good.replace("\"available\":true", "\"available\":false"),
        absent.replace(",\"local_interfaces\":null", ""),
        absent.replace(",\"address_records\":null", ""),
        absent.replace("\"local_interfaces\":null", "\"local_interfaces\":0"),
        absent.replace("\"until_ns\":\"0\"", "\"until_ns\":\"12000000000\""),
        String::from_utf8(wire(START + POLL_NS, 3, 7)).unwrap(),
        String::from_utf8(wire(START + POLL_NS + MAX_FUTURE_NS + 1, 3, 7)).unwrap(),
        good.trim_end().to_owned(),
        format!("{good}{good}"),
        format!("{good}tail"),
        "x".repeat(MAX_WIRE + 1),
    ];
    for bytes in bad {
        let mut f = Fixture::new();
        f.seed();
        f.reply(START + POLL_NS, bytes.as_bytes());
        f.clear(START + POLL_NS);
    }
    let mut f = Fixture::new();
    f.seed();
    f.reply(START + POLL_NS, absent.as_bytes());
    f.clear(START + POLL_NS);
}
