#![cfg(target_os = "linux")]
use omarchy_phone_core::tls::{ExchangeError, TlsClient};
use rustls::{
    RootCertStore, ServerConfig, ServerConnection, StreamOwned,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::Arc,
    time::{Duration, Instant},
};
fn identity(name: &str) -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
    let generated = rcgen::generate_simple_self_signed(vec![name.into()]).unwrap();
    (
        generated.cert.der().clone(),
        PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der()).into(),
    )
}
struct Fixture {
    client: TlsClient,
    server: Arc<ServerConfig>,
}
impl Fixture {
    fn new(bad_pin: bool, bad_name: bool, bad_root: bool) -> Self {
        Self::with_client(bad_pin, bad_name, bad_root, false)
    }
    fn with_client(bad_pin: bool, bad_name: bool, bad_root: bool, untrusted_client: bool) -> Self {
        let (server_cert, server_key) = identity("android.test");
        let (client_cert, client_key) = identity("client.test");
        let mut client_roots = RootCertStore::empty();
        client_roots
            .add(if untrusted_client {
                identity("different-client.test").0
            } else {
                client_cert.clone()
            })
            .unwrap();
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(client_roots),
            provider.clone(),
        )
        .build()
        .unwrap();
        let server = ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_client_cert_verifier(verifier)
            .with_single_cert(vec![server_cert.clone()], server_key)
            .unwrap();
        let pin: [u8; 32] = if bad_pin {
            [0; 32]
        } else {
            Sha256::digest(server_cert.as_ref()).into()
        };
        let root = if bad_root {
            identity("rogue.test").0
        } else {
            server_cert
        };
        let client = TlsClient::new(
            vec![root],
            vec![client_cert],
            client_key,
            if bad_name {
                "other.test"
            } else {
                "android.test"
            }
            .into(),
            pin,
        )
        .unwrap();
        Self {
            client,
            server: Arc::new(server),
        }
    }
    fn serve(
        &self,
        response: Vec<u8>,
        delay: Duration,
    ) -> (SocketAddr, std::thread::JoinHandle<usize>) {
        self.serve_request(response, delay, b"{\"op\":\"status\"}".to_vec())
    }
    fn serve_request(
        &self,
        response: Vec<u8>,
        delay: Duration,
        expected: Vec<u8>,
    ) -> (SocketAddr, std::thread::JoinHandle<usize>) {
        self.serve_after(response, delay, expected, Duration::ZERO)
    }
    fn serve_after(
        &self,
        response: Vec<u8>,
        delay: Duration,
        expected: Vec<u8>,
        initial_delay: Duration,
    ) -> (SocketAddr, std::thread::JoinHandle<usize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let config = self.server.clone();
        let worker = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            let raw = loop {
                match listener.accept() {
                    Ok((raw, _)) => break raw,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() >= deadline {
                            return 0;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => panic!("accept failed: {e}"),
                }
            };
            raw.set_nonblocking(false).unwrap();
            raw.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            raw.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut stream = StreamOwned::new(ServerConnection::new(config).unwrap(), raw);
            let mut header = [0; 4];
            if stream.read_exact(&mut header).is_err() {
                return 0;
            }
            let size = u32::from_be_bytes(header) as usize;
            assert!(size <= 65536);
            let mut body = vec![0; size];
            stream.read_exact(&mut body).unwrap();
            assert_eq!(body, expected);
            std::thread::sleep(initial_delay);
            if delay.is_zero() {
                let _ = stream.write_all(&response).and_then(|_| stream.flush());
                return size;
            }
            for byte in response {
                std::thread::sleep(delay);
                if stream
                    .write_all(&[byte])
                    .and_then(|_| stream.flush())
                    .is_err()
                {
                    break;
                }
            }
            size
        });
        (address, worker)
    }
}
fn frame(bytes: &[u8]) -> Vec<u8> {
    let mut out = (bytes.len() as u32).to_be_bytes().to_vec();
    out.extend(bytes);
    out
}
#[test]
fn authenticated_exchange_is_framed_and_reads_fragmented_reply() {
    let f = Fixture::new(false, false, false);
    let (address, worker) = f.serve(frame(b"{\"ok\":true}"), Duration::from_millis(1));
    let result = f
        .client
        .exchange(address, b"{\"op\":\"status\"}", Duration::from_secs(2));
    assert!(worker.join().unwrap() > 0);
    assert_eq!(result.unwrap(), b"{\"ok\":true}");
}
#[test]
fn request_and_response_limits_are_independent() {
    let f = Fixture::new(false, false, false);
    let mut request = b"{\"op\":\"status\"}".to_vec();
    request.resize(4096, b' ');
    let mut response = b"{}".to_vec();
    response.resize(65_536, b' ');
    let (address, worker) = f.serve_request(frame(&response), Duration::ZERO, request.clone());
    let actual = f.client.exchange(address, &request, Duration::from_secs(2));
    assert_eq!(worker.join().unwrap(), 4096);
    assert_eq!(actual.unwrap(), response);
}
#[test]
fn invalid_pin_name_or_root_never_sends_application_bytes() {
    for options in [
        (true, false, false),
        (false, true, false),
        (false, false, true),
    ] {
        let f = Fixture::new(options.0, options.1, options.2);
        let (address, worker) = f.serve(frame(b"{}"), Duration::ZERO);
        assert!(matches!(
            f.client
                .exchange(address, b"{\"op\":\"status\"}", Duration::from_secs(2)),
            Err(ExchangeError::NotSent(_))
        ));
        assert_eq!(worker.join().unwrap(), 0);
    }
}
#[test]
fn incomplete_or_oversize_reply_is_unknown_without_retry() {
    for body in [
        vec![0, 1, 0, 1],
        vec![0, 0, 0, 0],
        vec![0, 0, 0, 5, b'{', b'}'],
    ] {
        let f = Fixture::new(false, false, false);
        let (address, worker) = f.serve(body, Duration::ZERO);
        assert!(matches!(
            f.client
                .exchange(address, b"{\"op\":\"status\"}", Duration::from_secs(2)),
            Err(ExchangeError::Unknown(_))
        ));
        assert!(worker.join().unwrap() > 0);
    }
}
#[test]
fn drip_fed_reply_cannot_extend_absolute_budget() {
    let f = Fixture::new(false, false, false);
    let (address, worker) = f.serve(frame(&[b' '; 100]), Duration::from_millis(10));
    let start = Instant::now();
    assert!(matches!(
        f.client
            .exchange(address, b"{\"op\":\"status\"}", Duration::from_millis(150)),
        Err(ExchangeError::Unknown(_))
    ));
    assert!(start.elapsed() < Duration::from_millis(700));
    worker.join().unwrap();
}

