#![cfg(target_os = "linux")]
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};
struct Fixture {
    path: PathBuf,
    listener: std::net::TcpListener,
    server: std::sync::Arc<rustls::ServerConfig>,
}
impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("phone-daemon-{}", uuid::Uuid::new_v4().simple()));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        for name in ["enrollment", "runtime"] {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(path.join(name))
                .unwrap();
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let identity = rcgen::generate_simple_self_signed(vec!["android.test".into()]).unwrap();
        for (name,content) in [("ca.pem",identity.cert.pem()),("client.pem",identity.cert.pem()),("client.key",identity.signing_key.serialize_pem()),("endpoint.json",serde_json::json!({"endpoint":listener.local_addr().unwrap().to_string(),"server_name":"android.test","server_sha256":format!("{:x}",Sha256::digest(identity.cert.der().as_ref()))}).to_string())] {
            let target=path.join("enrollment").join(name);fs::write(&target,content).unwrap();fs::set_permissions(target,fs::Permissions::from_mode(0o600)).unwrap();
        }
        let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
        let mut roots = rustls::RootCertStore::empty();
        roots.add(identity.cert.der().clone()).unwrap();
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            std::sync::Arc::new(roots),
            provider.clone(),
        )
        .build()
        .unwrap();
        let server = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![identity.cert.der().clone()],
                rustls::pki_types::PrivatePkcs8KeyDer::from(identity.signing_key.serialize_der())
                    .into(),
            )
            .unwrap();
        Self {
            path,
            listener,
            server: std::sync::Arc::new(server),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn start(path: &Path) -> Process {
    let mut child = Process(
        Command::new(env!("CARGO_BIN_EXE_omarchy-phoned"))
            .arg("--enrollment")
            .arg(path.join("enrollment"))
            .arg("--state")
            .arg(path.join("state"))
            .arg("--runtime")
            .arg(path.join("runtime"))
            .stderr(fs::File::create(path.join("daemon.stderr")).unwrap())
            .spawn()
            .unwrap(),
    );
    let end = Instant::now() + Duration::from_secs(3);
    while !path.join("runtime/bridge.sock").exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "daemon exited before readiness"
        );
        assert!(Instant::now() < end, "readiness deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
    child
}
fn hello(path: &Path) -> (omarchy_phone_core::Identity, omarchy_phone_core::Identity) {
    let reply = omarchy_phone_core::local_client::exchange(
        &path.join("runtime/bridge.sock"),
        b"{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"op\":\"hello\"}\n",
        Duration::from_secs(2),
    )
    .unwrap();
    match reply.body() {
        omarchy_phone_core::local_reply::Body::Hello { epoch, session } => {
            (epoch.clone(), session.clone())
        }
        _ => panic!("daemon did not return hello"),
    }
}
fn stop(child: &mut Process) {
    rustix::process::kill_process(
        rustix::process::Pid::from_raw(child.0.id() as i32).unwrap(),
        rustix::process::Signal::TERM,
    )
    .unwrap();
    let end = Instant::now() + Duration::from_secs(4);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < end, "shutdown deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn real_daemon_serves_rpc_and_restarts_with_preserved_journal() {
    let f = Fixture::new();
    let mut first = start(&f.path);
    let before = hello(&f.path);
    stop(&mut first);
    assert!(!f.path.join("runtime/bridge.sock").exists());
    let mut second = start(&f.path);
    let after = hello(&f.path);
    assert_eq!(before.0, after.0);
    assert_ne!(before.1, after.1);
    stop(&mut second);
}

#[test]
fn cli_uses_real_daemon_and_reports_unsent_input() {
    use std::io::Write;
    use std::process::Stdio;
    let f = Fixture::new();
    let mut daemon = start(&f.path);
    let invoke = |input: &[u8]| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_omarchy-phone"))
            .arg("--socket")
            .arg(f.path.join("runtime/bridge.sock"))
            .arg("rpc")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    };
    let hello = invoke(b"{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"op\":\"hello\"}\n");
    assert!(
        hello.status.success(),
        "{}",
        String::from_utf8_lossy(&hello.stderr)
    );
    let reply: serde_json::Value = serde_json::from_slice(&hello.stdout).unwrap();
    assert_eq!(reply["result"]["kind"], "hello");
    let lookup = serde_json::json!({"v":1,"op":"lookup","id":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","epoch":reply["result"]["epoch"],"target":"cccccccccccccccccccccccccccccccc"}).to_string()+"\n";
    let found = invoke(lookup.as_bytes());
    assert!(found.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&found.stdout).unwrap();
    assert_eq!(receipt["result"]["kind"], "receipt");
    assert!(receipt["result"]["receipt"].is_null());
    let acknowledgement = serde_json::json!({"v":1,"op":"acknowledge","id":"dddddddddddddddddddddddddddddddd","epoch":reply["result"]["epoch"],"target":"cccccccccccccccccccccccccccccccc"}).to_string()+"\n";
    let refused = invoke(acknowledgement.as_bytes());
    assert_eq!(refused.status.code(), Some(4));
    let error: serde_json::Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(error["result"]["kind"], "error");
    let invalid = invoke(b"{\"private_field\":\"do-not-log-this\"}\n");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&invalid.stderr).contains("do-not-log-this"));
    stop(&mut daemon);
}

