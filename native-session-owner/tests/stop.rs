//! Root-only real native-client and SEQPACKET/SCM credential wiring tests.
use pokelinux_lifecycle::{AdminRequest, Reply, boottime_ns, ipc};
use rustix::{fd::OwnedFd, net};
use std::{
    fs,
    io::{self, Read},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
static NEXT: AtomicU32 = AtomicU32::new(0);
fn tick() {
    std::thread::sleep(Duration::from_millis(2));
}
struct Case {
    root: PathBuf,
    runtime: PathBuf,
    listener: OwnedFd,
}
impl Case {
    fn new() -> Self {
        assert_eq!(rustix::process::getuid().as_raw(), 0);
        let root = PathBuf::from(format!(
            "/run/session-stop-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        let runtime = root.join("runtime");
        fs::create_dir(&runtime).unwrap();
        let listener = ipc::listener(&runtime.join("admin.sock"), 0o666).unwrap();
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        Self {
            root,
            runtime,
            listener,
        }
    }
    fn accept(&self, child: &mut OwnedChild) -> Option<OwnedFd> {
        while boottime_ns() < child.until {
            match net::accept_with(
                &self.listener,
                net::SocketFlags::NONBLOCK | net::SocketFlags::CLOEXEC,
            ) {
                Ok(fd) => return Some(fd),
                Err(rustix::io::Errno::AGAIN) => {}
                Err(e) => panic!("accept: {e}"),
            }
            if child.poll().is_some() {
                return None;
            }
            tick();
        }
        None
    }
    fn no_connection(&self) {
        assert!(matches!(
            net::accept_with(
                &self.listener,
                net::SocketFlags::NONBLOCK | net::SocketFlags::CLOEXEC
            ),
            Err(rustix::io::Errno::AGAIN)
        ));
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
struct OwnedChild {
    child: Child,
    started: u64,
    until: u64,
    status: Option<ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    out_eof: bool,
    err_eof: bool,
    reported: bool,
    pipes_ready: bool,
}
impl OwnedChild {
    fn spawn(mut cmd: Command) -> Self {
        let started = boottime_ns();
        let child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // Retain custody before any fallible post-spawn descriptor setup.
        let mut owned = Self {
            child,
            started,
            until: started + 4_000_000_000,
            status: None,
            stdout: vec![],
            stderr: vec![],
            out_eof: false,
            err_eof: false,
            reported: false,
            pipes_ready: false,
        };
        for fd in [
            owned.child.stdout.as_ref().unwrap() as &dyn std::os::fd::AsFd,
            owned.child.stderr.as_ref().unwrap() as &dyn std::os::fd::AsFd,
        ] {
            let flags = rustix::fs::fcntl_getfl(fd).unwrap();
            rustix::fs::fcntl_setfl(fd, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
        }
        owned.pipes_ready = true;
        owned
    }
    fn pump(&mut self) {
        let mut data = [0; 1024];
        loop {
            match self.child.stdout.as_mut().unwrap().read(&mut data) {
                Ok(0) => {
                    self.out_eof = true;
                    break;
                }
                Ok(n) => {
                    self.stdout.extend_from_slice(&data[..n]);
                    assert!(self.stdout.len() <= 4096)
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("{e}"),
            }
        }
        loop {
            match self.child.stderr.as_mut().unwrap().read(&mut data) {
                Ok(0) => {
                    self.err_eof = true;
                    break;
                }
                Ok(n) => {
                    self.stderr.extend_from_slice(&data[..n]);
                    assert!(self.stderr.len() <= 4096)
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("{e}"),
            }
        }
    }
    fn poll(&mut self) -> Option<ExitStatus> {
        self.pump();
        if self.status.is_none() {
            self.status = self.child.try_wait().unwrap();
        }
        self.status
    }
    fn finish(&mut self) -> ExitStatus {
        while self.poll().is_none() {
            assert!(boottime_ns() < self.until, "original child deadline");
            tick();
        }
        self.pump();
        assert!(self.out_eof && self.err_eof);
        let status = self.status.unwrap();
        if !self.reported {
            println!(
                "{}",
                serde_json::json!({"owned_child_actual_wait":true,"pid":self.child.id(),"code":status.code(),"stdout_EOF":self.out_eof,"stderr_EOF":self.err_eof,"elapsed_ns":boottime_ns()-self.started,"stderr":String::from_utf8_lossy(&self.stderr)})
            );
            self.reported = true;
        }
        status
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.pipes_ready {
            // Never read pipes whose NONBLOCK setup may have failed. Closing
            // them does not establish EOF; the child still requires an owned wait.
            self.child.stdin.take();
            self.child.stdout.take();
            self.child.stderr.take();
            let _ = self.child.kill();
            let status = self.child.wait().unwrap();
            self.status = Some(status);
            println!(
                "{}",
                serde_json::json!({"owned_child_actual_wait":true,"partial_setup":true,"pid":self.child.id(),"code":status.code(),"stdout_EOF":false,"stderr_EOF":false,"stdout_closed_without_EOF":true,"stderr_closed_without_EOF":true})
            );
            return;
        }
        if self.status.is_none() {
            if self.child.try_wait().unwrap().is_none() {
                let _ = self.child.kill();
            }
            self.status = Some(self.child.wait().unwrap());
        }
        self.pump();
        assert!(self.out_eof && self.err_eof, "child PIPEs did not close");
        if !self.reported {
            println!(
                "{}",
                serde_json::json!({"owned_child_actual_wait":true,"cleanup":true,"pid":self.child.id(),"code":self.status.unwrap().code(),"stdout_EOF":self.out_eof,"stderr_EOF":self.err_eof,"stderr":String::from_utf8_lossy(&self.stderr)})
            );
        }
    }
}
fn native() -> Command {
    Command::new(std::env::var_os("SESSION_OWNER_NATIVE").expect("installed exact native Main"))
}
fn helper() -> Command {
    Command::new(std::env::var_os("SESSION_STOP_HELPER").expect("installed fixture helper"))
}
fn stop_request(fd: &OwnedFd, child: &mut OwnedChild) {
    while boottime_ns() < child.until {
        match ipc::receive(fd).unwrap() {
            Some((bytes, cred)) => {
                assert_eq!(cred.uid.as_raw(), 0);
                assert_eq!(cred.gid.as_raw(), 0);
                assert_eq!(cred.pid.as_raw_pid() as u32, child.child.id());
                assert!(matches!(
                    serde_json::from_slice::<AdminRequest>(&bytes).unwrap(),
                    AdminRequest::Stop {}
                ));
                assert_eq!(bytes, serde_json::to_vec(&AdminRequest::Stop {}).unwrap());
                return;
            }
            None => {
                assert!(child.poll().is_none(), "client died before Stop");
                tick();
            }
        }
    }
    panic!("no typed Stop before original child deadline");
}
fn only_request_then_closed(fd: &OwnedFd) {
    assert_eq!(
        ipc::receive(fd).unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
}
fn run_replied(mut command: Command) {
    let case = Case::new();
    command.arg(&case.runtime);
    let mut child = OwnedChild::spawn(command);
    let connection = case.accept(&mut child);
    if connection.is_none() {
        let _ = child.finish();
    }
    let fd =
        connection.expect("native stop must connect using runtime directory with no configuration");
    stop_request(&fd, &mut child);
    ipc::send(&fd, &serde_json::to_vec(&Reply::Ok).unwrap()).unwrap();
    assert!(child.finish().success());
    only_request_then_closed(&fd);
}
#[test]
#[ignore = "isolated root native/fixture binaries and actual credentials required"]
fn stop_route_control_typed_ipc_setup() {
    let mut cmd = helper();
    cmd.arg("--control");
    run_replied(cmd);
}
#[test]
#[ignore = "isolated root native/fixture binaries and actual credentials required"]
fn stop_route_works_without_configuration() {
    let mut cmd = native();
    cmd.arg("--stop");
    run_replied(cmd);
}
#[test]
#[ignore = "isolated root native/fixture binaries and actual credentials required"]
fn strict_stop_arguments_never_connect() {
    let case = Case::new();
    for args in [
        vec![],
        vec!["--stop".into()],
        vec!["--stop".into(), "relative".into()],
        vec![
            "--stop".into(),
            case.runtime.to_string_lossy().into_owned(),
            "extra".into(),
        ],
        vec![
            "--grant".into(),
            case.runtime.to_string_lossy().into_owned(),
        ],
    ] {
        let mut cmd = native();
        cmd.args(args);
        let mut child = OwnedChild::spawn(cmd);
        assert!(!child.finish().success());
        case.no_connection();
    }
}
#[test]
#[ignore = "isolated root native/fixture binaries and actual credentials required"]
fn nonroot_stop_caller_is_refused_before_connect() {
    let case = Case::new();
    let mut cmd = native();
    cmd.arg("--stop").arg(&case.runtime).gid(11042).uid(11042);
    let mut child = OwnedChild::spawn(cmd);
    assert!(!child.finish().success());
    case.no_connection();
}
#[test]
#[ignore = "isolated root native/fixture binaries and actual credentials required"]
fn actual_nonroot_reply_is_not_accepted() {
    let case = Case::new();
    let mut cmd = native();
    cmd.arg("--stop").arg(&case.runtime);
    let mut child = OwnedChild::spawn(cmd);
    let fd = case.accept(&mut child).expect("client connect");
    stop_request(&fd, &mut child);
    let watch = rustix::io::fcntl_dupfd_cloexec(&fd, 3).unwrap();
    let mut reply = helper();
    reply
        .arg("--reply-ok")
        .gid(11042)
        .uid(11042)
        .stdin(Stdio::from(fd));
    let mut sender = OwnedChild::spawn(reply);
    assert!(sender.finish().success());
    assert!(!child.finish().success());
    only_request_then_closed(&watch);
}
#[test]
#[ignore = "isolated root native/fixture binaries and actual credentials required"]
fn withheld_reply_uses_existing_original_ipc_cutoff() {
    let case = Case::new();
    let mut cmd = native();
    cmd.arg("--stop").arg(&case.runtime);
    let mut child = OwnedChild::spawn(cmd);
    let fd = case.accept(&mut child).expect("client connect");
    stop_request(&fd, &mut child);
    assert!(!child.finish().success());
    let elapsed = boottime_ns() - child.started;
    assert!((1_900_000_000..3_000_000_000).contains(&elapsed));
    only_request_then_closed(&fd);
}
