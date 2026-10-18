//! Actual local pipe custody only; no bootstrap or remote-cleanup claim.
use pokelinux_process_owner::{
    deadline::{BootClock, Clock},
    supervisor::Supervisor,
};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    fd::{AsFd, OwnedFd},
    net::{
        self, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendAncillaryBuffer, SendAncillaryMessage, SendFlags,
    },
    process::{Pid, PidfdFlags, WaitId, WaitIdOptions, pidfd_open, waitid},
};
use std::{
    fs,
    io::{self, IoSlice, IoSliceMut, Write},
    mem::MaybeUninit,
    os::unix::{fs::PermissionsExt, net::UnixDatagram},
    process::{Command, Stdio},
    time::Duration,
};

fn now() -> u64 {
    BootClock
        .now()
        .as_nanos()
        .try_into()
        .expect("boot time fits")
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn pause() {
    std::thread::sleep(Duration::from_millis(1));
}

#[test]
#[ignore = "selected only by the actual stdout-custody parent"]
fn stdout_writer_fixture_child() {
    let until: u64 = std::env::var("STDOUT_EOF_FIXTURE_UNTIL")
        .expect("parent cutoff")
        .parse()
        .expect("decimal cutoff");
    assert!(now() < until);
    let channel =
        rustix::io::fcntl_dupfd_cloexec(std::io::stdin(), 3).expect("owned inherited channel");
    let stdout =
        rustix::io::fcntl_dupfd_cloexec(std::io::stdout(), 3).expect("actual stdout writer");
    let handles = [stdout.as_fd()];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut ancillary = SendAncillaryBuffer::new(&mut space);
    assert!(ancillary.push(SendAncillaryMessage::ScmRights(&handles)));
    assert_eq!(
        net::sendmsg(
            &channel,
            &[IoSlice::new(b"writer")],
            &mut ancillary,
            SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
        )
        .expect("transfer real pipe writer"),
        6
    );
    std::io::stdout()
        .write_all(b"finite child tail\n")
        .expect("tail");
    std::io::stdout().flush().expect("flush tail");
    assert!(now() < until);
    // Avoid any additional libtest output. Parent retains the remaining writer.
    std::process::exit(0);
}

fn receive_writer(channel: &UnixDatagram, until: u64) -> io::Result<OwnedFd> {
    loop {
        let mut bytes = [0; 16];
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
        let mut ancillary = RecvAncillaryBuffer::new(&mut space);
        let result = net::recvmsg(
            channel,
            &mut [IoSliceMut::new(&mut bytes)],
            &mut ancillary,
            RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
        );
        if now() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match result {
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => pause(),
            Err(error) => return Err(error.into()),
            Ok(message) => {
                let mut handles = Vec::new();
                let mut rights = 0;
                let mut unexpected = false;
                for entry in ancillary.drain() {
                    match entry {
                        RecvAncillaryMessage::ScmRights(values) => {
                            rights += 1;
                            handles.extend(values);
                        }
                        _ => unexpected = true,
                    }
                }
                if message.bytes != 6
                    || bytes[..6] != *b"writer"
                    || message
                        .flags
                        .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC)
                    || unexpected
                    || rights != 1
                    || handles.len() != 1
                {
                    return Err(invalid("one exact inherited-channel writer required"));
                }
                return Ok(handles.pop().expect("one writer"));
            }
        }
    }
}

