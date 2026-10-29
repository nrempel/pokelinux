//! Actual UID11045 client and bounded credentialed SEQPACKET peers. No actor/GUI.
use pokelinux_permissions_client::Client;
use rustix::{
    fd::{AsRawFd, FromRawFd, OwnedFd},
    net::{self, AddressFamily, RecvFlags, SendFlags, SocketAddrUnix, SocketFlags, SocketType},
};
use serde_json::{json, Value};
use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};

const OWNER: &str = "11111111111111111111111111111111";
const BUDGET: u64 = 900_000_000;
const CASES: &[&str] = &[
    "confirmed",
    "wrong-target",
    "wrong-permission",
    "extra-field",
    "wrong-pid",
    "lost-set",
    "late-set",
    "focus-before-send",
    "expired-state",
    "expired-queued",
    "original-deadline",
    "choice-during-refresh",
];

fn now() -> u64 {
    let value = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    value.tv_sec as u64 * 1_000_000_000 + value.tv_nsec as u64
}
fn pause() {
    std::thread::sleep(Duration::from_millis(1));
}
fn until(predicate: impl FnMut() -> bool, message: &str) {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut predicate = predicate;
    while !predicate() {
        assert!(Instant::now() < deadline, "{message}");
        pause();
    }
}
fn clock_until(deadline: u64) {
    until(|| now() >= deadline, "original peer deadline");
}
fn gate(path: &Path, name: &str) {
    until(
        || path.with_extension(name).exists(),
        "parent did not release paused client",
    );
}
fn release(path: &Path, name: &str) {
    fs::write(path.with_extension(name), []).unwrap();
}

struct OwnedChild(Child);
impl OwnedChild {
    fn finish(&mut self) {
        let mut status = None;
        until(
            || {
                status = self.0.try_wait().unwrap();
                status.is_some()
            },
            "owned child exit",
        );
        assert!(status.unwrap().success());
        assert!(self.0.wait().unwrap().success());
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().unwrap().is_none() {
            self.0.kill().expect("exact owned child KILL capability");
            until(
                || self.0.try_wait().unwrap().is_some(),
                "owned child KILL wait",
            );
        }
        self.0.wait().unwrap();
    }
}
struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn accept(listener: &OwnedFd) -> OwnedFd {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match net::accept_with(listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC) {
            Ok(fd) => return fd,
            Err(rustix::io::Errno::AGAIN) if Instant::now() < deadline => pause(),
            other => panic!("missing client socket: {other:?}"),
        }
    }
}
fn packet(fd: &OwnedFd) -> Vec<u8> {
    let peer = net::sockopt::socket_peercred(fd).unwrap();
    assert_eq!((peer.uid.as_raw(), peer.gid.as_raw()), (11045, 11045));
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut bytes = [0; 4096];
    loop {
        match net::recv(fd, &mut bytes, RecvFlags::DONTWAIT) {
            Ok((n, _)) => return bytes[..n].to_vec(),
            Err(rustix::io::Errno::AGAIN) if Instant::now() < deadline => pause(),
            other => panic!("client packet: {other:?}"),
        }
    }
}
fn request(fd: &OwnedFd, operation: &str) -> Value {
    let bytes = packet(fd);
    assert!(
        !bytes.is_empty(),
        "request channel closed before {operation}"
    );
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value["op"], operation,
        "an explicit Set must never replay as recovery"
    );
    let deadline = value["deadline_ns"].as_u64().unwrap();
    assert!(now() < deadline && deadline <= now() + BUDGET);
    value
}
fn state(allowed: bool, revision: u64) -> Value {
    json!({"result":"permission_state", "target":"org.omarchy.Sounds", "permission":"media_route",
           "allowed":allowed, "owner":OWNER, "revision":revision})
}
fn send(fd: &OwnedFd, value: Value) {
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(
        net::send(fd, &bytes, SendFlags::NOSIGNAL).unwrap(),
        bytes.len()
    );
}
fn confirmed(client: &mut Client, allowed: bool) {
    until(
        || {
            client.tick();
            client.view().allowed == Some(allowed)
        },
        "saved state not confirmed",
    );
}
fn no_saved_state(client: &mut Client) {
    let view = client.view();
    assert_eq!(view.allowed, None);
    assert!(!view.can_set);
}