#[test]
fn android_peer_requires_the_enrolled_client_identity() {
    let f = Fixture::with_client(false, false, false, true);
    let (address, worker) = f.serve(frame(b"{}"), Duration::ZERO);
    assert!(
        f.client
            .exchange(address, b"{\"op\":\"status\"}", Duration::from_secs(2))
            .is_err()
    );
    assert_eq!(worker.join().unwrap(), 0);
}

#[test]
fn stalled_tls_handshake_has_a_total_deadline() {
    let f = Fixture::new(false, false, false);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        std::thread::sleep(Duration::from_millis(250));
        drop(stream);
    });
    let start = Instant::now();
    assert!(matches!(
        f.client.exchange(address, b"{}", Duration::from_millis(80)),
        Err(ExchangeError::NotSent(_))
    ));
    assert!(start.elapsed() >= Duration::from_millis(60));
    assert!(start.elapsed() < Duration::from_millis(240));
    worker.join().unwrap();
}

#[test]
fn invalid_requests_and_zero_budget_do_not_connect() {
    let f = Fixture::new(false, false, false);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    // Android accepts at most 4096 request bytes. Reject before opening a
    // connection, rather than reporting an ambiguous outcome after a write.
    for body in [vec![], vec![0; 4097], vec![0; 65_537]] {
        assert!(matches!(
            f.client.exchange(address, &body, Duration::from_secs(2)),
            Err(ExchangeError::NotSent(_))
        ));
    }
    assert!(matches!(
        f.client.exchange(address, b"{}", Duration::ZERO),
        Err(ExchangeError::NotSent(_))
    ));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn long_watch_has_separate_budget_and_does_not_extend_command_deadline() {
    use omarchy_phone_core::{
        Identity,
        watch::{SourceState, WatchQuery},
    };
    let f = Fixture::new(false, false, false);
    let id: Identity = serde_json::from_str("\"11111111111111111111111111111111\"").unwrap();
    let query = WatchQuery::new(id, None, 25_000).unwrap();
    let response = br#"{"v":1,"id":"11111111111111111111111111111111","generation":"22222222222222222222222222222222","revision":1,"source":"android.telecom.InCallService","source_state":"detached","ringing":[]}"#;
    let (watch_address, watcher) = f.serve_after(
        frame(response),
        Duration::ZERO,
        query.encode(),
        Duration::from_secs(6),
    );
    let (command_address, commander) = f.serve_after(
        frame(b"{}"),
        Duration::ZERO,
        b"{}".to_vec(),
        Duration::from_secs(6),
    );
    let short_query = WatchQuery::new(query.id().clone(), None, 1000).unwrap();
    let (short_address, short_worker) = f.serve_after(
        frame(response),
        Duration::ZERO,
        short_query.encode(),
        Duration::from_secs(6),
    );
    std::thread::scope(|scope| {
        let short = scope.spawn(|| {
            let start = Instant::now();
            assert!(
                f.client
                    .watch(
                        short_address,
                        &short_query,
                        &omarchy_phone_core::tls::WatchCancel::default()
                    )
                    .is_err()
            );
            assert!(start.elapsed() < Duration::from_secs(6));
        });
        let command = scope.spawn(|| {
            let start = Instant::now();
            assert!(matches!(
                f.client
                    .exchange(command_address, b"{}", Duration::from_secs(30)),
                Err(ExchangeError::Unknown(_))
            ));
            assert!(start.elapsed() < Duration::from_secs(6));
        });
        let snapshot = f
            .client
            .watch(
                watch_address,
                &query,
                &omarchy_phone_core::tls::WatchCancel::default(),
            )
            .unwrap();
        assert_eq!(snapshot.source_state(), SourceState::Detached);
        command.join().unwrap();
        short.join().unwrap();
    });
    assert!(short_worker.join().unwrap() > 0);
    assert!(watcher.join().unwrap() > 0);
    assert!(commander.join().unwrap() > 0);
}

#[test]
fn watch_validates_reply_identity_inside_authenticated_exchange() {
    use omarchy_phone_core::{Identity, watch::WatchQuery};
    let f = Fixture::new(false, false, false);
    let id: Identity = serde_json::from_str("\"11111111111111111111111111111111\"").unwrap();
    let query = WatchQuery::new(id, None, 0).unwrap();
    let (address, worker) = f.serve_request(frame(b"{}"), Duration::ZERO, query.encode());
    assert!(
        f.client
            .watch(
                address,
                &query,
                &omarchy_phone_core::tls::WatchCancel::default()
            )
            .is_err()
    );
    assert!(worker.join().unwrap() > 0);
}

#[test]
fn cancelled_watch_never_connects() {
    use omarchy_phone_core::{tls::WatchCancel, watch::WatchQuery};
    let f = Fixture::new(false, false, false);
    let stop = WatchCancel::default();
    stop.cancel();
    let query = WatchQuery::new("a".repeat(32).try_into().unwrap(), None, 25000).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    assert!(
        f.client
            .watch(listener.local_addr().unwrap(), &query, &stop)
            .is_err()
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn watch_cancellation_interrupts_stalled_handshake() {
    use omarchy_phone_core::{tls::WatchCancel, watch::WatchQuery};
    let f = Fixture::new(false, false, false);
    let stop = WatchCancel::default();
    let query = WatchQuery::new("a".repeat(32).try_into().unwrap(), None, 25000).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (accepted, ready) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut raw, _) = listener.accept().unwrap();
            raw.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            accepted.send(()).unwrap();
            let mut bytes = [0; 4096];
            loop {
                match raw.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(_) => (),
                    Err(_) => break,
                }
            }
        });
        let client = scope.spawn(|| f.client.watch(address, &query, &stop));
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        let start = Instant::now();
        stop.cancel();
        assert!(client.join().unwrap().is_err());
        assert!(start.elapsed() < Duration::from_secs(1));
        server.join().unwrap();
    });
}

