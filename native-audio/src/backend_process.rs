//! One owned process performs original-stack IO; the authority never joins its calls.
use crate::Role;
use crate::pipewire::{self, ObservedLink, Profile};
use pokelinux_power::clock::{BootClock, BootTime, Clock};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

const FRAME_LIMIT: usize = 65_536;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub token: u64,
    pub desired: Vec<(u64, Role, String)>,
    pub apply: bool,
    pub media: pipewire::MediaSource,
    pub until_boottime_ns: Option<u64>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub token: u64,
    pub cookie: Option<u64>,
    pub links: Vec<ObservedLink>,
    pub applied: bool,
    pub policy_settled: bool,
    pub media_present: bool,
    pub preview_identity: Option<pipewire::SourceIdentity>,
    pub error: Option<String>,
}

struct Stack {
    profile: Profile,
    backend: pipewire::Backend,
    cookie: Option<u64>,
}
impl Stack {
    fn graph(&mut self) -> Result<pipewire::Graph, &'static str> {
        let graph = pipewire::read_graph();
        let cookie = graph.as_ref().ok().map(pipewire::Graph::cookie);
        if self.cookie != cookie {
            self.backend.stop_all();
            self.cookie = cookie;
        }
        graph
    }
    fn execute(&mut self, job: Job) -> Reply {
        let mut reply = Reply {
            token: job.token,
            cookie: self.cookie,
            links: Vec::new(),
            applied: false,
            policy_settled: false,
            media_present: false,
            preview_identity: None,
            error: None,
        };
        let end = job
            .until_boottime_ns
            .map(|v| BootTime::from_elapsed(Duration::from_nanos(v)))
            .or_else(|| BootClock.now().checked_after(Duration::from_secs(2)));
        let result = (|| {
            let end = end.ok_or("deadline_overflow")?;
            loop {
                if BootClock.now() >= end {
                    return Err("application_expired");
                }
                let graph = self.graph()?;
                reply.cookie = self.cookie;
                reply.media_present = graph.media_present(&self.profile);
                if BootClock.now() >= end {
                    return Err("application_expired");
                }
                let plan = match graph.render_plan(&self.profile, &job.desired, &job.media) {
                    Err("preview_source_pending") if job.apply => {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    other => other?,
                };
                if job.apply {
                    self.backend.apply(&self.profile, &plan.desired, &graph)?;
                }
                let links = graph.observed(&self.profile);
                let actual: std::collections::BTreeSet<_> = links
                    .iter()
                    .filter(|link| link.state == "active")
                    .map(|link| (link.source.clone(), link.target.clone()))
                    .collect();
                // Check again after real graph and link IO; a buffered success is late.
                if BootClock.now() >= end {
                    return Err("application_expired");
                }
                reply.policy_settled = plan.wanted == actual && links.len() == plan.wanted.len();
                reply.applied = reply.policy_settled && !plan.idle_media;
                reply.preview_identity = plan.preview_identity;
                reply.links = links;
                if !job.apply || reply.policy_settled {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })();
        reply.cookie = self.cookie;
        if let Err(error) = result {
            reply.applied = false;
            reply.policy_settled = false;
            reply.preview_identity = None;
            reply.error = Some(error.into());
        }
        reply
    }
}

pub fn run(profile: Profile) -> Result<(), Box<dyn std::error::Error>> {
    let mut stack = Stack {
        profile,
        backend: pipewire::Backend::default(),
        cookie: None,
    };
    let mut reader = std::io::BufReader::new(std::io::stdin().lock());
    loop {
        let mut bytes = Vec::new();
        if reader
            .by_ref()
            .take((FRAME_LIMIT + 1) as u64)
            .read_until(b'\n', &mut bytes)?
            == 0
        {
            return Ok(());
        }
        if bytes.len() > FRAME_LIMIT || bytes.last() != Some(&b'\n') {
            return Err("backend input budget".into());
        }
        let job: Job = serde_json::from_slice(&bytes)?;
        let mut response = serde_json::to_vec(&stack.execute(job))?;
        response.push(b'\n');
        if response.len() > FRAME_LIMIT {
            return Err("backend output budget".into());
        }
        std::io::stdout().write_all(&response)?;
        std::io::stdout().flush()?;
    }
}

pub struct Process {
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
    outgoing: Vec<u8>,
    sent: usize,
    incoming: Vec<u8>,
    busy: Option<(u64, BootTime)>,
    retired: bool,
    reaped: bool,
}
impl Process {
    pub fn start(profile_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut child = Command::new(std::env::current_exe()?)
            .args([
                "--backend-worker",
                &std::process::id().to_string(),
                profile_path,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let input = child.stdin.take().expect("piped backend input");
        let output = child.stdout.take().expect("piped backend output");
        // Construct ownership before fallible setup; Drop must reclaim the child.
        let process = Self {
            child,
            input,
            output,
            outgoing: Vec::new(),
            sent: 0,
            incoming: Vec::new(),
            busy: None,
            retired: false,
            reaped: false,
        };
        for fd in [
            &process.input as &dyn std::os::fd::AsFd,
            &process.output as &dyn std::os::fd::AsFd,
        ] {
            let flags = rustix::fs::fcntl_getfl(fd.as_fd())?;
            rustix::fs::fcntl_setfl(fd.as_fd(), flags | rustix::fs::OFlags::NONBLOCK)?;
        }
        Ok(process)
    }
    pub fn idle(&self) -> bool {
        !self.retired && !self.reaped && self.busy.is_none()
    }
    pub fn reaped(&self) -> bool {
        self.reaped
    }
    pub fn retired(&self) -> bool {
        self.retired
    }
    pub fn submit(&mut self, job: Job) -> Result<(), &'static str> {
        if !self.idle() {
            return Err("backend_occupied");
        }
        let token = job.token;
        let end = match job.until_boottime_ns {
            Some(value) => BootTime::from_elapsed(Duration::from_nanos(value)),
            None => BootClock
                .now()
                .checked_after(Duration::from_secs(3))
                .ok_or("deadline_overflow")?,
        };
        if BootClock.now() >= end {
            return Err("application_expired");
        }
        self.outgoing = serde_json::to_vec(&job).map_err(|_| "invalid_backend_job")?;
        self.outgoing.push(b'\n');
        if self.outgoing.len() > FRAME_LIMIT {
            self.outgoing.clear();
            return Err("backend_job_budget");
        }
        self.sent = 0;
        self.busy = Some((token, end));
        Ok(())
    }
    /// Revoke publication before sending a signal. Retain Child until try_wait reaps it.
    pub fn retire(&mut self) {
        self.retired = true;
        self.busy = None;
        self.outgoing.clear();
        self.incoming.clear();
        if !self.reaped {
            let _ = self.child.kill();
        }
    }
    pub fn poll(&mut self) -> Result<Option<Reply>, &'static str> {
        match self.child.try_wait() {
            Ok(Some(_)) => {
                self.reaped = true;
                if !self.retired {
                    self.retire();
                    return Err("backend_exited");
                }
            }
            Ok(None) => {}
            Err(_) => {
                self.retire();
                return Err("backend_wait_failed");
            }
        }
        if self.retired {
            return Ok(None);
        }
        let Some((token, end)) = self.busy else {
            return Ok(None);
        };
        if BootClock.now() >= end {
            self.retire();
            return Err("backend_deadline");
        }
        if self.sent < self.outgoing.len() {
            match self.input.write(&self.outgoing[self.sent..]) {
                Ok(0) => {
                    self.retire();
                    return Err("backend_input_closed");
                }
                Ok(n) => self.sent += n,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => {
                    self.retire();
                    return Err("backend_input_failed");
                }
            }
        }
        let mut bytes = [0; 8192];
        for _ in 0..9 {
            match self.output.read(&mut bytes) {
                Ok(0) => {
                    self.retire();
                    return Err("backend_output_closed");
                }
                Ok(n) => {
                    self.incoming.extend_from_slice(&bytes[..n]);
                    if self.incoming.len() > FRAME_LIMIT {
                        self.retire();
                        return Err("backend_output_budget");
                    }
                    if self.incoming.last() == Some(&b'\n') {
                        // Check absolute expiry again after consuming the actual pipe bytes.
                        if BootClock.now() >= end {
                            self.retire();
                            return Err("late_backend_reply");
                        }
                        let reply: Reply = serde_json::from_slice(&self.incoming)
                            .map_err(|_| "invalid_backend_reply")?;
                        if reply.token != token {
                            self.retire();
                            return Err("wrong_backend_reply");
                        }
                        self.incoming.clear();
                        self.outgoing.clear();
                        self.busy = None;
                        return Ok(Some(reply));
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    break;
                }
                Err(_) => {
                    self.retire();
                    return Err("backend_output_failed");
                }
            }
        }
        Ok(None)
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        self.retire();
        let end = BootClock
            .now()
            .checked_after(Duration::from_secs(2))
            .expect("cleanup deadline");
        while !self.reaped {
            match self.child.try_wait() {
                Ok(Some(_)) => self.reaped = true,
                Ok(None) if BootClock.now() < end => std::thread::sleep(Duration::from_millis(5)),
                _ => {
                    eprintln!("owned audio backend was not reaped");
                    std::process::abort();
                }
            }
        }
    }
}
