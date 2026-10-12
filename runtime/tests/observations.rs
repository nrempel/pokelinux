#![cfg(target_os = "linux")]
use omarchy_phone_core::{
    coordinator::Coordinator, ledger::Ledger, local_protocol, local_reply, rpc, tls::TlsClient,
};
use rustls::{
    RootCertStore, ServerConfig, ServerConnection, StreamOwned, pki_types::PrivatePkcs8KeyDer,
};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    os::unix::net::UnixStream,
    sync::Arc,
    time::Duration,
};
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
#[test]
fn observations_cross_local_rpc_and_authenticated_android_transport() {
    for op in ["status", "controls", "dial_status"] {
        for corruption in ["valid", "wrong_id", "duplicate", "oversize"] {
            let (tls, config) = client_server();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let peer = std::thread::spawn(move || {
                let (raw, _) = listener.accept().unwrap();
                raw.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut stream = StreamOwned::new(ServerConnection::new(config).unwrap(), raw);
                let mut length = [0; 4];
                stream.read_exact(&mut length).unwrap();
                let mut bytes = vec![0; u32::from_be_bytes(length) as usize];
                stream.read_exact(&mut bytes).unwrap();
                let query: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(query["op"], op);
                let reply=serde_json::json!({"v":1,"id":query["id"],"generation":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","observed_ms":123,"dialer_role":true,"availability":"observed","source":"android.telecom.InCallService","calls":[{"id":1,"state":2}],"phone_state":{"availability":"observed","state":1,"source":"android.telephony.TelephonyCallback","scope":"default_subscription"}}).to_string();
                let reply = match op {
                "controls" => serde_json::json!({"v":1,"id":query["id"],"generation":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","revision":7,"controls_allowed":true,"availability":"observed","source":"android.telecom.InCallService","calls":[{"id":1,"state":2,"actions":["answer","reject"]}]}).to_string(),
                "dial_status" => serde_json::json!({"v":1,"id":query["id"],"generation":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","can_dial":true,"offer":"dddddddddddddddddddddddddddddddd","pending":false,"reason":"ready"}).to_string(),
                _ => reply,
            };
                let reply = match corruption {
                    "wrong_id" => reply.replace(
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        "cccccccccccccccccccccccccccccccc",
                    ),
                    "duplicate" => reply.replace("\"v\":1", "\"v\":1,\"v\":1"),
                    "oversize" => " ".repeat(65537),
                    _ => reply,
                };
                stream
                    .write_all(&(reply.len() as u32).to_be_bytes())
                    .unwrap();
                if corruption != "oversize" {
                    stream.write_all(reply.as_bytes()).unwrap();
                }
                stream.flush().unwrap();
            });
            let path = std::env::temp_dir()
                .join(format!("phone-observe-{}", uuid::Uuid::new_v4().simple()));
            let owner = Coordinator::new(Ledger::open(&path).unwrap(), tls, address).unwrap();
            let (mut app, mut service) = UnixStream::pair().unwrap();
            let frame = format!(
                "{}\n",
                serde_json::json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","op":op})
            );
            let frame = frame.as_bytes();
            app.write_all(frame).unwrap();
            rpc::serve(&owner, &mut service).unwrap();
            drop(service);
            let mut bytes = Vec::new();
            app.read_to_end(&mut bytes).unwrap();
            let reply =
                local_reply::decode(&bytes, &local_protocol::decode(frame).unwrap()).unwrap();
            if corruption == "valid" {
                match reply.body() {
                    local_reply::Body::Status { snapshot } => {
                        assert_eq!(snapshot.calls().len(), 1);
                        assert_eq!(snapshot.phone_state(), Some(1));
                    }
                    local_reply::Body::Controls { snapshot } => {
                        assert_eq!(snapshot.revision(), 7);
                        assert_eq!(snapshot.calls()[0].actions().len(), 2);
                    }
                    local_reply::Body::DialStatus { snapshot } => {
                        assert!(snapshot.offer().is_some());
                        assert!(!snapshot.pending());
                    }
                    _ => panic!("expected observation"),
                }
            } else {
                assert!(matches!(
                    reply.body(),
                    local_reply::Body::Error {
                        code: local_reply::ErrorCode::Unavailable
                    }
                ));
            }
            peer.join().unwrap();
            drop(owner);
            assert!(
                Ledger::open(&path)
                    .unwrap()
                    .unresolved()
                    .unwrap()
                    .is_empty()
            );
            std::fs::remove_dir_all(path).unwrap();
        }
    }
}