#[test]
fn cancellation_interrupts_authenticated_watch_without_affecting_other_exchanges() {
    use omarchy_phone_core::{tls::WatchCancel, watch::WatchQuery};
    let f = Fixture::new(false, false, false);
    let stop = WatchCancel::default();
    let query = WatchQuery::new("a".repeat(32).try_into().unwrap(), None, 25000).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (received, ready) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (raw, _) = listener.accept().unwrap();
            raw.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut stream =
                StreamOwned::new(ServerConnection::new(f.server.clone()).unwrap(), raw);
            let mut header = [0; 4];
            stream.read_exact(&mut header).unwrap();
            let mut body = vec![0; u32::from_be_bytes(header) as usize];
            stream.read_exact(&mut body).unwrap();
            assert_eq!(body, query.encode());
            received.send(()).unwrap();
            let mut byte = [0];
            assert!(!matches!(stream.read(&mut byte), Ok(1)));
        });
        let client = scope.spawn(|| f.client.watch(address, &query, &stop));
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        let start = Instant::now();
        stop.cancel();
        assert!(client.join().unwrap().is_err());
        assert!(start.elapsed() < Duration::from_secs(1));
        server.join().unwrap();
    });
    let (address, server) = f.serve(frame(b"{}"), Duration::ZERO);
    assert_eq!(
        f.client
            .exchange(address, b"{\"op\":\"status\"}", Duration::from_secs(2))
            .unwrap(),
        b"{}"
    );
    assert!(server.join().unwrap() > 0);
}

