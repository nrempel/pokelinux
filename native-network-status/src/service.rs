use pokelinux_kernel_identity::{TerminationGuard, termination_requested};
use pokelinux_network_state::{KernelObserver, NamespaceIdentity};
use pokelinux_network_status::{Collection, Counts, Model, View};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    pipe::{PipeFlags, pipe_with},
    process::{self, Signal},
    time::{ClockId, clock_gettime},
};
use serde_json::json;
#[cfg(test)]
use std::path::Path;
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixListener,
    },
    process::{Child, Command, Stdio},
};

pub fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    (t.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(t.tv_nsec as u64)
}
fn namespace() -> io::Result<(fs::File, NamespaceIdentity)> {
    let file = fs::File::open("/proc/thread-self/ns/net")?;
    let metadata = file.metadata()?;
    let identity = NamespaceIdentity {
        dev: metadata.dev(),
        ino: metadata.ino(),
    };
    Ok((file, identity))
}
fn snapshot(view: View) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&json!({
        "schema": 1, "revision": view.revision, "until_ns": view.until_ns.to_string(),
        "available": view.counts.is_some(),
        "local_interfaces": view.counts.map(|v| v.local_interfaces),
        "address_records": view.counts.map(|v| v.address_records),
    }))
    .expect("finite status snapshot");
    bytes.push(b'\n');
    bytes
}
enum Source {
    Kernel(NamespaceIdentity),
    #[cfg(test)]
    #[allow(
        dead_code,
        reason = "Used by the private fixture driver, absent from the normal bin test entry."
    )]
    Fixture(std::path::PathBuf),
}
struct Reader {
    child: Child,
    output: fs::File,
    collection: Collection,
    bytes: Vec<u8>,
    reaped: bool,
    success: bool,
    eof: bool,
    retired: bool,
}
impl Reader {
    fn start(collection: Collection, source: &Source) -> io::Result<Self> {
        let (output, writer) = pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK)?;
        let mut command = Command::new(std::env::current_exe()?);
        match source {
            Source::Kernel(ns) => {
                command.args([
                    "--collect",
                    &std::process::id().to_string(),
                    &collection.until_ns.to_string(),
                    &ns.dev.to_string(),
                    &ns.ino.to_string(),
                ]);
            }
            #[cfg(test)]
            Source::Fixture(directory) => {
                command.args([
                    "--fixture-collect",
                    &std::process::id().to_string(),
                    &collection.until_ns.to_string(),
                    directory.to_str().ok_or(io::ErrorKind::InvalidInput)?,
                ]);
            }
        }
        let child = command
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::from(writer))
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self {
            child,
            output: fs::File::from(output),
            collection,
            bytes: Vec::new(),
            reaped: false,
            success: false,
            eof: false,
            retired: false,
        })
    }
    fn fence(&mut self) {
        self.retired = true;
        if !self.reaped {
            let _ = self.child.kill();
        }
    }
    fn poll(&mut self) -> io::Result<bool> {
        if !self.reaped
            && let Some(code) = self.child.try_wait()?
        {
            self.reaped = true;
            self.success = code.success();
        }
        if !self.eof {
            let mut buffer = [0; 129];
            match self.output.read(&mut buffer) {
                Ok(0) => self.eof = true,
                Ok(n) if self.bytes.len() + n <= 128 => self.bytes.extend_from_slice(&buffer[..n]),
                Ok(_) => self.fence(),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => {
                    self.fence();
                    return Err(e);
                }
            }
        }
        Ok(self.reaped && self.eof)
    }
    fn value(&self) -> Option<Counts> {
        if self.retired
            || !self.reaped
            || !self.eof
            || !self.success
            || now() >= self.collection.until_ns
            || self.bytes.last() != Some(&b'\n')
            || self.bytes[..self.bytes.len().saturating_sub(1)].contains(&b'\n')
        {
            return None;
        }
        let value: Counts = serde_json::from_slice(&self.bytes).ok()?;
        value.valid().then_some(value)
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            std::process::abort();
        }
    }
}
pub fn child_setup(parent: &str, until: &str) -> io::Result<(process::Pid, u64)> {
    let parent = parent
        .parse::<i32>()
        .ok()
        .and_then(process::Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let deadline: u64 = until.parse().map_err(|_| io::ErrorKind::InvalidInput)?;
    process::set_parent_process_death_signal(Some(Signal::KILL))?;
    child_check(parent, deadline)?;
    Ok((parent, deadline))
}
pub fn child_check(parent: process::Pid, until: u64) -> io::Result<()> {
    if process::getppid() != Some(parent) || now() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    Ok(())
}
pub fn collect(parent: &str, until: &str, dev: &str, ino: &str) -> io::Result<()> {
    let (parent, until) = child_setup(parent, until)?;
    let expected = NamespaceIdentity {
        dev: dev.parse().map_err(|_| io::ErrorKind::InvalidInput)?,
        ino: ino.parse().map_err(|_| io::ErrorKind::InvalidInput)?,
    };
    let (_retained, actual) = namespace()?;
    if actual != expected {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mut observer = KernelObserver::subscribe_current_namespace(until)
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    let snapshot = observer
        .refresh(until)
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    if snapshot.namespace != expected {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let counts = Counts::from_kernel(&snapshot).ok_or(io::ErrorKind::InvalidData)?;
    drop(observer);
    child_check(parent, until)?;
    let mut bytes = serde_json::to_vec(&counts)?;
    bytes.push(b'\n');
    std::io::stdout().lock().write_all(&bytes)
}
pub fn serve(socket: &str) -> io::Result<()> {
    let (_retained, ns) = namespace()?;
    run(socket, Source::Kernel(ns))
}
#[cfg(test)]
#[allow(
    dead_code,
    reason = "Used by the private fixture driver, absent from the normal bin test entry."
)]
pub fn serve_fixture(socket: &str, directory: &Path) -> io::Result<()> {
    run(socket, Source::Fixture(directory.into()))
}
fn run(socket: &str, source: Source) -> io::Result<()> {
    if [
        process::getuid().as_raw(),
        process::geteuid().as_raw(),
        process::getgid().as_raw(),
        process::getegid().as_raw(),
    ] != [0; 4]
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let _termination = TerminationGuard::install()?;
    let listener = UnixListener::bind(socket)?;
    fs::set_permissions(socket, fs::Permissions::from_mode(0o666))?;
    listener.set_nonblocking(true)?;
    let mut model = Model::default();
    let mut reader: Option<Reader> = None;
    let mut next = 0;
    let mut stopping = false;
    let mut fatal = None;
    loop {
        let sample = now();
        if termination_requested() && !stopping {
            stopping = true;
            model.withdraw();
            if let Some(reader) = &mut reader {
                reader.fence();
            }
        }
        model.view(sample);
        if let Some(r) = &mut reader {
            if now() >= r.collection.until_ns && !r.retired {
                model.withdraw();
                r.fence();
            }
            match r.poll() {
                Ok(true) => {
                    model.complete(r.collection, now(), r.value());
                    reader.take();
                }
                Ok(false) => {}
                Err(error) => {
                    model.withdraw();
                    r.fence();
                    stopping = true;
                    fatal = Some(error);
                }
            }
        }
        if stopping && reader.is_none() {
            break;
        }
        if !stopping
            && reader.is_none()
            && sample >= next
            && let Some(collection) = model.begin(sample)
        {
            next = sample.saturating_add(1_000_000_000);
            match Reader::start(collection, &source) {
                Ok(r) => reader = Some(r),
                Err(_) => {
                    model.complete(collection, now(), None);
                }
            }
        }
        for _ in 0..16 {
            match listener.accept() {
                Ok((mut peer, _)) => {
                    if peer.set_nonblocking(true).is_ok() {
                        let _ = peer.write(&snapshot(model.view(now())));
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    break;
                }
                Err(error) => {
                    stopping = true;
                    model.withdraw();
                    if let Some(r) = &mut reader {
                        r.fence();
                    }
                    fatal = Some(error);
                    break;
                }
            }
        }
        let timeout = if reader.is_some() {
            50_000_000
        } else {
            next.saturating_sub(now()).clamp(1_000_000, 1_000_000_000)
        };
        let mut descriptors = vec![PollFd::new(&listener, PollFlags::IN)];
        if let Some(r) = &reader
            && !r.eof
        {
            descriptors.push(PollFd::new(&r.output, PollFlags::IN));
        }
        if let Err(error) = poll(
            &mut descriptors,
            Some(&Timespec {
                tv_sec: (timeout / 1_000_000_000) as i64,
                tv_nsec: (timeout % 1_000_000_000) as i64,
            }),
        ) && error != rustix::io::Errno::INTR
        {
            stopping = true;
            model.withdraw();
            if let Some(r) = &mut reader {
                r.fence();
            }
            fatal = Some(error.into());
        }
    }
    drop(listener);
    fs::remove_file(socket)?;
    fatal.map_or(Ok(()), Err)
}
