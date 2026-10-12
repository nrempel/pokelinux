#![cfg(target_os = "linux")]
use omarchy_phone_core::{coordinator::Coordinator, ledger::Ledger, rpc, tls::TlsClient};
use rustls::pki_types::PrivatePkcs8KeyDer;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    os::unix::net::UnixStream,
    path::PathBuf,
    time::Duration,
};
struct Fixture {
    path: PathBuf,
    owner: Option<Coordinator>,
    endpoint: TcpListener,
}
impl Fixture {
    fn new() -> Self {
        Self::with_pending(false)
    }
    fn with_pending(pending: bool) -> Self {
        let path =
            std::env::temp_dir().join(format!("phone-rpc-{}", uuid::Uuid::new_v4().simple()));
        let mut ledger = Ledger::open(&path).unwrap();
        if pending {
            let frame = format!(
                "{}\n",
                serde_json::json!({"v":1,"id":"cccccccccccccccccccccccccccccccc","epoch":ledger.epoch(),"session":"dddddddddddddddddddddddddddddddd","command":{"kind":"dial","generation":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","offer":"ffffffffffffffffffffffffffffffff","number":"5550100"}})
            );
            ledger
                .reserve(&omarchy_phone_core::decode_request(frame.as_bytes()).unwrap())
                .unwrap();
            drop(ledger);
            ledger = Ledger::open(&path).unwrap();
        }

        let identity = rcgen::generate_simple_self_signed(vec!["android.test".into()]).unwrap();
        let client = TlsClient::new(
            vec![identity.cert.der().clone()],
            vec![identity.cert.der().clone()],
            PrivatePkcs8KeyDer::from(identity.signing_key.serialize_der()).into(),
            "android.test".into(),
            Sha256::digest(identity.cert.der().as_ref()).into(),
        )
        .unwrap();
        let endpoint = TcpListener::bind("127.0.0.1:0").unwrap();
        let owner = Coordinator::new(ledger, client, endpoint.local_addr().unwrap()).unwrap();
        Self {
            path,
            owner: Some(owner),
            endpoint,
        }
    }
    fn exchange(&self, request: serde_json::Value) -> serde_json::Value {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        writeln!(client, "{request}").unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        rpc::serve(self.owner.as_ref().unwrap(), &mut server).unwrap();
        drop(server);
        let mut reply = String::new();
        client.read_to_string(&mut reply).unwrap();
        assert!(reply.ends_with('\n'));
        let expected =
            omarchy_phone_core::local_protocol::decode(format!("{request}\n").as_bytes()).unwrap();
        omarchy_phone_core::local_reply::decode(reply.as_bytes(), &expected).unwrap();
        serde_json::from_str(&reply).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.owner.take());
        let _ = fs::remove_dir_all(&self.path);
    }
}
#[test]
fn hello_and_absent_lookup_do_not_connect_or_admit() {
    let f = Fixture::new();
    let id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let hello = f.exchange(serde_json::json!({"v":1,"id":id,"op":"hello"}));
    assert_eq!(hello["id"], id);
    assert_eq!(hello["v"], 1);
    assert_eq!(hello["result"]["kind"], "hello");
    assert_eq!(hello["result"]["session"].as_str().unwrap().len(), 32);
    let lookup=f.exchange(serde_json::json!({"v":1,"id":id,"op":"lookup","epoch":hello["result"]["epoch"],"target":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}));
    assert_eq!(lookup["result"]["kind"], "receipt");
    assert!(lookup["result"]["receipt"].is_null());
    let refused=f.exchange(serde_json::json!({"v":1,"id":id,"op":"acknowledge","epoch":hello["result"]["epoch"],"target":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}));
    assert_eq!(refused["result"]["kind"], "error");
    f.endpoint.set_nonblocking(true).unwrap();
    assert!(f.endpoint.accept().is_err());
}
#[test]
fn malformed_frame_is_closed_without_a_reflected_payload() {
    let f = Fixture::new();
    let (mut client, mut server) = UnixStream::pair().unwrap();
    client
        .write_all(b"{\"v\":1,\"op\":\"hello\",\"id\":\"private-invalid-data\"}\n")
        .unwrap();
    assert!(rpc::serve(f.owner.as_ref().unwrap(), &mut server).is_err());
    drop(server);
    let mut reply = Vec::new();
    client.read_to_end(&mut reply).unwrap();
    assert!(reply.is_empty());
}

#[test]
fn recovery_discovers_unknown_after_restart_without_dispatch() {
    let f = Fixture::with_pending(true);
    let id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let hello = f.exchange(serde_json::json!({"v":1,"id":id,"op":"hello"}));
    let query = serde_json::json!({"v":1,"id":id,"op":"recovery","epoch":hello["result"]["epoch"]});
    let recovery = f.exchange(query.clone());
    assert_eq!(recovery["result"]["kind"], "recovery");
    assert_eq!(
        recovery["result"]["items"][0]["id"],
        "cccccccccccccccccccccccccccccccc"
    );
    assert_eq!(recovery["result"]["items"][0]["outcome"], "unknown");
    assert_eq!(recovery["result"]["items"].as_array().unwrap().len(), 1);
    let mut stale = query.clone();
    stale["epoch"] = serde_json::json!("00000000000000000000000000000000");
    assert_eq!(f.exchange(stale)["result"]["kind"], "error");
    f.exchange(serde_json::json!({"v":1,"id":id,"op":"acknowledge","epoch":hello["result"]["epoch"],"target":"cccccccccccccccccccccccccccccccc"}));
    assert!(
        f.exchange(query)["result"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    f.endpoint.set_nonblocking(true).unwrap();
    assert_eq!(
        f.endpoint.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn fence_rpc_renews_admission_without_changing_daemon_session() {
    let f = Fixture::new();
    let id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let hello = f.exchange(serde_json::json!({"v":1,"id":id,"op":"hello"}));
    let query = serde_json::json!({"v":1,"id":id,"op":"fence","epoch":hello["result"]["epoch"],"session":hello["result"]["session"]});
    let first = f.exchange(query.clone());
    let second = f.exchange(query.clone());
    assert_eq!(first["result"]["kind"], "fence");
    assert_ne!(first["result"]["admission"], second["result"]["admission"]);
    assert_eq!(first["result"]["session"], hello["result"]["session"]);
    assert!(first["result"]["items"].as_array().unwrap().is_empty());
    let mut wrong = query;
    wrong["session"] = serde_json::json!("00000000000000000000000000000000");
    assert_eq!(f.exchange(wrong)["result"]["kind"], "error");
    f.endpoint.set_nonblocking(true).unwrap();
    assert_eq!(
        f.endpoint.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
