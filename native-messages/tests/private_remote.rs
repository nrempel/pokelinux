//! Actual normal daemon/Remote composition. No authored storage or policy peer.
use pokelinux_lifecycle::{
    boottime_ns, fresh_id,
    messages::{Event, Provider},
};
use pokelinux_messages::{
    projection::Reader,
    protocol::{Page, Query},
    read_service::Remote,
};
use pokelinux_messaging::receipt::{BackendScope, IncomingKind, ReceiptOwner, Slot};
use rustix::{
    fd::OwnedFd,
    process::{Pid, PidfdFlags, Signal},
};
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
fn require(v: bool, why: &'static str) -> io::Result<()> {
    if v {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}
fn living(fd: &OwnedFd) -> bool {
    pokelinux_lifecycle::authority::living(fd).unwrap_or(false)
}
struct Process {
    child: Child,
    status: Option<ExitStatus>,
    close: u64,
}
impl Process {
    fn new(command: &mut Command, close: u64) -> io::Result<Self> {
        Ok(Self {
            child: command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
            status: None,
            close,
        })
    }
    fn wait(&mut self) -> io::Result<ExitStatus> {
        loop {
            if self.status.is_none() {
                self.status = self.child.try_wait()?;
            }
            require(boottime_ns() < self.close, "original child cleanup bound")?;
            if let Some(status) = self.status {
                return Ok(status);
            }
            tick();
        }
    }
}
impl Drop for Process {
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
struct Role {
    pid: Pid,
    alive: OwnedFd,
}
struct Case {
    root: PathBuf,
    runtime: PathBuf,
    raw: PathBuf,
    inbox: PathBuf,
    daemon: Process,
    expected: Vec<String>,
    work: u64,
    close: u64,
}
impl Case {
    fn new() -> io::Result<Self> {
        let start = boottime_ns();
        let work = start + 5_000_000_000;
        let close = start + 8_000_000_000;
        let root = PathBuf::from("/run").join(format!("messages-remote-{}", fresh_id()?));
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let child = |name: &str| -> io::Result<PathBuf> {
            let p = root.join(name);
            fs::create_dir(&p)?;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o700))?;
            Ok(p)
        };
        let runtime = child("runtime")?;
        let raw = child("raw")?;
        let inbox = child("inbox")?;
        let expected = {
            let mut owner = ReceiptOwner::open(&raw).map_err(|_| io::ErrorKind::InvalidData)?;
            let source = owner
                .attach(Slot(0), BackendScope::Unqualified)
                .map_err(|_| io::ErrorKind::InvalidData)?;
            let h = "000008915155100000006201213141510005e8329bfd06";
            let bytes: Vec<_> = (0..h.len())
                .step_by(2)
                .map(|n| u8::from_str_radix(&h[n..n + 2], 16).unwrap())
                .collect();
            (0..2)
                .map(|_| {
                    owner
                        .admit(source, IncomingKind::Deliver, &bytes)
                        .map(|id| id.to_string())
                        .map_err(|_| io::ErrorKind::InvalidData.into())
                })
                .collect::<io::Result<Vec<_>>>()?
        };
        let daemon = Self::spawn(&runtime, &raw, &inbox, close)?;
        Ok(Self {
            root,
            runtime,
            raw,
            inbox,
            daemon,
            expected,
            work,
            close,
        })
    }
    fn spawn(runtime: &Path, raw: &Path, inbox: &Path, close: u64) -> io::Result<Process> {
        Process::new(
            Command::new(env!("CARGO_BIN_EXE_messages-service")).args([runtime, raw, inbox]),
            close,
        )
    }
    fn connect(&self) -> io::Result<Remote> {
        while !self.runtime.join("read-provider.sock").exists() {
            require(boottime_ns() < self.work, "original daemon startup bound")?;
            tick();
        }
        let mut remote = Remote::connect(&self.runtime.join("read-provider.sock"))?;
        self.ready(&mut remote)?;
        Ok(remote)
    }
    fn ready(&self, remote: &mut Remote) -> io::Result<()> {
        while !remote.available() {
            require(boottime_ns() < self.work, "original Remote readiness bound")?;
            let _ = remote.poll()?;
            tick();
        }
        Ok(())
    }
    fn roles(&self) -> io::Result<(Role, Role)> {
        let children = fs::read_to_string(format!(
            "/proc/{}/task/{}/children",
            self.daemon.child.id(),
            self.daemon.child.id()
        ))?;
        let mut writer = None;
        let mut reader = None;
        for p in children.split_whitespace() {
            let raw: i32 = p.parse().map_err(|_| io::ErrorKind::InvalidData)?;
            let pid = Pid::from_raw(raw).ok_or(io::ErrorKind::InvalidData)?;
            let command = fs::read(format!("/proc/{raw}/cmdline"))?;
            let role = Role {
                pid,
                alive: rustix::process::pidfd_open(pid, PidfdFlags::NONBLOCK)?,
            };
            if command
                .split(|b| *b == 0)
                .any(|a| a == b"--messages-worker")
            {
                writer = Some(role);
            } else if command
                .split(|b| *b == 0)
                .any(|a| a == b"--messages-read-worker")
            {
                reader = Some(role);
            }
        }
        Ok((
            writer.ok_or(io::ErrorKind::NotFound)?,
            reader.ok_or(io::ErrorKind::NotFound)?,
        ))
    }
    fn query(&self) -> io::Result<Query> {
        let mut reader = Reader::open(&self.inbox).map_err(|_| io::ErrorKind::InvalidData)?;
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
        require(
            items.len() == 1,
            "actual independently projected conversation",
        )?;
        Ok(Query::Messages {
            conversation: items[0].id.clone(),
            after_ordinal: 0,
            limit: 2,
        })
    }
    fn read(&self, remote: &mut Remote) -> io::Result<()> {
        let until = self.work.min(boottime_ns() + 1_000_000_000);
        let token = remote.start(serde_json::to_value(self.query()?)?, until)?;
        loop {
            require(boottime_ns() < until, "original complete read bound")?;
            if let Some(event) = remote.poll()? {
                let Event::Complete {
                    token: seen,
                    packet,
                } = event
                else {
                    return Err(io::Error::other("actual history read failed"));
                };
                require(
                    seen == token && boottime_ns() < until,
                    "exact current token and original deadline",
                )?;
                let Page::Messages { items } = serde_json::from_slice(&packet)? else {
                    return Err(io::ErrorKind::InvalidData.into());
                };
                return require(
                    items.len() == 2
                        && items
                            .iter()
                            .zip(&self.expected)
                            .all(|(m, id)| m.receipt.as_str() == id && m.text == "hello"),
                    "exact actual Remote IDs/text",
                );
            }
            tick();
        }
    }
    fn stop(&mut self) -> io::Result<()> {
        if self.daemon.status.is_some() {
            return Ok(());
        }
        let mut stop = Process::new(
            Command::new(env!("CARGO_BIN_EXE_messages-stop")).arg(self.runtime.join("admin.sock")),
            self.close,
        )?;
        require(stop.wait()?.success(), "normal exact-peer Stop helper")?;
        require(self.daemon.wait()?.success(), "normal daemon actual wait")
    }
    fn restart_daemon(&mut self) -> io::Result<()> {
        require(
            self.daemon.status.is_some(),
            "old daemon must be actually waited",
        )?;
        self.daemon = Self::spawn(&self.runtime, &self.raw, &self.inbox, self.close)?;
        while !self.runtime.join("read-provider.sock").exists() {
            require(boottime_ns() < self.work, "new normal daemon startup bound")?;
            tick();
        }
        Ok(())
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = self.stop();
        if self.daemon.status.is_some() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
fn finish(mut case: Case, observation: io::Result<()>) {
    let stopped = case.stop();
    assert!(
        stopped.is_ok(),
        "actual normal Stop/wait failed: {stopped:?}"
    );
    observation.unwrap();
}

#[test]
#[ignore = "isolated root; actual normal daemon and Remote"]
fn remote_exact_history_and_local_stop_preserve_background_owner() {
    let case = Case::new().unwrap();
    let observation = (|| {
        let mut remote = case.connect()?;
        let generation = remote.incarnation().to_owned();
        let (writer, reader) = case.roles()?;
        case.read(&mut remote)?;
        remote.stop();
        require(
            !remote.occupied() && !remote.read_occupied() && !remote.retirement_uncertain(),
            "local Stop with no pending read",
        )?;
        require(
            living(&writer.alive) && living(&reader.alive),
            "local Stop must retain actual background writer/reader",
        )?;
        let mut next = case.connect()?;
        require(
            next.incarnation() == generation,
            "local Stop retains daemon generation",
        )?;
        case.read(&mut next)?;
        next.stop();
        Ok(())
    })();
    finish(case, observation);
}
#[test]
#[ignore = "isolated root; actual stopped reader and acknowledged retirement"]
fn cancelled_read_reaps_only_reader_before_explicit_recovery() {
    let case = Case::new().unwrap();
    let observation = (|| {
        let mut remote = case.connect()?;
        let generation = remote.incarnation().to_owned();
        let (writer, reader) = case.roles()?;
        rustix::process::kill_process(reader.pid, Signal::STOP)?;
        let token = remote.start(
            serde_json::to_value(case.query()?)?,
            boottime_ns() + 1_000_000_000,
        )?;
        let _ = remote.poll()?;
        remote.cancel_read(token);
        require(
            remote.read_occupied(),
            "sent read retains local retirement capacity",
        )?;
        loop {
            require(boottime_ns() < case.work, "original cancellation bound")?;
            if let Some(event) = remote.poll()? {
                require(
                    matches!(event,Event::Failed{token:seen}if seen==token),
                    "exact cancelled token",
                )?;
                break;
            }
            tick();
        }
        require(
            !remote.read_occupied() && !remote.retirement_uncertain() && !living(&reader.alive),
            "Retired ACK follows actual server reader reap",
        )?;
        require(
            living(&writer.alive),
            "reader cancellation must retain sole writer",
        )?;
        remote.restart()?;
        case.ready(&mut remote)?;
        require(
            remote.incarnation() != generation,
            "explicit readonly recovery creates fresh generation",
        )?;
        let (same_writer, new_reader) = case.roles()?;
        require(
            same_writer.pid == writer.pid && living(&new_reader.alive),
            "read recovery must preserve writer identity",
        )?;
        case.read(&mut remote)?;
        remote.stop();
        Ok(())
    })();
    finish(case, observation);
}
#[test]
#[ignore = "isolated root; actual held reader and original read deadline"]
fn original_deadline_fences_page_and_local_close_retains_retirement_uncertainty() {
    let case = Case::new().unwrap();
    let observation = (|| {
        let mut remote = case.connect()?;
        let (writer, reader) = case.roles()?;
        rustix::process::kill_process(reader.pid, Signal::STOP)?;
        let until = boottime_ns() + 20_000_000;
        let token = remote.start(serde_json::to_value(case.query()?)?, until)?;
        loop {
            require(boottime_ns() < case.work, "original deadline case bound")?;
            if let Some(event) = remote.poll()? {
                require(
                    matches!(event,Event::Failed{token:seen}if seen==token),
                    "expired read must not publish a Page",
                )?;
                break;
            }
            tick();
        }
        require(
            boottime_ns() >= until
                && !remote.occupied()
                && !remote.read_occupied()
                && remote.retirement_uncertain(),
            "local timeout closure is not server reap evidence",
        )?;
        while living(&reader.alive) {
            require(
                boottime_ns() < case.work,
                "server reader must retire under original case bound",
            )?;
            tick();
        }
        require(
            living(&writer.alive),
            "expired read preserves background writer",
        )?;
        Ok(())
    })();
    finish(case, observation);
}
#[test]
#[ignore = "isolated root; actual daemon replacement and old request fencing"]
fn daemon_replacement_fences_old_request_and_requires_explicit_new_connection() {
    let mut case = Case::new().unwrap();
    let observation = (|| {
        let mut remote = case.connect()?;
        let old = remote.incarnation().to_owned();
        let (_writer, reader) = case.roles()?;
        rustix::process::kill_process(reader.pid, Signal::STOP)?;
        let token = remote.start(
            serde_json::to_value(case.query()?)?,
            boottime_ns() + 1_000_000_000,
        )?;
        let _ = remote.poll()?;
        case.stop()?;
        require(
            matches!(remote.poll()?,Some(Event::Failed{token:seen})if seen==token),
            "old daemon request cannot complete",
        )?;
        require(
            !remote.available() && !remote.occupied() && remote.incarnation() == old,
            "no automatic replacement or generation adoption",
        )?;
        case.restart_daemon()?;
        remote.restart()?;
        case.ready(&mut remote)?;
        require(
            remote.incarnation() != old,
            "replacement needs fresh generation",
        )?;
        case.read(&mut remote)?;
        remote.stop();
        Ok(())
    })();
    finish(case, observation);
}

#[test]
#[ignore = "isolated root; actual normal Main SIGTERM and retained workers"]
fn normal_sigterm_drains_background_children_before_main_exit() {
    let mut case = Case::new().unwrap();
    let observation = (|| {
        let mut remote = case.connect()?;
        let (writer, reader) = case.roles()?;
        let pid = Pid::from_raw(case.daemon.child.id() as i32).ok_or(io::ErrorKind::InvalidData)?;
        rustix::process::kill_process(pid, Signal::TERM)?;
        require(case.daemon.wait()?.success(), "normal SIGTERM Main exit")?;
        require(
            !living(&writer.alive) && !living(&reader.alive),
            "Main exit follows retained writer/reader death",
        )?;
        let _ = remote.poll()?;
        require(
            !remote.available(),
            "terminated daemon cannot remain available",
        )?;
        remote.stop();
        Ok(())
    })();
    finish(case, observation);
}