#[test]
fn background_confirmation_expires_while_authenticated_peer_stalls() {
    use omarchy_phone_core::background::BackgroundObserver;
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let f = Fixture::new(false, false, false);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let config = f.server.clone();
    let (held, ready) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        let server = scope.spawn(move || {
            for expected_wait in [25000, 0, 1000] {
                let (raw, _) = listener.accept().unwrap();
                raw.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
                let mut stream = StreamOwned::new(ServerConnection::new(config.clone()).unwrap(), raw);
                let mut header = [0; 4]; stream.read_exact(&mut header).unwrap();
                let size = u32::from_be_bytes(header) as usize; assert!(size <= 4096);
                let mut body = vec![0; size]; stream.read_exact(&mut body).unwrap();
                let query: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(query["op"], "watch"); assert_eq!(query["wait_ms"], expected_wait);
                if expected_wait == 1000 {
                    held.send(()).unwrap();
                    let mut byte = [0]; assert!(!matches!(stream.read(&mut byte), Ok(1)));
                } else {
                    if expected_wait == 0 { std::thread::sleep(Duration::from_millis(250)); }
                    let response = serde_json::to_vec(&serde_json::json!({"v":1,"id":query["id"],"generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":1,"source":"android.telecom.InCallService","source_state":"observed","ringing":[{"id":7,"episode":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]})).unwrap();
                    stream.write_all(&frame(&response)).unwrap(); stream.flush().unwrap();
                }
            }
        });
        let mut service = BackgroundObserver::start(f.client, address).unwrap();
        ready.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(
            service.lease().unwrap().unwrap().snapshot().ringing().len(),
            1
        );
        assert!(service.drain_changes().unwrap());
        assert!(!service.drain_changes().unwrap());
        let mut fds = [PollFd::new(service.timer(), PollFlags::IN)];
        assert_eq!(
            poll(
                &mut fds,
                Some(&Timespec {
                    tv_sec: 6,
                    tv_nsec: 0
                })
            )
            .unwrap(),
            1
        );
        assert!(service.expire().unwrap());
        assert!(service.lease().unwrap().is_none());
        let start = Instant::now();
        service.shutdown().unwrap();
        assert!(start.elapsed() < Duration::from_secs(1));
        server.join().unwrap();
    });
}

