//! One retained child and its bounded result pipe. Cancellation fences
//! publication, while retirement still requires an actual wait and pipe EOF.
use crate::model::{Observation, Ticket};
use rustix::{
    pipe::{PipeFlags, pipe_with},
    process::{self, Pid, Signal},
    time::{ClockId, clock_gettime},
};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    process::{Child, Command, Stdio},
};

const MAX_PAYLOAD: usize = 128;

fn boottime() -> io::Result<u64> {
    let time = clock_gettime(ClockId::Boottime);
    if !(0..1_000_000_000).contains(&time.tv_nsec) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    u64::try_from(time.tv_sec)
        .ok()
        .and_then(|seconds| seconds.checked_mul(1_000_000_000))
        .and_then(|seconds| seconds.checked_add(time.tv_nsec as u64))
        .ok_or_else(|| io::ErrorKind::InvalidData.into())
}

/// The fixed worker entry must call this before backend construction or I/O.
/// Safe Rust cannot install a post-fork hook in Command; the exec entry owns
/// this setup and checks the original parent again after installing PDEATHSIG.
pub fn child_setup(parent: u32, until_ns: u64) -> io::Result<()> {
    process::set_parent_process_death_signal(Some(Signal::KILL))?;
    child_check(parent, until_ns)
}

/// Check the same parent and immutable BOOTTIME deadline after backend I/O and
/// before writing a result. A failed Set may already have changed the device;
/// this function makes no rollback or driver-completion claim.
pub fn child_check(parent: u32, until_ns: u64) -> io::Result<()> {
    let parent = i32::try_from(parent)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or(io::ErrorKind::InvalidInput)?;
    if process::getppid() != Some(parent) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    if boottime()? >= until_ns {
        return Err(io::ErrorKind::TimedOut.into());
    }
    Ok(())
}

/// Canonical private child-pipe format; never a public command endpoint.
pub fn encode_observation(value: Observation) -> io::Result<String> {
    if !value.valid() {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let actual = value.actual.map_or_else(|| "-".into(), |n| n.to_string());
    Ok(format!("BR1 {} {} {actual}\n", value.max, value.requested))
}

fn decimal(text: &str) -> Option<u32> {
    if text.is_empty()
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return None;
    }
    text.parse().ok()
}
fn decode(bytes: &[u8]) -> Option<Observation> {
    let line = std::str::from_utf8(bytes.strip_suffix(b"\n")?).ok()?;
    let mut fields = line.split(' ');
    if fields.next()? != "BR1" {
        return None;
    }
    let max = decimal(fields.next()?)?;
    let requested = decimal(fields.next()?)?;
    let actual = match fields.next()? {
        "-" => None,
        text => Some(decimal(text)?),
    };
    if fields.next().is_some() {
        return None;
    }
    let value = Observation {
        max,
        requested,
        actual,
    };
    value.valid().then_some(value)
}

pub struct Worker {
    child: Child,
    output: File,
    ticket: Ticket,
    bytes: Vec<u8>,
    reaped: bool,
    success: bool,
    eof: bool,
    fenced: bool,
}
impl Worker {
    /// Spawn exactly one exec child. The reserved prefix is consumed by the
    /// service's fixed worker entry, which must call child_setup/child_check.
    pub fn start(executable: &Path, args: &[String], ticket: Ticket) -> io::Result<Self> {
        if boottime()? >= ticket.until_ns {
            return Err(io::ErrorKind::TimedOut.into());
        }
        // Both flags are installed atomically before spawn. No fallible fcntl
        // after admission can leave Main owning a blocking pipe or lose a child.
        let (output, writer) = pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK)?;
        let child = Command::new(executable)
            .arg("--brightness-worker")
            .arg(std::process::id().to_string())
            .arg(ticket.until_ns.to_string())
            .args(args)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::from(writer))
            .stderr(Stdio::null())
            .spawn()?;
        // No fallible operation after spawn and before ownership is returned.
        Ok(Self {
            child,
            output: File::from(output),
            ticket,
            bytes: Vec::new(),
            reaped: false,
            success: false,
            eof: false,
            fenced: false,
        })
    }
    pub fn ticket(&self) -> Ticket {
        self.ticket
    }
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Withdraw publication before requesting kill. Failure retains the exact
    /// Child and pipe; repeated fencing never releases or replaces the owner.
    pub fn fence(&mut self) -> io::Result<()> {
        self.fenced = true;
        if self.reaped {
            return Ok(());
        }
        match self.child.kill() {
            Ok(()) => Ok(()),
            Err(error) => {
                // An already-exited child can still require its actual wait.
                if let Some(status) = self.child.try_wait()? {
                    self.reaped = true;
                    self.success = status.success();
                    Ok(())
                } else {
                    Err(error)
                }
            }
        }
    }

    /// One nonblocking wait probe and one bounded pipe read per turn. Even an
    /// error or oversize result retains custody until actual wait plus EOF.
    pub fn poll(&mut self) -> io::Result<bool> {
        if boottime()? >= self.ticket.until_ns && !self.fenced {
            self.fence()?;
        }
        if !self.reaped
            && let Some(status) = self.child.try_wait()?
        {
            self.reaped = true;
            self.success = status.success();
        }
        if !self.eof {
            let mut buffer = [0; MAX_PAYLOAD + 1];
            match self.output.read(&mut buffer) {
                Ok(0) => self.eof = true,
                Ok(count) if self.bytes.len() + count <= MAX_PAYLOAD => {
                    self.bytes.extend_from_slice(&buffer[..count]);
                }
                Ok(_) => {
                    self.bytes.clear();
                    self.fence()?;
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(error) => {
                    let _ = self.fence();
                    // An I/O error is not an observed EOF.
                    return Err(error);
                }
            }
        }
        Ok(self.reaped && self.eof)
    }

    /// Fresh observation only after real retirement. Getter time is sampled
    /// independently of the event-loop/model, so a queued success cannot revive.
    pub fn result(&self) -> Option<Observation> {
        if self.fenced
            || !self.reaped
            || !self.eof
            || !self.success
            || boottime().ok()? >= self.ticket.until_ns
        {
            return None;
        }
        decode(&self.bytes)
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            // No blocking Drop, detached child or invented wait receipt. Service
            // owners must retain this object through poll()==true, including EOF.
            std::process::abort();
        }
    }
}
