//! Actual lifecycle/storage/admin actor with an authored Source and raw test-only hook trace.
//! This observer records arguments only: no eligibility, notification or one-shot logic.
use pokelinux_lifecycle::{boottime_ns, session::Provider};
use pokelinux_session_owner::{Compositor, SourceOwner, runtime::drain_after};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    os::{fd::AsFd, unix::net::UnixDatagram},
    path::{Path, PathBuf},
};
struct Trace(UnixDatagram);
impl pokelinux_lifecycle::startup::Observer for Trace {
    fn poll(&mut self, source_until: Option<u64>, stopping: bool) -> io::Result<()> {
        let raw = serde_json::json!({
            "pid": std::process::id(),
            "observed_at_boottime_ns": boottime_ns(),
            "source_until": source_until,
            "stopping": stopping,
        });
        let packet = serde_json::to_vec(&raw)?;
        assert!(packet.len() <= 256);
        if self.0.send(&packet)? != packet.len() {
            return Err(io::ErrorKind::WriteZero.into());
        }
        Ok(())
    }
}
struct Custody {
    source: SourceOwner,
    until: u64,
    reported: bool,
}
impl Custody {
    fn drain(&mut self) {
        drain_after(&mut self.source, Ok(()), self.until).expect("actual Source drain");
        assert!(!self.source.occupied());
        let status = self
            .source
            .exit_status()
            .expect("actual Source parent wait");
        if !self.reported {
            self.reported = true;
            println!(
                "SOURCE_STATUS_ACTUAL_WAIT pid={} status={status:?} actual_wait_observed_at_boottime_ns={} original_until_ns={} stdout=DEVNULL stderr=DEVNULL",
                self.source.pid().as_raw_nonzero(),
                boottime_ns(),
                self.until
            );
        }
    }
}
impl Drop for Custody {
    fn drop(&mut self) {
        self.drain();
    }
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&args[1..]);
    }
    if args.len() == 1 && args[0] == "--compositor-bootstrap" {
        return pokelinux_session_owner::bootstrap_entry();
    }
    let [mode, root, source_until, close_until] = args.as_slice() else {
        return Err(io::ErrorKind::InvalidInput.into());
    };
    let mode = mode.to_str().ok_or(io::ErrorKind::InvalidInput)?;
    let parse = |v: &std::ffi::OsStr| {
        v.to_str()
            .ok_or(io::ErrorKind::InvalidInput)?
            .parse::<u64>()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
    };
    let root = Path::new(root);
    // Only this test-owned connected datagram endpoint is inherited on stdin.
    // Both original and duplicate are CLOEXEC before any real child is spawned.
    rustix::io::fcntl_setfd(io::stdin().as_fd(), rustix::io::FdFlags::CLOEXEC)?;
    let trace_fd = rustix::io::fcntl_dupfd_cloexec(io::stdin().as_fd(), 3)?;
    let trace_socket = UnixDatagram::from(trace_fd);
    trace_socket.set_nonblocking(true)?;
    let mut trace = Trace(trace_socket);
    let close_until = parse(close_until)?;
    let mut custody = if mode == "none" {
        None
    } else {
        if !matches!(mode, "gated_locked" | "gated_malformed" | "gated_stale") {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let spec = Compositor {
            helper: std::env::current_exe()?,
            executable: PathBuf::from(
                std::env::var_os("STARTUP_SOURCE_FIXTURE").ok_or(io::ErrorKind::InvalidInput)?,
            ),
            arguments: vec![mode.into(), root.to_string_lossy().into_owned()],
            environment: BTreeMap::new(),
            source: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        };
        Some(Custody {
            source: SourceOwner::spawn(spec, parse(source_until)?)?,
            until: close_until,
            reported: false,
        })
    };
    if let Some(c) = &custody {
        println!(
            "SOURCE_STATUS_ACTUAL_CHILD pid={}",
            c.source.pid().as_raw_nonzero()
        );
        io::stdout().flush()?;
    }
    let result = pokelinux_lifecycle::service::run_with_services(
        &root.join("runtime"),
        &root.join("policy"),
        &root.join("profile.json"),
        &std::env::current_exe()?,
        None,
        None,
        custody.as_mut().map(|c| &mut c.source as &mut dyn Provider),
        None,
        None,
        None,
        Some(&mut trace),
    );
    if let Some(c) = &mut custody {
        c.drain();
    }
    result
}