#[test]
fn cli_lost_reply_is_unknown_without_retry() {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::net::UnixListener,
        process::Stdio,
    };
    let f = Fixture::new();
    let path = f.path.join("runtime/bridge.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_omarchy-phone"))
        .arg("--socket")
        .arg(&path)
        .arg("rpc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"op\":\"hello\"}\n")
        .unwrap();
    let (peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut input = String::new();
    BufReader::new(peer).read_line(&mut input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn daemon_observes_without_ui_and_cancels_a_held_watch_on_shutdown() {
    exercise_background_observation(false);
}

#[test]
fn daemon_keeps_rpc_available_after_presentation_storage_failure() {
    exercise_background_observation(true);
}

fn exercise_background_observation(fail_storage: bool) {
    use std::io::{Read, Write};
    let fixture = Fixture::new();
    if fail_storage {
        // Fault only the presentation write, using the real daemon database.
        // Command receipts and startup validation must remain available.
        drop(omarchy_phone_core::ledger::Ledger::open(&fixture.path.join("state")).unwrap());
        let db = rusqlite::Connection::open(fixture.path.join("state/journal.sqlite3")).unwrap();
        db.execute_batch("CREATE TRIGGER reject_policy BEFORE INSERT ON notification_episodes BEGIN SELECT RAISE(ABORT, 'injected presentation failure'); END;").unwrap();
    }
    let listener = fixture.listener.try_clone().unwrap();
    listener.set_nonblocking(true).unwrap();
    let config = fixture.server.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(6);
        for index in 0..3 {
            let raw = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "watch connection deadline");
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("watch accept: {error}"),
                }
            };
            raw.set_nonblocking(false).unwrap();
            raw.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
            raw.set_write_timeout(Some(Duration::from_secs(4))).unwrap();
            let mut stream = rustls::StreamOwned::new(
                rustls::ServerConnection::new(config.clone()).unwrap(),
                raw,
            );
            let mut prefix = [0; 4];
            stream.read_exact(&mut prefix).unwrap();
            let size = u32::from_be_bytes(prefix) as usize;
            assert!(size <= 4096);
            let mut body = vec![0; size];
            stream.read_exact(&mut body).unwrap();
            let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(request["op"], "watch");
            assert_eq!(request["wait_ms"], [25000, 0, 1000][index]);
            if index == 2 {
                ready_tx.send(()).unwrap();
                let mut extra = [0];
                match stream.read(&mut extra) {
                    Ok(0) => (),
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
                        ) => {}
                    other => panic!("shutdown must close held watch: {other:?}"),
                }
                return;
            }
            let reply = serde_json::to_vec(&serde_json::json!({"v":1,"id":request["id"],
                "generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":1,
                "source":"android.telecom.InCallService","source_state":"observed",
                "ringing":[{"id":7,"episode":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]}))
            .unwrap();
            stream
                .write_all(&(reply.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&reply).unwrap();
            stream.flush().unwrap();
        }
    });
    let mut daemon = start(&fixture.path);
    ready_rx
        .recv_timeout(Duration::from_secs(4))
        .expect("daemon must watch without a UI connection");
    let db = rusqlite::Connection::open(fixture.path.join("state/journal.sqlite3")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let count: u32 = db
            .query_row(
                "SELECT count(*) FROM notification_episodes WHERE call_id=7 AND dismissed=0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        if fail_storage {
            assert_eq!(count, 0, "failed policy must not be published as committed");
            if fs::read_to_string(fixture.path.join("daemon.stderr"))
                .unwrap()
                .contains("phone presentation journal unavailable")
            {
                break;
            }
        } else if count == 1 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "confirmed observation must reach coordinator storage"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    hello(&fixture.path);
    let stopped = Instant::now();
    stop(&mut daemon);
    assert!(stopped.elapsed() < Duration::from_secs(2));
    peer.join().unwrap();
    assert!(!fixture.path.join("runtime/bridge.sock").exists());
    if fail_storage {
        assert_eq!(
            fs::read_to_string(fixture.path.join("daemon.stderr")).unwrap(),
            "phone presentation journal unavailable\n"
        );
        let count: u32 = db
            .query_row("SELECT count(*) FROM notification_state", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            count, 0,
            "the failed transaction must roll back its state row"
        );
    }
}
