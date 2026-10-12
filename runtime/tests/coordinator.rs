#![cfg(target_os = "linux")]
use omarchy_phone_core::{
    Identity,
    coordinator::Coordinator,
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
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{Arc, mpsc},
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

fn frame(epoch: &Identity, session: &Identity, admission: &Identity, id: u32) -> UnixStream {
    let (mut client, peer) = UnixStream::pair().unwrap();
    let value = serde_json::json!({"op":"submit","v":1,"id":format!("{id:032x}"),"epoch":epoch,"session":session,"admission":admission,"command":{"kind":"dial","generation":"dddddddddddddddddddddddddddddddd","offer":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","number":"5550100"}});
    writeln!(client, "{value}").unwrap();
    // The service owns the command even after the local app closes.
    drop(client);
    peer
}
#[test]
fn concurrent_actions_are_busy_and_owner_survives_ui_disconnect() {
    let dir = Scratch(
        std::env::temp_dir().join(format!("phone-owner-{}", uuid::Uuid::new_v4().simple())),
    );
    let ledger = Ledger::open(&dir.0).unwrap();
    let (tls, config) = client_server();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let observer = listener.try_clone().unwrap();
    let owner = Coordinator::new(ledger, tls, listener.local_addr().unwrap()).unwrap();
    let (_, peer) = UnixStream::pair().unwrap();
    let (epoch, session) = owner.hello(&peer).unwrap();
    let (admission, _) = owner.fence(&peer, &epoch, &session).unwrap();
    let (arrived_tx, arrived_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (raw, _) = listener.accept().unwrap();
        raw.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        raw.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut stream = StreamOwned::new(ServerConnection::new(config).unwrap(), raw);
        let mut prefix = [0; 4];
        stream.read_exact(&mut prefix).unwrap();
        let mut request = vec![0; u32::from_be_bytes(prefix) as usize];
        stream.read_exact(&mut request).unwrap();
        arrived_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let reply=serde_json::json!({"v":1,"id":format!("{:032x}",1),"generation":"dddddddddddddddddddddddddddddddd","outcome":"invocation_attempted","reason":"returned"}).to_string();
        stream
            .write_all(&(reply.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(reply.as_bytes()).unwrap();
        stream.flush().unwrap();
    });
    assert_eq!(
        owner
            .submit(&mut frame(&epoch, &session, &admission, 1))
            .unwrap(),
        Outcome::Pending
    );
    arrived_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let (next, items) = owner.fence(&peer, &epoch, &session).unwrap();
    assert_ne!(next, admission);
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].outcome(),
        omarchy_phone_core::local_reply::Outcome::Pending
    );
    assert!(
        owner
            .submit(&mut frame(&epoch, &session, &next, 2))
            .is_err()
    );

    assert_eq!(
        owner
            .submit(&mut frame(&epoch, &session, &admission, 1))
            .unwrap(),
        Outcome::Pending
    );
    assert!(
        owner
            .submit(&mut frame(&epoch, &session, &admission, 2))
            .is_err()
    );
    let second = Identity::try_from(format!("{:032x}", 2)).unwrap();
    assert!(owner.lookup(&peer, &epoch, &second).unwrap().is_none());
    assert!(
        Ledger::open(&dir.0).is_err(),
        "ownership remains exclusive during work"
    );
    release_tx.send(()).unwrap();
    // Drop must join service-owned work before releasing the ledger lock.
    drop(owner);
    server.join().unwrap();
    let recovered = Ledger::open(&dir.0).unwrap();
    let first = Identity::try_from(format!("{:032x}", 1)).unwrap();
    assert_eq!(
        recovered.lookup(&epoch, &first).unwrap().unwrap().outcome(),
        Outcome::DialInvoked
    );
    assert!(recovered.lookup(&epoch, &second).unwrap().is_none());
    observer.set_nonblocking(true).unwrap();
    assert_eq!(
        observer.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn empty_recovery_is_only_a_snapshot_not_an_admission_barrier() {
    let dir = Scratch(std::env::temp_dir().join(format!(
        "phone-late-admission-{}",
        uuid::Uuid::new_v4().simple()
    )));
    let (tls, _) = client_server();
    let endpoint = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = endpoint.local_addr().unwrap();
    drop(endpoint);
    let owner = Coordinator::new(Ledger::open(&dir.0).unwrap(), tls, address).unwrap();
    let (_ui, service) = UnixStream::pair().unwrap();
    let (epoch, session) = owner.hello(&service).unwrap();
    let (admission, _) = owner.fence(&service, &epoch, &session).unwrap();
    // A complete command is buffered; its caller has already disconnected.
    let mut delayed = frame(&epoch, &session, &admission, 991);
    assert!(owner.recovery(&service, &epoch).unwrap().is_empty());
    // Read-only recovery cannot revoke this later admission. A separate fence
    // is required before UI readiness can promise old intent is excluded.
    assert_eq!(owner.submit(&mut delayed).unwrap(), Outcome::Pending);
}

#[test]
fn fence_excludes_delayed_unadmitted_commands_without_dispatch() {
    let dir = Scratch(
        std::env::temp_dir().join(format!("phone-fence-{}", uuid::Uuid::new_v4().simple())),
    );
    let (tls, _) = client_server();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let owner = Coordinator::new(
        Ledger::open(&dir.0).unwrap(),
        tls,
        listener.local_addr().unwrap(),
    )
    .unwrap();
    let (_, peer) = UnixStream::pair().unwrap();
    let (epoch, session) = owner.hello(&peer).unwrap();
    let (old_token, items) = owner.fence(&peer, &epoch, &session).unwrap();
    assert!(items.is_empty());
    let mut delayed = frame(&epoch, &session, &old_token, 992);
    let (new_token, items) = owner.fence(&peer, &epoch, &session).unwrap();
    assert_ne!(old_token, new_token);
    assert!(owner.submit(&mut delayed).is_err());
    let id = Identity::try_from(format!("{:032x}", 992)).unwrap();
    assert!(owner.lookup(&peer, &epoch, &id).unwrap().is_none());
    assert!(items.is_empty());
    assert_eq!(owner.hello(&peer).unwrap().1, session);
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

fn ringing(episode: &str) -> omarchy_phone_core::watch::Snapshot {
    let id = Identity::try_from("dddddddddddddddddddddddddddddddd".to_owned()).unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({"v":1,"id":id,
        "generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":1,
        "source":"android.telecom.InCallService","source_state":"observed",
        "ringing":[{"id":7,"episode":episode}]}))
    .unwrap();
    omarchy_phone_core::watch::decode_snapshot(&bytes, &id).unwrap()
}
#[test]
fn coordinator_restores_dismissal_before_returning_presentation_policy() {
    let dir =
        Scratch(std::env::temp_dir().join(format!("phone-policy-owner-{}", uuid::Uuid::new_v4())));
    let (tls, _) = client_server();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let owner = Coordinator::new(
        Ledger::open(&dir.0).unwrap(),
        tls.clone(),
        listener.local_addr().unwrap(),
    )
    .unwrap();
    let current = ringing("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    assert!(owner.observe_presentation(&current).unwrap());
    assert!(owner.claim_presentation(&current).unwrap());
    assert!(!owner.claim_presentation(&current).unwrap());
    assert!(!owner.dismiss_presentation(&current, &current).unwrap());
    drop(owner);
    let owner = Coordinator::new(
        Ledger::open(&dir.0).unwrap(),
        tls,
        listener.local_addr().unwrap(),
    )
    .unwrap();
    assert!(!owner.observe_presentation(&current).unwrap());
    assert!(!owner.claim_presentation(&current).unwrap());
    assert!(
        owner
            .claim_presentation(&ringing("cccccccccccccccccccccccccccccccc"))
            .unwrap()
    );
    assert!(
        owner
            .observe_presentation(&ringing("cccccccccccccccccccccccccccccccc"))
            .unwrap()
    );
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
#[test]
fn failed_presentation_commit_withdraws_eligibility_without_blocking_command_session() {
    let dir =
        Scratch(std::env::temp_dir().join(format!("phone-policy-fault-{}", uuid::Uuid::new_v4())));
    let (tls, _) = client_server();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let owner = Coordinator::new(
        Ledger::open(&dir.0).unwrap(),
        tls,
        listener.local_addr().unwrap(),
    )
    .unwrap();
    let current = ringing("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    assert!(owner.observe_presentation(&current).unwrap());
    let fault = rusqlite::Connection::open(dir.0.join("journal.sqlite3")).unwrap();
    fault.execute_batch("CREATE TRIGGER reject_policy BEFORE INSERT ON notification_episodes BEGIN SELECT RAISE(ABORT, 'injected policy failure'); END;").unwrap();
    // Identical healthy observations must not write the journal repeatedly.
    assert!(owner.observe_presentation(&current).unwrap());
    assert!(owner.dismiss_presentation(&current, &current).is_err());
    assert!(owner.claim_presentation(&current).is_err());
    fault.execute("DROP TRIGGER reject_policy", []).unwrap();
    assert!(
        owner.observe_presentation(&current).is_err(),
        "failed persistence must not silently restore presentation"
    );
    assert_eq!(
        fault
            .query_row(
                "SELECT dismissed FROM notification_episodes WHERE call_id=7",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    let (_client, service) = UnixStream::pair().unwrap();
    let (epoch, session) = owner.hello(&service).unwrap();
    assert!(
        owner
            .fence(&service, &epoch, &session)
            .unwrap()
            .1
            .is_empty()
    );
}

#[test]
fn coordinator_startup_rejects_corrupt_presentation_state_without_reset() {
    let dir = Scratch(
        std::env::temp_dir().join(format!("phone-policy-corrupt-{}", uuid::Uuid::new_v4())),
    );
    let (tls, _) = client_server();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let owner = Coordinator::new(
        Ledger::open(&dir.0).unwrap(),
        tls.clone(),
        listener.local_addr().unwrap(),
    )
    .unwrap();
    owner
        .observe_presentation(&ringing("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"))
        .unwrap();
    drop(owner);
    let fault = rusqlite::Connection::open(dir.0.join("journal.sqlite3")).unwrap();
    fault
        .execute("UPDATE notification_state SET generation='invalid'", [])
        .unwrap();
    assert!(
        Coordinator::new(
            Ledger::open(&dir.0).unwrap(),
            tls,
            listener.local_addr().unwrap()
        )
        .is_err()
    );
    assert_eq!(
        fault
            .query_row("SELECT generation FROM notification_state", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "invalid"
    );
}