#[test]
fn wait_status_does_not_release_stdout_before_actual_eof() {
    let started = now();
    let work_until = started.checked_add(3_000_000_000).expect("work cutoff");
    let close_until = started.checked_add(5_000_000_000).expect("close cutoff");
    let directory = std::env::temp_dir().join(format!(
        "process-stdout-eof-{}-{started}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("fresh fixture directory");
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).expect("private directory");
    let journal = directory.join("admission");
    let mut owner = Supervisor::new(journal.clone());
    let (channel, child_channel) = UnixDatagram::pair().expect("inherited control");
    let mut child_channel = Some(child_channel);
    let mut held_writer = None;
    let mut retained_pidfd = None;
    let mut child_pid = 0;
    let mut terminal_before_work = false;
    let mut actual_would_block = false;
    let mut early_returned_status = false;
    let mut retained_until_eof = false;
    let mut publication_revoked = false;
    let mut actual_wait_echild = false;
    let mut repeated_stop_ok = false;
    let mut actual_stdout_eof = false;
    let mut status_success = false;
    let mut caller_eof_still_occupied = false;
    let result = (|| -> io::Result<()> {
        let mut command = Command::new(std::env::current_exe()?);
        command.args([
            "--ignored",
            "--exact",
            "stdout_writer_fixture_child",
            "--nocapture",
            "--test-threads=1",
        ]);
        command.env("STDOUT_EOF_FIXTURE_UNTIL", work_until.to_string());
        command.stdin(Stdio::from(OwnedFd::from(
            child_channel.take().expect("one inherited endpoint"),
        )));
        command.stderr(Stdio::null());
        let incarnation = owner
            .start(&mut command)
            .map_err(|_| invalid("actual child start"))?;
        child_pid = owner.child_pid().ok_or_else(|| invalid("retained PID"))?;
        let raw = child_pid.try_into().map_err(|_| invalid("PID range"))?;
        let pid = Pid::from_raw(raw).ok_or_else(|| invalid("positive PID"))?;
        retained_pidfd = Some(pidfd_open(pid, PidfdFlags::empty())?);
        held_writer = Some(receive_writer(&channel, work_until)?);
        loop {
            let mut fds = [PollFd::new(
                retained_pidfd.as_ref().expect("pidfd"),
                PollFlags::IN,
            )];
            let zero = Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            let count = poll(&mut fds, Some(&zero))?;
            if now() >= work_until {
                return Err(io::ErrorKind::TimedOut.into());
            }
            if count == 1 && fds[0].revents() == PollFlags::IN {
                terminal_before_work = true;
                break;
            }
            pause();
        }
        // Empty-buffer read0 is not an EOF witness.
        if owner.read_output(&mut [])? != 0 {
            return Err(invalid("empty read control"));
        }
        let mut bytes = [0; 4096];
        let mut total = 0;
        loop {
            if now() >= work_until {
                return Err(io::ErrorKind::TimedOut.into());
            }
            match owner.read_output(&mut bytes) {
                Ok(0) => return Err(invalid("writer held: EOF impossible")),
                Ok(count) => {
                    total += count;
                    if total > 8192 {
                        return Err(invalid("finite output bound"));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    actual_would_block = true;
                    break;
                }
                Err(error) => return Err(error),
            }
        }
        let status = owner.poll_reap()?;
        early_returned_status = status.is_some();
        status_success = status.is_some_and(|value| value.success());
        retained_until_eof = owner.occupied() && owner.child_pid() == Some(child_pid);
        publication_revoked = !owner.admits(incarnation);
        // WNOWAIT cannot consume the child; ECHILD proves Supervisor waited.
        actual_wait_echild = matches!(
            waitid(
                WaitId::Pid(pid),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            ),
            Err(rustix::io::Errno::CHILD)
        );
        repeated_stop_ok = owner.stop().is_ok() && owner.stop().is_ok();
        Ok(())
    })();
    drop(held_writer.take());
    drop(child_channel.take());
    drop(channel);
    if owner.occupied() && result.is_err() {
        let _ = owner.stop();
    }
    while owner.occupied() && now() < close_until {
        let mut bytes = [0; 4096];
        match owner.read_output(&mut bytes) {
            Ok(0) => {
                actual_stdout_eof = true;
                caller_eof_still_occupied = owner.occupied();
            }
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => break,
        }
        match owner.poll_reap() {
            Ok(Some(status)) => status_success = status.success(),
            Ok(None) => pause(),
            Err(_) => break,
        }
    }
    let fully_closed = !owner.occupied() && now() < close_until;
    if !fully_closed {
        let _ = owner.stop();
        // Never detach or hide an incomplete fixture behind an assertion.
        std::process::abort();
    }
    drop(owner);
    drop(retained_pidfd.take());
    fs::remove_file(journal).expect("remove admission fixture");
    fs::remove_dir(directory).expect("remove fixture directory");
    println!(
        "STDOUT_EOF_CONTROL child_pid={child_pid} work_until_ns={work_until} close_until_ns={close_until} completed_ns={} terminal_before_work={terminal_before_work} actual_would_block={actual_would_block} actual_wait_echild={actual_wait_echild} early_returned_status={early_returned_status} retained_until_eof={retained_until_eof} publication_revoked={publication_revoked} repeated_stop_ok={repeated_stop_ok} actual_stdout_eof={actual_stdout_eof} caller_eof_still_occupied={caller_eof_still_occupied} status_success={status_success} fully_closed={fully_closed} setup_ok={}",
        now(),
        result.is_ok()
    );
    std::io::stdout().flush().expect("control flush");
    assert!(
        result.is_ok()
            && terminal_before_work
            && actual_would_block
            && actual_wait_echild
            && fully_closed,
        "Actual stdout EOF fixture must reach its controlled boundary and close"
    );
    assert!(
        !early_returned_status
            && retained_until_eof
            && publication_revoked
            && repeated_stop_ok
            && actual_stdout_eof
            && caller_eof_still_occupied
            && status_success,
        "Supervisor must retain waited child stdout until actual EOF"
    );
}
