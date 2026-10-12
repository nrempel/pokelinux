#![cfg(target_os = "linux")]
use omarchy_phone_core::{
    decode_request,
    dispatch::execute,
    ledger::{Ledger, Outcome},
    tls::TlsClient,
};
use rustls::{
    RootCertStore, ServerConfig, ServerConnection, StreamOwned, pki_types::PrivatePkcs8KeyDer,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn client_server() -> (TlsClient, Arc<ServerConfig>) {
    let server = rcgen::generate_simple_self_signed(vec!["android.test".into()]).unwrap();
    let client = rcgen::generate_simple_self_signed(vec!["client.test".into()]).unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut roots = RootCertStore::empty();
    roots.add(client.cert.der().clone()).unwrap();
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        provider.clone(),
    )
    .build()
    .unwrap();
    let config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![server.cert.der().clone()],
            PrivatePkcs8KeyDer::from(server.signing_key.serialize_der()).into(),
        )
        .unwrap();
    let tls = TlsClient::new(
        vec![server.cert.der().clone()],
        vec![client.cert.der().clone()],
        PrivatePkcs8KeyDer::from(client.signing_key.serialize_der()).into(),
        "android.test".into(),
        Sha256::digest(server.cert.der().as_ref()).into(),
    )
    .unwrap();
    (tls, Arc::new(config))
}
fn setup() -> (Scratch, Ledger, omarchy_phone_core::Request) {
    let dir = Scratch(
        std::env::temp_dir().join(format!("phone-dispatch-{}", uuid::Uuid::new_v4().simple())),
    );
    let ledger = Ledger::open(&dir.0).unwrap();
    let value = serde_json::json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","epoch":ledger.epoch(),"session":"cccccccccccccccccccccccccccccccc","command":{"kind":"dial","generation":"dddddddddddddddddddddddddddddddd","offer":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","number":"5550100"}});
    let request = decode_request(format!("{value}\n").as_bytes()).unwrap();
    (dir, ledger, request)
}
#[test]
fn local_not_sent_is_durable_and_is_not_an_android_receipt() {
    let (dir, mut ledger, request) = setup();
    let (tls, _) = client_server();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    // Zero budget deterministically fails before a connection or write.
    assert_eq!(
        execute(&mut ledger, &tls, address, &request, Duration::ZERO).unwrap(),
        Outcome::NotSent
    );
    drop(ledger);
    let mut ledger = Ledger::open(&dir.0).unwrap();
    assert_eq!(
        execute(&mut ledger, &tls, address, &request, Duration::from_secs(1)).unwrap(),
        Outcome::NotSent
    );
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(ledger.unresolved().unwrap().is_empty());
}
#[test]
fn real_tls_dispatch_requires_committed_admission_and_never_replays() {
    for (reply, expected) in [
        (
            r#"{"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","generation":"dddddddddddddddddddddddddddddddd","outcome":"invocation_attempted","reason":"returned"}"#,
            Outcome::DialInvoked,
        ),
        (
            r#"{"v":1,"id":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","generation":"dddddddddddddddddddddddddddddddd","outcome":"invocation_attempted","reason":"returned"}"#,
            Outcome::Unknown,
        ),
        ("{}", Outcome::Unknown),
    ] {
        let (dir, mut ledger, request) = setup();
        let (tls, config) = client_server();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let observer = listener.try_clone().unwrap();
        let db_path = dir.0.join("journal.sqlite3");
        let worker = std::thread::spawn(move || {
            let (raw, _) = listener.accept().unwrap();
            raw.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            raw.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
            let db = rusqlite::Connection::open(db_path).unwrap();
            let state: String = db
                .query_row("SELECT outcome FROM operations", [], |r| r.get(0))
                .unwrap();
            assert_eq!(state, "pending", "admission must commit before connecting");
            let mut stream = StreamOwned::new(ServerConnection::new(config).unwrap(), raw);
            let mut prefix = [0; 4];
            stream.read_exact(&mut prefix).unwrap();
            let mut body = vec![0; u32::from_be_bytes(prefix) as usize];
            stream.read_exact(&mut body).unwrap();
            let command: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(command["op"], "dial");
            stream
                .write_all(&(reply.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(reply.as_bytes()).unwrap();
            stream.flush().unwrap();
        });
        assert_eq!(
            execute(&mut ledger, &tls, address, &request, Duration::from_secs(2)).unwrap(),
            expected
        );
        worker.join().unwrap();
        drop(ledger);
        let mut ledger = Ledger::open(&dir.0).unwrap();
        assert_eq!(
            execute(&mut ledger, &tls, address, &request, Duration::from_secs(2)).unwrap(),
            expected
        );
        observer.set_nonblocking(true).unwrap();
        assert_eq!(
            observer.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert_eq!(
            ledger.unresolved().unwrap().is_empty(),
            expected != Outcome::Unknown
        );
    }
}
