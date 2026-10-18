//! Authored cached Fact visibility only; no kernel-connectivity evidence.
//! Actual local pipe custody only; no bootstrap or remote-cleanup claim.
use super::{Fact, LEASE, Service, metadata, owner};
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
    // Keep the real child nonterminal until the parent proves its ready cache.
    let control = UnixDatagram::from(channel);
    control
        .set_nonblocking(true)
        .expect("nonblocking exit gate");
    loop {
        assert!(now() < until, "original child exit gate cutoff");
        let mut gate = [0_u8; 5];
        match control.recv(&mut gate) {
            Ok(4) if gate[..4] == *b"exit" => break,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                pause()
            }
            value => panic!("one exact parent exit gate required: {value:?}"),
        }
    }
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
fn cached_snapshot_revokes_after_actual_wait_before_stdout_eof() {
    let started = now();
    let work_until = started.checked_add(3_000_000_000).expect("work cutoff");
    let close_until = started.checked_add(5_000_000_000).expect("close cutoff");
    let directory = std::env::temp_dir().join(format!(
        "service-admission-eof-{}-{started}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("fresh fixture directory");
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
        .expect("private fixture directory");
    let journal = directory.join("admission");
    // Both descriptors refer to actual namespace files. They deliberately name
    // the same existing netns: this test does not create or attest a virtual link,
    // invoke Service::start, drop capabilities, or claim worker bootstrap.
    let current = fs::File::open("/proc/thread-self/ns/net").expect("actual current namespace");
    let peer = fs::File::open("/proc/thread-self/ns/net").expect("second actual namespace handle");
    let namespace = metadata(&current).expect("actual namespace identity");
    let peer_namespace = metadata(&peer).expect("actual second namespace identity");
    let mut service = Service {
        supervisor: Supervisor::new(journal.clone()),
        identity: None,
        _current: current,
        _peer: peer,
        namespace,
        owner: owner::Owner::default(),
        observed: None,
        handshake: false,
        terminal: false,
        stopping: false,
        output_eof: false,
        reaped: false,
        exit_status: None,
        main_caps_dropped: false,
        start_until: work_until,
    };
    let (channel, child_channel) = UnixDatagram::pair().expect("actual inherited control");
    channel
        .set_nonblocking(true)
        .expect("bounded parent control");
    let mut child_channel = Some(child_channel);
    let mut held_writer = None;
    let mut retained_pidfd = None;
    let mut child_pid = 0;
    let mut pre_ready = false;
    let mut pre_live_pidfd = false;
    let mut pre_admitted = false;
    let mut actual_terminal_pidfd = false;
    let mut actual_wait_echild = false;
    let mut reap_returned_none = false;
    let mut actual_would_block = false;
    let mut publication_revoked = false;
    let mut retained_child_output = false;
    let mut namespace_files_live = false;
    let mut cache_fresh_around_snapshot = false;
    let mut cache_age_ns = u64::MAX;
    let mut snapshot_after = None;
    let mut private_terminal_after = false;
    let mut private_cache_cleared_after = false;
    let mut actual_stdout_eof = false;
    let mut status_success = false;
    let result = (|| -> io::Result<()> {
        let mut command = Command::new(std::env::current_exe()?);
        command.args([
            "--ignored",
            "--exact",
            "service::tests::stdout_writer_fixture_child",
            "--nocapture",
            "--test-threads=1",
        ]);
        command.env("STDOUT_EOF_FIXTURE_UNTIL", work_until.to_string());
        command.stdin(Stdio::from(OwnedFd::from(
            child_channel
                .take()
                .ok_or_else(|| invalid("one inherited endpoint"))?,
        )));
        command.stderr(Stdio::null());
        let incarnation = service
            .supervisor
            .start(&mut command)
            .map_err(|_| invalid("actual low-level Supervisor child start"))?;
        service.identity = Some(incarnation);
        child_pid = service
            .supervisor
            .child_pid()
            .ok_or_else(|| invalid("actual retained PID"))?;
        let raw = child_pid.try_into().map_err(|_| invalid("PID range"))?;
        let pid = Pid::from_raw(raw).ok_or_else(|| invalid("positive PID"))?;
        retained_pidfd = Some(pidfd_open(pid, PidfdFlags::empty())?);
        held_writer = Some(receive_writer(&channel, work_until)?);
        let mut fds = [PollFd::new(
            retained_pidfd
                .as_ref()
                .ok_or_else(|| invalid("retained pidfd"))?,
            PollFlags::IN,
        )];
        let zero = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        pre_live_pidfd = poll(&mut fds, Some(&zero))? == 0;
        pre_admitted = service.supervisor.admits(incarnation);
        // Explicitly authored component cache data, not observed kernel state.
        let fact = Fact {
            namespace,
            ifindex: 2,
            lifetime: 1,
            connected: true,
        };
        let observed_at = now();
        service.observed = Some((fact, observed_at));
        service.owner.observe(fact.connected, observed_at);
        service.handshake = true;
        let before = service.snapshot();
        pre_ready = before.ready
            && before.fact == Some(fact)
            && before.observed_at_ns == Some(observed_at)
            && before
                .owner
                .observed
                .is_some_and(|v| v.connected && v.observed_at_ns == observed_at)
            && before.worker == Some(incarnation)
            && before.occupied
            && before.namespace_handles_open
            && !before.terminal_uncertain
            && !before.owner.terminal_uncertain
            && !before.output_eof
            && !before.reaped
            && !before.stopping;
        if !(pre_ready && pre_live_pidfd && pre_admitted) {
            return Err(invalid("actual nonterminal ready pre-state required"));
        }
        if channel.send(b"exit")? != 4 {
            return Err(invalid("one original parent exit gate"));
        }
        loop {
            let mut fds = [PollFd::new(
                retained_pidfd
                    .as_ref()
                    .ok_or_else(|| invalid("retained pidfd"))?,
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
                actual_terminal_pidfd = true;
                break;
            }
            pause();
        }
        // A nonempty-buffer EAGAIN while our transferred writer is held proves
        // the read side remains owned without an actual EOF witness.
        let mut bytes = [0; 4096];
        let mut total = 0;
        loop {
            if now() >= work_until {
                return Err(io::ErrorKind::TimedOut.into());
            }
            match service.supervisor.read_output(&mut bytes) {
                Ok(0) => return Err(invalid("held stdout writer forbids EOF")),
                Ok(count) => {
                    total += count;
                    if total > 8192 {
                        return Err(invalid("finite child output bound"));
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
        reap_returned_none = service.supervisor.poll_reap()?.is_none();
        publication_revoked = !service.supervisor.admits(incarnation);
        actual_wait_echild = matches!(
            waitid(
                WaitId::Pid(pid),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            ),
            Err(rustix::io::Errno::CHILD)
        );
        if !(actual_terminal_pidfd
            && actual_wait_echild
            && reap_returned_none
            && actual_would_block
            && publication_revoked
            && service.supervisor.occupied())
        {
            return Err(invalid("actual waited-but-held-output boundary required"));
        }
        let before_snapshot = now();
        let after = service.snapshot();
        let after_snapshot = now();
        cache_fresh_around_snapshot = before_snapshot >= observed_at
            && after_snapshot >= before_snapshot
            && after_snapshot - observed_at < LEASE;
        cache_age_ns = after_snapshot.saturating_sub(observed_at);
        private_terminal_after = service.terminal;
        private_cache_cleared_after = service.observed.is_none();
        retained_child_output = service.supervisor.occupied()
            && service.supervisor.child_pid() == Some(child_pid)
            && after.occupied
            && !after.output_eof
            && !after.reaped
            && !after.stopping;
        namespace_files_live = metadata(&service._current)? == namespace
            && metadata(&service._peer)? == peer_namespace
            && after.namespace_handles_open;
        snapshot_after = Some(after);
        if !(cache_fresh_around_snapshot && retained_child_output && namespace_files_live) {
            return Err(invalid(
                "fresh cache and actual child/output/namespace custody required",
            ));
        }
        let mut one = [0_u8; 1];
        match service.supervisor.read_output(&mut one) {
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            _ => return Err(invalid("snapshot must retain the held output reader")),
        }
        Ok(())
    })();
    // Save behavior above, then release the actual writer and completely close
    // the real child/output fixture before any desired-behavior assertion.
    drop(held_writer.take());
    drop(child_channel.take());
    drop(channel);
    if service.supervisor.occupied() && result.is_err() {
        let _ = service.supervisor.stop();
    }
    while service.supervisor.occupied() && now() < close_until {
        let mut bytes = [0; 4096];
        match service.supervisor.read_output(&mut bytes) {
            Ok(0) => actual_stdout_eof = true,
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => break,
        }
        match service.supervisor.poll_reap() {
            Ok(Some(status)) => status_success = status.success(),
            Ok(None) => pause(),
            Err(_) => break,
        }
    }
    let fully_closed = !service.supervisor.occupied() && now() < close_until;
    if !fully_closed {
        let _ = service.supervisor.stop();
        std::process::abort();
    }
    drop(service);
    drop(retained_pidfd.take());
    fs::remove_file(journal).expect("remove actual admission journal");
    fs::remove_dir(directory).expect("remove private fixture directory");
    let after_ready = snapshot_after.is_some_and(|s| s.ready);
    let after_fact_present = snapshot_after.is_some_and(|s| s.fact.is_some());
    let after_at_present = snapshot_after.is_some_and(|s| s.observed_at_ns.is_some());
    let after_owner_observed_present = snapshot_after.is_some_and(|s| s.owner.observed.is_some());
    let after_terminal = snapshot_after.is_some_and(|s| s.terminal_uncertain);
    let after_owner_terminal = snapshot_after.is_some_and(|s| s.owner.terminal_uncertain);
    println!(
        "SERVICE_ADMISSION_EOF_CONTROL child_pid={child_pid} work_until_ns={work_until} close_until_ns={close_until} completed_ns={} pre_ready={pre_ready} pre_live_pidfd={pre_live_pidfd} pre_admitted={pre_admitted} actual_terminal_pidfd={actual_terminal_pidfd} actual_wait_echild={actual_wait_echild} reap_returned_none={reap_returned_none} actual_would_block={actual_would_block} publication_revoked={publication_revoked} retained_child_output={retained_child_output} namespace_files_live={namespace_files_live} cache_fresh_around_snapshot={cache_fresh_around_snapshot} cache_age_ns={cache_age_ns} after_ready={after_ready} after_fact_present={after_fact_present} after_at_present={after_at_present} after_owner_observed_present={after_owner_observed_present} after_terminal={after_terminal} after_owner_terminal={after_owner_terminal} private_terminal_after={private_terminal_after} private_cache_cleared_after={private_cache_cleared_after} actual_stdout_eof={actual_stdout_eof} status_success={status_success} fully_closed={fully_closed} setup_ok={}",
        now(),
        result.is_ok()
    );
    std::io::stdout().flush().expect("actual control flush");
    assert!(
        result.is_ok()
            && pre_ready
            && pre_live_pidfd
            && pre_admitted
            && actual_terminal_pidfd
            && actual_wait_echild
            && reap_returned_none
            && actual_would_block
            && publication_revoked
            && retained_child_output
            && namespace_files_live
            && cache_fresh_around_snapshot
            && actual_stdout_eof
            && status_success
            && fully_closed,
        "Actual service cached-fact fixture must reach held-writer actual wait boundary and close"
    );
    assert!(
        !after_ready
            && !after_fact_present
            && !after_at_present
            && !after_owner_observed_present
            && after_terminal
            && after_owner_terminal
            && private_terminal_after
            && private_cache_cleared_after,
        "Service snapshot must revoke cached visibility at actual wait before stdout EOF"
    );
}