#[test]
fn background_failure_withdraws_and_backoff_shutdown_is_interruptible() {
    use omarchy_phone_core::background::BackgroundObserver;
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let f = Fixture::new(false, false, false);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let mut service = BackgroundObserver::start(f.client, address).unwrap();
    let changed = service.changes();
    let mut fds = [PollFd::new(&changed, PollFlags::IN)];
    assert_eq!(
        poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 2,
                tv_nsec: 0
            })
        )
        .unwrap(),
        1
    );
    assert!(service.drain_changes().unwrap());
    assert!(service.lease().unwrap().is_none());
    let start = Instant::now();
    service.shutdown().unwrap();
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
fn background_restart_discards_pre_sleep_cursor_and_requires_confirmation() {
    use omarchy_phone_core::background::BackgroundObserver;
    let f = Fixture::new(false, false, false);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let config = f.server.clone();
    let (stage_tx, stage_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        for phase in 0..2 {
            for wait in [25000, 0, 1000] {
                let (raw, _) = listener.accept().unwrap();
                raw.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut stream =
                    StreamOwned::new(ServerConnection::new(config.clone()).unwrap(), raw);
                let mut header = [0; 4];
                stream.read_exact(&mut header).unwrap();
                let size = u32::from_be_bytes(header) as usize;
                assert!(size <= 4096);
                let mut body = vec![0; size];
                stream.read_exact(&mut body).unwrap();
                let query: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(query["wait_ms"], wait);
                if wait == 25000 {
                    assert!(query.get("after_generation").is_none());
                    assert!(query.get("after_revision").is_none());
                }
                if phase == 1 && wait == 0 {
                    stage_tx.send("confirming").unwrap();
                    release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
                }
                if wait == 1000 {
                    stage_tx.send("held").unwrap();
                    let mut byte = [0];
                    assert!(!matches!(stream.read(&mut byte), Ok(1)));
                } else {
                    let response = serde_json::to_vec(&serde_json::json!({
                        "v":1,"id":query["id"],"generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        "revision":1,"source":"android.telecom.InCallService","source_state":"observed",
                        "ringing":[{"id":7,"episode":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]
                    })).unwrap();
                    stream.write_all(&frame(&response)).unwrap();
                    stream.flush().unwrap();
                }
            }
        }
    });
    let mut service = BackgroundObserver::start(f.client, address).unwrap();
    assert_eq!(
        stage_rx.recv_timeout(Duration::from_secs(3)).unwrap(),
        "held"
    );
    let before = service.lease().unwrap().unwrap().id().clone();
    service.shutdown().unwrap();
    assert!(service.lease().unwrap().is_none());
    assert!(!service.timer().consume().unwrap());
    service.restart().unwrap();
    assert_eq!(
        stage_rx.recv_timeout(Duration::from_secs(3)).unwrap(),
        "confirming"
    );
    assert!(service.lease().unwrap().is_none());
    release_tx.send(()).unwrap();
    assert_eq!(
        stage_rx.recv_timeout(Duration::from_secs(3)).unwrap(),
        "held"
    );
    assert_ne!(service.lease().unwrap().unwrap().id(), &before);
    service.shutdown().unwrap();
    server.join().unwrap();
}
