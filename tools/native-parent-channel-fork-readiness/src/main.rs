//! Actual fork ownership control; source-only until built and executed.
mod common;
use common::*;
use pokelinux_process_owner::{
    bootstrap::{BoundParent, WORKER_SELECTOR},
    supervisor::{ProcessIdentity, Supervisor},
};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    process::{self, Pid, PidfdFlags, Signal, WaitOptions},
};
use std::{fs, io, path::PathBuf};

fn worker(until: u64) -> io::Result<()> {
    let mut parent = BoundParent::from_stdin()?;
    // The real bootstrapped worker sends nothing until the original parent asks.
    let request = loop {
        before(until)?;
        if let Some(bytes) = parent.receive_message(deadline(until))? {
            break bytes;
        }
        pause();
    };
    let bytes: [u8; 8] = request.try_into().map_err(|_| io::ErrorKind::InvalidData)?;
    let reply_until = u64::from_le_bytes(bytes);
    require(reply_until <= until, "reply renewed original work cutoff")?;
    before(reply_until)?;
    let sampled = now();
    parent.send_message(deadline(reply_until), &sampled.to_le_bytes())?;
    let _parent = parent;
    loop {
        std::thread::park();
    }
}

// SAFETY: this standalone binary verifies that its forking process has exactly
// one thread. The child calls only the inherited ownership check and _exit;
// it never unwinds or drops the inherited Supervisor/worker handles.
#[allow(unsafe_code)]
fn fork_refusal(
    owner: &mut Supervisor,
    identity: ProcessIdentity,
    until: u64,
    cleanup_until: u64,
) -> io::Result<u64> {
    let tasks = fs::read_dir("/proc/self/task")?.collect::<io::Result<Vec<_>>>()?;
    require(tasks.len() == 1, "fork control requires one parent thread")?;
    drop(tasks);
    before(until)?;
    let child = unsafe { libc::fork() };
    if child < 0 {
        return Err(io::Error::last_os_error());
    }
    if child == 0 {
        let rejected = matches!(
            owner.message_ready(identity),
            Err(error) if error.kind() == io::ErrorKind::InvalidInput
        );
        unsafe { libc::_exit(if rejected { 0 } else { 17 }) };
    }
    // The positive, unreaped child PID cannot be reused. Retain it even when
    // acquiring its pidfd fails, so error cleanup still owns the exact child.
    let pid = Pid::from_raw(child).expect("positive fork child PID");
    let mut waited = false;
    let result = (|| -> io::Result<u64> {
        let pidfd = process::pidfd_open(pid, PidfdFlags::empty())?;
        let zero = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        loop {
            before(until)?;
            let mut watch = [PollFd::new(&pidfd, PollFlags::IN)];
            poll(&mut watch, Some(&zero))?;
            if !watch[0].revents().is_empty() {
                require(watch[0].revents() == PollFlags::IN, "fork pidfd error")?;
                break;
            }
            pause();
        }
        let (actual, status) = process::waitpid(Some(pid), WaitOptions::NOHANG)?
            .ok_or_else(|| io::Error::other("ready fork pidfd without actual wait"))?;
        waited = true;
        let waited_at = now();
        require(
            actual == pid && status.exit_status() == Some(0),
            "fork hint was not InvalidInput",
        )?;
        before(until)?;
        require(
            matches!(
                process::waitpid(Some(pid), WaitOptions::NOHANG),
                Err(rustix::io::Errno::CHILD)
            ),
            "fork child was not fully waited",
        )?;
        println!(
            "FORK-WAIT pid={child} pidfd_terminal=true exit_code=0 ECHILD=true waited_at_ns={waited_at}"
        );
        Ok(waited_at)
    })();
    if !waited {
        let _ = process::kill_process(pid, Signal::KILL);
        while now() < cleanup_until {
            match process::waitpid(Some(pid), WaitOptions::NOHANG) {
                Ok(Some((actual, _))) if actual == pid => {
                    waited = true;
                    break;
                }
                Ok(None) | Err(rustix::io::Errno::INTR) => pause(),
                _ => break,
            }
        }
        if !waited {
            // Never unwind into worker disposal with unresolved fork custody.
            std::process::abort();
        }
    }
    result
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == WORKER_SELECTOR) {
        require(args.len() == 3 && args[1] == "silent", "worker arguments")?;
        return worker(args[2].parse().map_err(|_| io::ErrorKind::InvalidInput)?);
    }
    require(
        args.len() == 1,
        "usage: parent-fork-readiness NEW_PRIVATE_RUNTIME",
    )?;
    let runtime = PathBuf::from(&args[0]);
    private_dir(&runtime)?;
    let began = now();
    let until = plus(began, 4_000_000_000)?;
    let cleanup_until = plus(began, 6_000_000_000)?;
    let mut owner = Supervisor::new(runtime.join("admission"));
    let mut command = command("silent", &[until.to_string()])?;
    let identity = start(&mut owner, &mut command, cleanup_until)?;
    let result = (|| -> io::Result<()> {
        ready(&mut owner, identity, until)?;
        let worker_pid = owner.child_pid();
        require(
            !owner.message_ready(identity)?,
            "silent worker was readable",
        )?;
        let waited_at = fork_refusal(&mut owner, identity, until, cleanup_until)?;
        require(
            owner.occupied() && owner.admits(identity),
            "fork changed parent admission",
        )?;
        require(
            owner.child_pid() == worker_pid,
            "fork replaced original worker",
        )?;
        require(
            !owner.message_ready(identity)?,
            "fork changed silent channel",
        )?;
        let reply_until = until.min(plus(now(), 1_000_000_000)?);
        let requested_at = now();
        owner.send_message(identity, deadline(reply_until), &reply_until.to_le_bytes())?;
        loop {
            before(reply_until)?;
            if owner.message_ready(identity)? {
                break;
            }
            pause();
        }
        let reply = owner
            .receive_message(identity, deadline(reply_until))?
            .ok_or_else(|| io::Error::other("readable reply disappeared"))?;
        let bytes: [u8; 8] = reply.try_into().map_err(|_| io::ErrorKind::InvalidData)?;
        let sampled = u64::from_le_bytes(bytes);
        before(reply_until)?;
        require(
            sampled >= requested_at && sampled >= waited_at && sampled < reply_until,
            "reply was not fresh after fork wait",
        )?;
        owner.stop()?;
        require(
            owner.occupied() && !owner.admits(identity),
            "Stop custody/admission",
        )?;
        killed(reap(&mut owner, cleanup_until)?)?;
        journal_unchanged(&mut owner, &mut command, &runtime, identity)?;
        println!(
            "FORK-READINESS passed=true original_parent_usable=true one_request_reply=true original_until_ns={until} original_cleanup_until_ns={cleanup_until}"
        );
        Ok(())
    })();
    dispose(&mut owner, cleanup_until)?;
    result
}
