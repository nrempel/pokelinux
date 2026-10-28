//! The normal service must expose history without a second policy actor.
//! The desired wire and data assertions are identical for the selected old/new CLI.
use pokelinux_lifecycle::{boottime_ns, fresh_id, ipc};
use pokelinux_messages::{
    projection::Reader,
    protocol::{Page, Query},
};
use pokelinux_messaging::receipt::{BackendScope, IncomingKind, ReceiptOwner, Slot};
use rustix::{
    fd::OwnedFd,
    net::{UCred, sockopt::socket_peercred},
};
use serde_json::{Value, json};
use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::Duration,
};

fn tick() {
    std::thread::sleep(Duration::from_millis(2));
}
fn require(value: bool, why: &'static str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}
fn call(fd: &OwnedFd, request: Value, until: u64) -> io::Result<Value> {
    let expected = socket_peercred(fd)?;
    require(
        expected.uid.as_raw() == 0 && expected.gid.as_raw() == 0,
        "root daemon peer required",
    )?;
    let _alive = pokelinux_kernel_identity::socket_peer_pidfd(fd)?;
    let bytes = serde_json::to_vec(&request)?;
    loop {
        require(boottime_ns() < until, "original send deadline")?;
        match ipc::send(fd, &bytes) {
            Ok(()) => break,
            Err(e) if matches!(e.raw_os_error(), Some(4 | 11)) => tick(),
            Err(e) => return Err(e),
        }
    }
    receive(fd, expected, until)
}
fn receive(fd: &OwnedFd, expected: UCred, until: u64) -> io::Result<Value> {
    loop {
        require(boottime_ns() < until, "original response deadline")?;
        match ipc::receive(fd) {
            Ok(Some((bytes, cred))) => {
                require(
                    cred.pid == expected.pid
                        && cred.uid == expected.uid
                        && cred.gid == expected.gid,
                    "exact SCM daemon peer",
                )?;
                let response = serde_json::from_slice(&bytes)?;
                require(
                    boottime_ns() < until,
                    "original deadline includes response decoding",
                )?;
                return Ok(response);
            }
            Ok(None) => tick(),
            Err(e) if e.raw_os_error() == Some(4) => {}
            Err(e) => return Err(e),
        }
    }
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> io::Result<Self> {
        let root = PathBuf::from("/run").join(format!("messages-private-provider-{}", fresh_id()?));
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        Ok(Self(root))
    }
    fn child(&self, name: &str) -> io::Result<PathBuf> {
        let path = self.0.join(name);
        fs::create_dir(&path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
        Ok(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct OwnedChild {
    child: Child,
    status: Option<ExitStatus>,
    close: u64,
}
impl OwnedChild {
    fn spawn(command: &mut Command, close: u64) -> io::Result<Self> {
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self {
            child,
            status: None,
            close,
        })
    }
    fn living(&mut self) -> io::Result<bool> {
        if self.status.is_none() {
            self.status = self.child.try_wait()?;
        }
        Ok(self.status.is_none())
    }
    fn wait(&mut self) -> io::Result<ExitStatus> {
        while self.living()? {
            require(boottime_ns() < self.close, "original child wait deadline")?;
            tick();
        }
        require(boottime_ns() < self.close, "late actual child wait")?;
        Ok(self.status.unwrap())
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.status.is_none() {
            let _ = self.child.kill();
            let _ = self.wait();
        }
        if self.status.is_none() {
            std::process::abort();
        }
    }
}
fn seed(raw: &Path) -> io::Result<Vec<String>> {
    let mut raw = ReceiptOwner::open(raw).map_err(|_| io::ErrorKind::InvalidData)?;
    let source = raw
        .attach(Slot(0), BackendScope::Unqualified)
        .map_err(|_| io::ErrorKind::InvalidData)?;
    let hex = "000008915155100000006201213141510005e8329bfd06";
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|n| u8::from_str_radix(&hex[n..n + 2], 16).unwrap())
        .collect();
    (0..2)
        .map(|_| {
            raw.admit(source, IncomingKind::Deliver, &bytes)
                .map(|id| id.to_string())
                .map_err(|_| io::ErrorKind::InvalidData.into())
        })
        .collect()
}
fn exact_query(inbox: &Path, expected: &[String]) -> io::Result<Query> {
    let mut reader = Reader::open(inbox).map_err(|_| io::ErrorKind::InvalidData)?;
    let Page::Conversations { items } = reader
        .query(
            &Query::Conversations {
                after: None,
                limit: 8,
            },
            2,
        )
        .map_err(|_| io::ErrorKind::InvalidData)?
    else {
        return Err(io::ErrorKind::InvalidData.into());
    };
    require(items.len() == 1, "one independently projected conversation")?;
    let query = Query::Messages {
        conversation: items[0].id.clone(),
        after_ordinal: 0,
        limit: 2,
    };
    let page = reader
        .query(&query, 2)
        .map_err(|_| io::ErrorKind::InvalidData)?;
    require(
        exact_page(page, expected),
        "exact independent authored IDs and text",
    )?;
    Ok(query)
}
fn exact_page(page: Page, expected: &[String]) -> bool {
    matches!(page, Page::Messages { items } if items.len() == 2 && items.iter().zip(expected).all(|(m,id)| m.receipt.as_str() == id && m.text == "hello"))
}
fn ready(fd: &OwnedFd, until: u64) -> io::Result<Value> {
    let peer = socket_peercred(fd)?;
    let mut state = call(fd, json!({"op":"open","version":1}), until)?;
    loop {
        require(
            state["event"] == "state" && state["version"] == 1,
            "strict private State response",
        )?;
        if state["available"] == true {
            return Ok(state);
        }
        state = receive(fd, peer, until)?;
    }
}
fn private_page(runtime: &Path, query: &Query, expected: &[String], work: u64) -> io::Result<()> {
    // This is the actual missing-production boundary in the old normal daemon.
    let fd = ipc::connect(&runtime.join("read-provider.sock"))?;
    let state = ready(&fd, work)?;
    let generation = state["generation"]
        .as_str()
        .ok_or(io::ErrorKind::InvalidData)?;
    pokelinux_messages::protocol::DataId::parse(generation)
        .map_err(|_| io::ErrorKind::InvalidData)?;
    let until = work.min(boottime_ns() + 1_000_000_000);
    let response = call(
        &fd,
        json!({"op":"read","version":1,"generation":generation,"token":1,"until_ns":until,"query":query}),
        until,
    )?;
    require(
        response["event"] == "complete"
            && response["version"] == 1
            && response["generation"] == generation
            && response["token"] == 1
            && response["until_ns"] == until,
        "exact original-deadline private Complete",
    )?;
    require(
        exact_page(serde_json::from_value(response["page"].clone())?, expected),
        "exact private history page",
    )?;
    drop(fd);
    // Closing a session connection must preserve background history ownership.
    let next = ipc::connect(&runtime.join("read-provider.sock"))?;
    let next_state = ready(&next, work)?;
    require(
        next_state["generation"] == generation,
        "session close must retain daemon reader generation",
    )?;
    Ok(())
}
#[test]
#[ignore = "isolated Linux root; normal production daemon and real Projector children"]
fn normal_daemon_serves_private_history_without_session_database_ownership() {
    let start = boottime_ns();
    let work = start + 5_000_000_000;
    let close = start + 8_000_000_000;
    let directory = Directory::new().unwrap();
    let runtime = directory.child("runtime").unwrap();
    let raw = directory.child("raw").unwrap();
    let inbox = directory.child("inbox").unwrap();
    let policy = directory.child("policy").unwrap();
    let profile = directory.0.join("profile.json");
    fs::write(&profile, serde_json::to_vec(&json!({"apps":[{"app":"org.omarchy.Messages","uid":11010,"executable":"/usr/bin/true","args":[]}]})).unwrap()).unwrap();
    let expected = seed(&raw).unwrap();
    assert_ne!(expected[0], expected[1]);
    // Only argv differs for the explicitly selected historical/current artifact.
    // The private socket, response, page and persistence requirements do not.
    let legacy = match std::env::var("POKELINUX_MESSAGES_TEST_LAYOUT").as_deref() {
        Ok("legacy-policy-actor") => true,
        Ok("private-history-owner") => false,
        _ => panic!("explicit normal build layout required"),
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_messages-service"));
    if legacy {
        command.args([
            &runtime,
            &policy,
            &profile,
            Path::new("/usr/bin/true"),
            &raw,
            &inbox,
        ]);
    } else {
        command.args([&runtime, &raw, &inbox]);
    }
    let mut daemon = OwnedChild::spawn(&mut command, close).unwrap();
    let observation = (|| -> io::Result<()> {
        if legacy {
            loop {
                require(
                    daemon.living()? && boottime_ns() < work,
                    "normal history daemon readiness",
                )?;
                if runtime.join("admin.sock").exists() {
                    let fd = ipc::connect(&runtime.join("admin.sock"))?;
                    let state = call(&fd, json!({"op":"messages_status"}), work)?;
                    require(
                        state["result"] == "messages_storage",
                        "actual normal history status",
                    )?;
                    if state["available"] == true && state["busy"] == false {
                        break;
                    }
                }
                tick();
            }
        } else {
            while !runtime.join("read-provider.sock").exists() {
                require(
                    daemon.living()? && boottime_ns() < work,
                    "normal private daemon startup",
                )?;
                tick();
            }
            let fd = ipc::connect(&runtime.join("read-provider.sock"))?;
            let _ = ready(&fd, work)?;
        }
        let query = exact_query(&inbox, &expected)?;
        println!(
            "PRIVATE_PROVIDER_PREREQUISITES normal_daemon=true exact_independent_history=true"
        );
        private_page(&runtime, &query, &expected, work)?;
        require(
            daemon.living()?,
            "normal daemon survives session connection closure",
        )
    })();
    let mut stop = OwnedChild::spawn(
        Command::new(env!("CARGO_BIN_EXE_messages-stop")).arg(runtime.join("admin.sock")),
        close,
    )
    .unwrap();
    let stop_status = stop.wait().unwrap();
    let daemon_status = daemon.wait().unwrap();
    println!(
        "PRIVATE_PROVIDER_CLOSED daemon_wait={} stop_wait={} before_original_close={} observation_error={:?}",
        daemon_status.success(),
        stop_status.success(),
        boottime_ns() < close,
        observation.as_ref().err().map(ToString::to_string)
    );
    assert!(
        stop_status.success() && daemon_status.success() && boottime_ns() < close,
        "normal typed Stop and actual wait must succeed before assertion"
    );
    observation.expect("NORMAL_PRIVATE_PROVIDER_VALID_PAGE_REQUIRED");
}