fn child(path: &Path, scenario: &str) {
    // SAFETY: this single-threaded harness drops root and supplementary identities before client creation.
    unsafe {
        assert_eq!(libc::setgroups(0, std::ptr::null()), 0);
        assert_eq!(libc::setgid(11045), 0);
        assert_eq!(libc::setuid(11045), 0);
    }
    assert_eq!(rustix::process::getuid().as_raw(), 11045);
    println!("CLIENT_READY_UID11045 {scenario}");
    let mut client = Client::new(path).unwrap();
    client.active(true);
    if matches!(
        scenario,
        "wrong-target" | "wrong-permission" | "extra-field" | "wrong-pid"
    ) {
        client.tick();
        until(
            || {
                client.tick();
                !client.view().pending
            },
            "malformed peer state not refused",
        );
        no_saved_state(&mut client);
        assert!(!client.choose(true));
        return;
    }
    confirmed(&mut client, false);
    assert!(client.view().can_set);
    if scenario == "expired-state" {
        gate(path, "resume"); // No tick or fresh readonly exchange can renew this cached state.
        assert!(!client.choose(true)); // Recheck at click time, before any getter can expire the cache.
        no_saved_state(&mut client);
        return;
    }
    if scenario == "choice-during-refresh" {
        until(
            || {
                client.tick();
                client.view().pending
            },
            "readonly refresh did not start",
        );
        gate(path, "held");
        assert!(
            client.view().can_set,
            "readonly refresh must not disable a fresh explicit choice"
        );
        assert!(client.choose(true));
        assert!(
            !client.choose(false),
            "second choice must not replace pending Set"
        );
        confirmed(&mut client, true);
        client.active(false);
        no_saved_state(&mut client);
        return;
    }
    assert!(client.choose(true));
    assert_eq!(
        client.view().allowed,
        None,
        "choice must not optimistically grant"
    );
    match scenario {
        "focus-before-send" => {
            client.active(false);
            client.tick();
            no_saved_state(&mut client);
            assert!(!client.view().pending && !client.view().unconfirmed);
            assert!(!client.choose(false));
        }
        "expired-queued" => {
            gate(path, "resume"); // A queued Set cannot outlive the original confirmed-state cutoff.
            client.tick();
            no_saved_state(&mut client);
            assert!(!client.view().pending && !client.view().unconfirmed);
        }
        "original-deadline" => {
            gate(path, "resume"); // Delay first send, without replacing the accepted choice's deadline.
            confirmed(&mut client, true);
        }
        "late-set" => {
            client.tick();
            gate(path, "resume"); // Peer queues a valid late reply while client ticks pause.
            no_saved_state(&mut client); // Getter itself expires the sent operation.
            assert!(client.view().unconfirmed);
            client.tick();
            no_saved_state(&mut client);
        }
        "lost-set" => {
            client.tick();
            until(
                || {
                    client.tick();
                    client.view().unconfirmed
                },
                "lost Set must remain unconfirmed",
            );
            no_saved_state(&mut client);
            client.active(false);
            client.active(true);
            confirmed(&mut client, true); // Readonly recovery may report that the lost Set committed.
            assert!(
                client.view().unconfirmed,
                "readonly Status cannot confirm the old explicit choice"
            );
            until(
                || {
                    client.tick();
                    path.with_extension("verified").exists()
                },
                "readonly recovery refresh",
            );
            assert!(client.view().unconfirmed);
        }
        "confirmed" => confirmed(&mut client, true),
        _ => panic!("unknown scenario"),
    }
    client.active(false);
    no_saved_state(&mut client);
    assert!(!client.choose(false));
}

