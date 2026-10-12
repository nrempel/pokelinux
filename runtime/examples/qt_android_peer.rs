//! Disposable authenticated peer for native QML integration tests. Never packaged.
#[cfg(target_os = "linux")]
fn main() {
    use rustls::{
        RootCertStore, ServerConfig, ServerConnection, StreamOwned, pki_types::PrivatePkcs8KeyDer,
    };
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::Arc,
        time::Duration,
    };
    let path = PathBuf::from(std::env::args_os().nth(1).expect("fresh enrollment path"));
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
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
    let config = Arc::new(
        ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![server.cert.der().clone()],
                PrivatePkcs8KeyDer::from(server.signing_key.serialize_der()).into(),
            )
            .unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    for (name, content) in [
        ("ca.pem",server.cert.pem()),
        ("client.pem",client.cert.pem()),
        ("client.key",client.signing_key.serialize_pem()),
        ("endpoint.json",json!({"endpoint":listener.local_addr().unwrap(),"server_name":"android.test","server_sha256":format!("{:x}",Sha256::digest(server.cert.der().as_ref()))}).to_string()),
    ] {
        let file=path.join(name); fs::write(&file,content).unwrap();
        fs::set_permissions(file,fs::Permissions::from_mode(0o600)).unwrap();
    }
    println!("ready");
    std::io::stdout().flush().unwrap();
    // Stateful synthetic peer: observation reads may repeat. Exactly one
    // validated Answer followed by one Dial is accepted. The fixture models
    // the incoming call ending after Answer; it does not emulate call audio.
    let mut answered = false;
    let mut controls_seen = false;
    let mut busy_seen = false;
    let mut offer_seen = false;
    let mut answer_id = serde_json::Value::Null;
    for _ in 0..32 {
        let (raw, _) = listener.accept().unwrap();
        raw.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        raw.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut stream = StreamOwned::new(ServerConnection::new(config.clone()).unwrap(), raw);
        let mut length = [0; 4];
        stream.read_exact(&mut length).unwrap();
        let length = u32::from_be_bytes(length) as usize;
        assert!(length <= 16384);
        let mut bytes = vec![0; length];
        stream.read_exact(&mut bytes).unwrap();
        let query: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let generation = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let offer = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        let operation = query["op"].as_str().expect("operation");
        let reply = match operation {
            // The production daemon now observes independently of open windows.
            // This synthetic peer replies immediately; it does not model held
            // long polls, modem state, ringing audio or Android lifecycle.
            "watch" => json!({"v":1,"id":query["id"],"generation":generation,
                "revision":if answered {8} else {7},
                "source":"android.telecom.InCallService","source_state":"observed",
                "ringing":if answered {json!([])} else {json!([{"id":1,"episode":"ffffffffffffffffffffffffffffffff"}])}}),
            "controls" => {
                controls_seen = true;
                let calls = if answered { json!([]) } else {
                    json!([{"id":1,"state":2,"actions":["answer","reject"]}])
                };
                json!({"v":1,"id":query["id"],"generation":generation,"revision":if answered {8} else {7},"controls_allowed":true,"availability":if answered {"unavailable"} else {"observed"},"source":"android.telecom.InCallService","calls":calls})
            }
            "control" => {
                assert!(!answered && controls_seen && busy_seen);
                assert_eq!(query["generation"],generation);
                assert_eq!(query["call_id"],1);
                assert_eq!(query["expected_revision"],7);
                assert_eq!(query["expected_state"],2);
                assert_eq!(query["action"],"answer");
                answered = true;
                answer_id = query["id"].clone();
                json!({"v":1,"id":query["id"],"generation":generation,"outcome":"submitted","reason":"accepted"})
            }
            "dial_status" => {
                if answered { offer_seen = true; } else { busy_seen = true; }
                json!({"v":1,"id":query["id"],"generation":generation,"can_dial":answered,"offer":if answered {offer} else {""},"pending":false,"reason":if answered {"ready"} else {"busy"}})
            }
            "dial" => {
                assert!(answered && offer_seen);
                assert_ne!(query["id"],answer_id);
                assert_eq!(query["generation"],generation);
                assert_eq!(query["offer"],offer);
                assert_eq!(query["number"],"5550100");
                json!({"v":1,"id":query["id"],"generation":generation,"outcome":"invocation_attempted","reason":"returned"})
            }
            _ => panic!("unexpected operation"),
        }.to_string();
        stream
            .write_all(&(reply.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(reply.as_bytes()).unwrap();
        stream.flush().unwrap();
        if operation == "dial" {
            println!("two authenticated commands verified");
            return;
        }
    }
    panic!("fixture observation budget exhausted");
}
#[cfg(not(target_os = "linux"))]
fn main() {
    panic!("Linux fixture required");
}
