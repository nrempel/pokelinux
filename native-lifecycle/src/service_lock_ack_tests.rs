//! Component regression: a real owned storage replacement completes between
//! queueing a typed lock Ok and checking its publication. No full actor, EAGAIN,
//! authentication, clear, Source, PAM, guest or phone execution is claimed.
use super::{Connection, revalidate_lock_ack};
use crate::{
    Permission, Reply,
    authority::{Authority, Decision},
    boottime_ns, fresh_id, ipc,
    storage::{Durable, Event, Grant, Worker},
};
use rustix::net::{self, AddressFamily, SocketFlags, SocketType};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

struct Installed {
    directory: PathBuf,
    executable: PathBuf,
}

impl Installed {
    fn new() -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let root = std::env::var_os("NATIVE_LIFECYCLE_TEST_ROOT")
            .expect("root-owned isolated kernel runner");
        let id = fresh_id().unwrap();
        let directory = PathBuf::from(root).join(format!("lock-ack-{id}"));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let executable = PathBuf::from("/opt").join(format!("lifecycle-lock-ack-helper-{id}"));
        let source = std::env::var_os("NATIVE_LIFECYCLE_STORAGE_HELPER")
            .expect("exact recorded daemon helper");
        fs::copy(source, &executable).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        Self {
            directory,
            executable,
        }
    }
}

impl Drop for Installed {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.executable);
        let _ = fs::remove_dir_all(&self.directory);
    }
}

// Preserve actual child custody even if setup, IPC or an intermediate assertion
// fails. Worker::drain aborts rather than detaching an unreaped child.
struct OwnedWorker(Worker);
impl Drop for OwnedWorker {
    fn drop(&mut self) {
        self.0.drain();
    }
}

fn completion(worker: &mut Worker, token: u64, until: u64) -> Option<Durable> {
    loop {
        match worker.poll().unwrap() {
            Some(Event::Complete {
                token: returned,
                success: true,
                state,
            }) if returned == token => return state,
            None if boottime_ns() < until => {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            _ => panic!("exact owned storage request failed or expired"),
        }
    }
}

#[test]
fn queued_lock_ok_is_revoked_after_complete_intervening_storage_write() {
    let installed = Installed::new();
    let initial = Durable {
        grants: vec![Grant {
            app: "one".into(),
            permission: Permission::Capture,
            allowed: true,
        }],
        receipts: vec![],
        lock_pending: true,
    };
    let policy = installed.directory.join("policy.json");
    fs::write(&policy, serde_json::to_vec(&initial).unwrap()).unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
    let mut owner = OwnedWorker(
        Worker::spawn(
            &installed.directory,
            &installed.directory,
            &installed.executable,
            None,
        )
        .unwrap(),
    );
    let until = boottime_ns().saturating_add(1_000_000_000);
    let load = owner.0.start_load(vec!["one".into()], until).unwrap();
    let loaded = completion(&mut owner.0, load, until).expect("actual initial storage load");
    let mut authority = Authority::loading(vec![]).unwrap();
    authority.activate(loaded);
    assert!(authority.ready() && owner.0.occupied() && !owner.0.busy());
    let reply = match authority.plan_lock(boottime_ns()) {
        Decision::Immediate(reply) => reply,
        Decision::Persist(_) => panic!("seeded durable pending lock must already be admitted"),
    };
    assert_eq!(reply, Reply::Ok);
    let (fd, peer) = net::socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    net::sockopt::set_socket_passcred(&peer, true).unwrap();
    let mut connection = Connection {
        id: 1,
        awaiting: None,
        fd,
        admin: false,
        desktop: None,
        lock: true,
        reading: None,
        read_stamp: None,
        deadline: boottime_ns().saturating_add(1_000_000_000),
        pending: Some(serde_json::to_vec(&reply).unwrap()),
        lock_ack: Some(owner.0.admission_sequence()),
        launching: None,
    };
    // One sole real Worker completes a normal policy replacement. The guard is
    // deliberately not called while it is busy, so a boolean busy check misses
    // the entire interval. There is no clear or authored completion event.
    let mut next = initial;
    next.grants[0].allowed = false;
    let until = boottime_ns().saturating_add(1_000_000_000);
    let replace = owner.0.start_write(next, until).unwrap();
    assert!(owner.0.busy());
    assert!(completion(&mut owner.0, replace, until).is_none());
    assert!(authority.ready() && owner.0.occupied() && !owner.0.busy());
    let on_disk: Durable = serde_json::from_slice(&fs::read(policy).unwrap()).unwrap();
    assert!(on_disk.lock_pending && !on_disk.grants[0].allowed);
    let operation = None;
    revalidate_lock_ack(&mut connection, false, &authority, &operation, &owner.0);
    ipc::send(&connection.fd, connection.pending.as_ref().unwrap()).unwrap();
    let (packet, _) = ipc::receive(&peer)
        .unwrap()
        .expect("actual queued reply packet");
    let received: Reply = serde_json::from_slice(&packet).unwrap();
    let still_marked = connection.lock_ack;
    // Reap before the expected RED assertion: failure must remain an ordinary
    // assertion failure, never an occupied Worker Drop abort or detached child.
    owner.0.drain();
    assert!(!owner.0.occupied());
    assert_eq!(received, Reply::Unavailable);
    assert!(still_marked.is_none());
}