fn run(root: &Path, scenario: &str) -> io::Result<()> {
    let path = root.join(format!("{scenario}.sock"));
    let listener = net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
        None,
    )?;
    net::bind(&listener, &SocketAddrUnix::new(&path)?)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o666))?;
    net::listen(&listener, 4)?;
    let mut process = OwnedChild(
        Command::new(std::env::current_exe()?)
            .arg("--client")
            .arg(&path)
            .arg(scenario)
            .spawn()?,
    );
    let fd = accept(&listener);
    let status = request(&fd, "permission_status");
    let original_state_until = status["deadline_ns"].as_u64().unwrap();
    match scenario {
        "wrong-target" | "wrong-permission" | "extra-field" => {
            let mut value = state(false, 7);
            match scenario {
                "wrong-target" => value["target"] = json!("org.omarchy.Messages"),
                "wrong-permission" => value["permission"] = json!("capture"),
                _ => value["uid"] = json!(0),
            }
            send(&fd, value);
        }
        "wrong-pid" => {
            // This parent remains the accepted root peer; a real different PID sends the packet.
            rustix::io::fcntl_setfd(&fd, rustix::io::FdFlags::empty())?;
            let mut sender = OwnedChild(
                Command::new(std::env::current_exe()?)
                    .arg("--reply")
                    .arg(fd.as_raw_fd().to_string())
                    .spawn()?,
            );
            rustix::io::fcntl_setfd(&fd, rustix::io::FdFlags::CLOEXEC)?;
            sender.finish();
        }
        "choice-during-refresh" => {
            send(&fd, state(false, 7));
            drop(fd);
            let refresh = accept(&listener);
            request(&refresh, "permission_status");
            release(&path, "held");
            let setting = accept(&listener);
            let set = request(&setting, "permission_set");
            assert_eq!(set["allowed"], true);
            assert_eq!(set["expected_revision"], 7);
            assert_eq!(set["expected_owner"], OWNER);
            assert!(
                packet(&refresh).is_empty(),
                "superseded readonly socket remains open"
            );
            send(&setting, state(true, 8));
        }
        "expired-state" => {
            send(&fd, state(false, 7));
            clock_until(original_state_until + 1);
            release(&path, "resume");
        }
        _ => {
            if scenario == "expired-queued" {
                clock_until(original_state_until.saturating_sub(600_000_000));
            }
            send(&fd, state(false, 7));
            drop(fd);
            let fd = accept(&listener);
            match scenario {
                "focus-before-send" => assert!(packet(&fd).is_empty(), "focus loss emitted a Set"),
                "expired-queued" => {
                    clock_until(original_state_until + 1);
                    release(&path, "resume");
                    assert!(
                        packet(&fd).is_empty(),
                        "queued Set renewed expired cached authority"
                    );
                }
                _ => {
                    let queued_at = now();
                    if scenario == "original-deadline" {
                        clock_until(queued_at + 250_000_000);
                        release(&path, "resume");
                    }
                    let set = request(&fd, "permission_set");
                    assert_eq!(set["allowed"], true);
                    assert_eq!(set["expected_revision"], 7);
                    assert_eq!(set["expected_owner"], OWNER);
                    assert!(
                        set["deadline_ns"].as_u64().unwrap() <= queued_at + BUDGET,
                        "first send renewed the queued choice's original deadline"
                    );
                    match scenario {
                        "lost-set" => {
                            drop(fd);
                            let recovery = accept(&listener);
                            request(&recovery, "permission_status");
                            send(&recovery, state(true, 8));
                            drop(recovery);
                            let refresh = accept(&listener);
                            request(&refresh, "permission_status");
                            send(&refresh, state(true, 8));
                            release(&path, "verified");
                        }
                        "late-set" => {
                            clock_until(set["deadline_ns"].as_u64().unwrap() + 1);
                            send(&fd, state(true, 8));
                            release(&path, "resume");
                        }
                        _ => send(&fd, state(true, 8)),
                    }
                }
            }
        }
    }
    process.finish();
    assert!(
        matches!(
            net::accept_with(&listener, SocketFlags::NONBLOCK | SocketFlags::CLOEXEC),
            Err(rustix::io::Errno::AGAIN)
        ),
        "unexpected extra connection or automatic Set replay"
    );
    println!("PASS actual UID11045 {scenario}");
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() == 4 && args[1] == "--client" {
        child(Path::new(&args[2]), args[3].to_str().unwrap());
        return Ok(());
    }
    if args.len() == 3 && args[1] == "--reply" {
        // SAFETY: the parent transferred this sole inherited descriptor to this helper.
        let fd = unsafe { OwnedFd::from_raw_fd(args[2].to_str().unwrap().parse().unwrap()) };
        send(&fd, state(false, 7));
        return Ok(());
    }
    assert_eq!(
        rustix::process::getuid().as_raw(),
        0,
        "isolated root required"
    );
    let root = Root(PathBuf::from(format!(
        "/run/permissions-client-{}",
        std::process::id()
    )));
    fs::create_dir(&root.0)?;
    fs::set_permissions(&root.0, fs::Permissions::from_mode(0o755))?;
    assert!(
        Client::new(&root.0.join("wrong-uid.sock")).is_err(),
        "root is not the installed controller identity"
    );
    for scenario in CASES {
        run(&root.0, scenario)?;
    }
    Ok(())
}
