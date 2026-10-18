//! One Main-owned finite preview producer. No authority, route or PCM claim.
use pokelinux_power::clock::{BootClock, BootTime, Clock};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use rustix::pipe::{PipeFlags, fcntl_setpipe_size, pipe_with};
use rustix::process::{Pid, PidfdFlags, Signal, pidfd_open, pidfd_send_signal};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::process::{Child, Command, ExitStatus, Stdio};

const STDERR_LIMIT: usize = 4096;
const WRITE_QUANTUM: usize = 65_536;
const PCM_LIMIT: usize = 4 * 48_000 * 4;
const PCM_PIPE_REQUEST: usize = 4096;
const PCM_PIPE_LIMIT: usize = 16_384;

/// Local process completion, never evidence of delivered sound or graph cleanup.
#[derive(Debug)]
pub struct Finished {
    pub status: ExitStatus,
    pub successful: bool,
}

/// Retains the actual child and pidfd through wait, stderr EOF and handle close.
/// Main separately fences Starting at its original admission/start deadline.
pub struct Process {
    child: Child,
    pid: u32,
    pidfd: Option<OwnedFd>,
    input: Option<File>,
    stderr: Option<File>,
    pcm: Vec<u8>,
    unmuted_pcm: Vec<u8>,
    muted: bool,
    sent: usize,
    diagnostic: Vec<u8>,
    status: Option<ExitStatus>,
    stderr_eof: bool,
    retired: bool,
    closed: bool,
    work_until: BootTime,
}

impl Process {
    /// `source` comes from trusted deployment, never an application request.
    /// The tool, format, PCM and non-autoconnect properties are fixed by Rust.
    pub fn start(
        source: &str,
        level: crate::PreviewLevel,
        work_until: BootTime,
    ) -> io::Result<Self> {
        if source.is_empty() || source.len() > 256 || source.chars().any(char::is_control) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // This private graph has no session-manager port configuration policy.
        // Configure this owned mono adapter at creation; linking remains separate.
        let port_config = serde_json::json!({
            "mode": "dsp",
            "monitor": false,
            "control": false,
            "position": "preserve",
        })
        .to_string();
        let properties = serde_json::json!({
            "adapter.auto-port-config": port_config,
            "node.name": source,
            "node.autoconnect": false,
            "node.dont-reconnect": true,
        })
        .to_string();
        let mut command = Command::new(std::env::current_exe()?);
        command.env("PIPEWIRE_DEBUG", "0");
        command.args([
            "--tool-worker",
            &std::process::id().to_string(),
            "/usr/bin/pw-cat",
            "--playback",
            "--raw",
            "--format",
            "f32",
            "--rate",
            "48000",
            "--channels",
            "1",
            "--channel-map",
            "MONO",
            "--target",
            "0",
            "-P",
            &properties,
            "-",
        ]);
        let unmuted = crate::PreviewLevel::new(u64::from(level.percent()), false)
            .expect("validated frozen preview percentage");
        let mut process = Self::spawn_owned(
            command,
            crate::fixed_pcm::fixed_preview_pcm(unmuted),
            work_until,
        )?;
        // No input has been written before the owner is returned. Retain the
        // immutable original percentage so an initially muted Play can unmute.
        if level.muted() {
            process.pcm.fill(0);
            process.muted = true;
        }
        Ok(process)
    }

