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
pub struct Job {
    pub token: u64,
    pub desired: Option<Vec<(u64, Role, String)>>,
}
#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub token: u64,
    pub cookie: Option<u64>,
    pub links: Vec<ObservedLink>,
    pub applied: bool,
    pub error: Option<String>,
}

struct Stack {
    profile: Profile,
    backend: pipewire::Backend,
    cookie: Option<u64>,
}
impl Stack {
    fn refresh(&mut self) -> Result<Vec<ObservedLink>, &'static str> {
        let graph = pipewire::read_graph();
        let cookie = graph.as_ref().ok().map(pipewire::Graph::cookie);
        if self.cookie != cookie {
            self.backend.stop_all();
            self.cookie = cookie;
        }
        graph.map(|graph| graph.observed(&self.profile))
    }
    fn execute(&mut self, job: Job) -> Reply {
        let mut applied = false;
        let result = (|| {
            if let Some(desired) = &job.desired {
                let end = BootClock
                    .now()
                    .checked_after(Duration::from_secs(2))
                    .ok_or("deadline_overflow")?;
                let graph = pipewire::read_graph()?;
                // Establish the new process's graph identity before creating links.
                // Otherwise the first refresh would discard its own fresh workers.
                if self.cookie != Some(graph.cookie()) {
                    self.backend.stop_all();
                    self.cookie = Some(graph.cookie());
                }
                if BootClock.now() >= end {
                    return Err("application_expired");
                }
                self.backend.apply(&self.profile, desired, &graph)?;
                let wanted = pipewire::wanted(&self.profile, desired)?;
                loop {
                    let links = self.refresh()?;
                    // A late successful graph reply cannot settle application.
                    if BootClock.now() >= end {
                        return Err("application_expired");
                    }
                    let actual: std::collections::BTreeSet<_> = links
                        .iter()
                        .filter(|link| link.state == "active")
                        .map(|link| (link.source.clone(), link.target.clone()))
                        .collect();
                    if wanted == actual && links.len() == wanted.len() {
                        applied = true;
                        return Ok(links);
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            self.refresh()
        })();
        match result {
            Ok(links) => Reply {
                token: job.token,
                cookie: self.cookie,
                links,
                applied,
                error: None,
            },
            Err(error) => Reply {
                token: job.token,
                cookie: self.cookie,
                links: Vec::new(),
                applied: false,
                error: Some(error.into()),
            },
        }
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
        self.outgoing = serde_json::to_vec(&job).map_err(|_| "invalid_backend_job")?;
        self.outgoing.push(b'\n');
        if self.outgoing.len() > FRAME_LIMIT {
            self.outgoing.clear();
            return Err("backend_job_budget");
        }
        self.sent = 0;
        self.busy = Some((
            token,
            BootClock
                .now()
                .checked_after(Duration::from_secs(3))
                .ok_or("deadline_overflow")?,
        ));
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