    // Production and the private finite-process fixture share the same owner.
    // No program/argument/data selector is exposed by production IPC or CLI.
    fn spawn_owned(mut command: Command, pcm: Vec<u8>, work_until: BootTime) -> io::Result<Self> {
        if pcm.len() > PCM_LIMIT {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        if BootClock.now() >= work_until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        // Opposite pipe endpoints have distinct file descriptions. Only the
        // parent endpoints become nonblocking; child read_to_end stays blocking.
        let (child_input, parent_input) = pipe_with(PipeFlags::CLOEXEC)?;
        // Limit already queued PCM before any child exists. Linux may round the
        // request to a kernel page; admit 4K/16K pages without inheriting the
        // default pipe's much larger playback lookahead. This is not a bound on
        // downstream PipeWire buffering or a claim of immediate sink silence.
        if fcntl_setpipe_size(&parent_input, PCM_PIPE_REQUEST)? > PCM_PIPE_LIMIT {
            return Err(io::ErrorKind::Unsupported.into());
        }
        let (parent_stderr, child_stderr) = pipe_with(PipeFlags::CLOEXEC)?;
        for fd in [&parent_input, &parent_stderr] {
            fcntl_setfl(fd, fcntl_getfl(fd)? | OFlags::NONBLOCK)?;
        }
        if BootClock.now() >= work_until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let child = command
            .stdin(Stdio::from(child_input))
            .stdout(Stdio::null())
            .stderr(Stdio::from(child_stderr))
            .spawn()?;
        let pid = child.id();
        // Own the child before the remaining fallible pidfd setup. Its PID
        // cannot be reused while this direct child is retained and unreaped.
        let mut process = Self {
            child,
            pid,
            pidfd: None,
            input: Some(File::from(parent_input)),
            stderr: Some(File::from(parent_stderr)),
            unmuted_pcm: pcm.clone(),
            pcm,
            muted: false,
            sent: 0,
            diagnostic: Vec::new(),
            status: None,
            stderr_eof: false,
            retired: false,
            closed: false,
            work_until,
        };
        let retained_pidfd = Pid::from_raw(pid as i32)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))
            .and_then(|pid| pidfd_open(pid, PidfdFlags::NONBLOCK).map_err(io::Error::from));
        match retained_pidfd {
            Ok(pidfd) => process.pidfd = Some(pidfd),
            Err(_) => process.retire(),
        }
        if BootClock.now() >= work_until {
            process.retire();
        }
        // After spawn, Main always receives ownership, including failed setup.
        // Returning Err here would enter Drop's last-resort abort with live custody.
        // Main must retain a refused admission in its Retiring slot and poll it.
        Ok(process)
    }

    /// Change future complete frames on this exact retained preview producer.
    /// Buffered audio is separate from acceptance of the digital feed policy.
    pub fn set_live_muted(&mut self, muted: bool, original_until: BootTime) -> io::Result<()> {
        let until = original_until.min(self.work_until);
        if BootClock.now() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if self.retired || self.closed || self.status.is_some() || !self.alive() {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        // Preserve the remainder of a sample that has already entered the pipe.
        // No mutation can retract buffered audio or renew this finite asset.
        let first = self.sent.div_ceil(4) * 4;
        if self.input.is_none() || first >= self.pcm.len() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if !self.pcm.len().is_multiple_of(4) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if muted {
            self.pcm[first..].fill(0);
        } else {
            self.pcm[first..].copy_from_slice(&self.unmuted_pcm[first..]);
        }
        if BootClock.now() >= until {
            // This is one Main-thread operation: no poll/write can occur while
            // changing the tail. Roll back a late local change before any IO.
            if self.muted {
                self.pcm[first..].fill(0);
            } else {
                self.pcm[first..].copy_from_slice(&self.unmuted_pcm[first..]);
            }
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.muted = muted;
        Ok(())
    }

    /// Accepted digital feed policy, never an observation of delivered silence.
    pub fn muted(&self) -> bool {
        self.muted
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Refused post-spawn setup still returns an owned retiring Process.
    /// This check permits admission only; it never releases child custody.
    pub fn admission_ready(&self) -> bool {
        !self.retired && !self.closed && self.alive()
    }

    /// A fresh zero-timeout observation of the retained exact producer pidfd.
    pub fn alive(&self) -> bool {
        let Some(fd) = &self.pidfd else {
            return false;
        };
        let mut fds = [PollFd::new(fd, PollFlags::IN)];
        let zero = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        poll(&mut fds, Some(&zero)).is_ok_and(|count| count == 0 && fds[0].revents().is_empty())
    }

    /// Fence local acceptance and close PCM input before signaling. No wait here.
    pub fn retire(&mut self) {
        self.retired = true;
        self.input.take();
        if self.status.is_none() {
            if let Some(fd) = &self.pidfd {
                let _ = pidfd_send_signal(fd, Signal::KILL);
            } else {
                // Constructor failure still owns this unreaped direct Child.
                let _ = self.child.kill();
            }
        }
    }

    /// No live child, stderr writer custody or owned pipe/pidfd is released early.
    pub fn settled(&self) -> bool {
        self.closed
    }

    /// Bounded, nonblocking progress. Failures fence acceptance but never detach.
    /// A Finished event is emitted exactly once, after actual wait and stderr EOF.
    /// Expired work fences success; delayed polling still reclaims the child.
    /// Main retains this owner and enforces its separate whole-service Stop bound.
    pub fn poll(&mut self) -> Option<Finished> {
        if self.closed {
            return None;
        }
        if BootClock.now() >= self.work_until {
            self.retire();
        }
        if self.status.is_none() {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.status = Some(status);
                    if !status.success() || self.sent != self.pcm.len() {
                        self.retire();
                    }
                }
                Ok(None) => {}
                Err(_) => self.retire(),
            }
        }
        if !self.retired && self.status.is_none() {
            if self.sent == self.pcm.len() {
                self.input.take();
            } else if let Some(input) = &mut self.input {
                let end = self.sent.saturating_add(WRITE_QUANTUM).min(self.pcm.len());
                match input.write(&self.pcm[self.sent..end]) {
                    Ok(0) => self.retire(),
                    Ok(count) => {
                        self.sent += count;
                        if self.sent == self.pcm.len() {
                            self.input.take();
                        }
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(_) => self.retire(),
                }
            }
        }
        if self.status.is_some() {
            self.input.take();
        }
        if let Some(stderr) = &mut self.stderr {
            let mut bytes = [0_u8; STDERR_LIMIT + 1];
            match stderr.read(&mut bytes) {
                Ok(0) => {
                    self.stderr_eof = true;
                    self.stderr.take();
                }
                Ok(count) => {
                    let remaining = STDERR_LIMIT.saturating_sub(self.diagnostic.len());
                    self.diagnostic
                        .extend_from_slice(&bytes[..count.min(remaining)]);
                    if count > remaining {
                        self.retire();
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                // A read error is not EOF. Retain custody; Main owns the
                // separate whole-service Stop deadline, never a renewed work deadline.
                Err(_) => self.retire(),
            }
        }
        if BootClock.now() >= self.work_until {
            self.retire();
        }
        if let Some(status) = self.status
            && self.stderr_eof
        {
            self.input.take();
            self.stderr.take();
            self.pidfd.take();
            self.closed = true;
            return Some(Finished {
                status,
                successful: !self.retired && status.success() && self.sent == self.pcm.len(),
            });
        }
        None
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.closed {
            return;
        }
        // Dropping occupied capacity is a programming error, not a cleanup
        // operation that may block Main. Retire/kill, then fail the service.
        self.retire();
        eprintln!("owned preview process dropped before actual wait/stderr EOF");
        std::process::abort();
    }
}

#[cfg(test)]
#[allow(non_snake_case)] // Preserve the independently frozen fixture's names.
#[path = "preview_process_tests.rs"]
mod tests;
