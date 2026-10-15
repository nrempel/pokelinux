//! One cooperative Samsung radioExternal transport process. Raw observations are
//! historical backend data, never decoded IMS state or a successful phone action.
use pokelinux_process_owner::{
    bootstrap::{BoundParent, WORKER_SELECTOR},
    deadline::{BootClock, Clock, Deadline},
};
use pokelinux_radio::{observation, owner::OwnerError, worker::TransportSession};
use rsbinder::{
    ProcessState, Proxy,
    hub::android_16::{BpServiceManager, IServiceManager},
};
use rustix::{
    fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat},
    io::Errno,
};
use std::{num::NonZeroUsize, path::PathBuf, time::Duration};

struct Endpoint {
    device: PathBuf,
    service: String,
    slot: i32,
    until: Duration,
}
impl Endpoint {
    fn arguments() -> Result<Self, ()> {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.len() != 9
            || args[0] != WORKER_SELECTOR
            || args[1] != "--device"
            || args[3] != "--service"
            || args[5] != "--slot"
            || args[7] != "--until-ns"
        {
            return Err(());
        }
        let device = PathBuf::from(&args[2]);
        let service = &args[4];
        let slot: i32 = args[6].parse().map_err(|_| ())?;
        let until: u64 = args[8].parse().map_err(|_| ())?;
        if !device.is_absolute()
            || args[2].len() > 256
            || service.is_empty()
            || service.len() > 128
            || !service
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
            || slot < 0
            || until == 0
        {
            return Err(());
        }
        Ok(Self {
            device,
            service: service.clone(),
            slot,
            until: Duration::from_nanos(until),
        })
    }
}
fn fatal() -> ! {
    std::process::exit(70)
}
fn main() {
    let endpoint = Endpoint::arguments().unwrap_or_else(|_| std::process::exit(64));
    let parent = BoundParent::from_stdin().unwrap_or_else(|_| std::process::exit(73));
    let output = std::io::stdout();
    let stat = fstat(&output).unwrap_or_else(|_| fatal());
    if FileType::from_raw_mode(stat.st_mode) != FileType::Fifo {
        fatal();
    }
    let flags = fcntl_getfl(&output).unwrap_or_else(|_| fatal());
    fcntl_setfl(&output, flags | OFlags::NONBLOCK).unwrap_or_else(|_| fatal());
    if endpoint.until <= BootClock.now() {
        fatal();
    }
    // All discovery/SDK/Binder work remains in this owned process. The selected
    // device/service is explicit; no fixture context-bootstrap or retry exists.
    let process = ProcessState::init(endpoint.device.to_str().unwrap_or_else(|| fatal()), 3)
        .unwrap_or_else(|_| fatal());
    ProcessState::start_thread_pool();
    let context = process.context_object().unwrap_or_else(|_| fatal());
    if !context.is_remote() || context.descriptor() != "android.os.IServiceManager" {
        fatal();
    }
    let manager = BpServiceManager::from_binder(context).unwrap_or_else(|| fatal());
    // One fixed current public interface method; not a multi-version adapter.
    let binder = manager
        .checkService(&endpoint.service)
        .unwrap_or_else(|_| fatal())
        .unwrap_or_else(|| fatal());
    let transport =
        pokelinux_radio::transport::RadioTransport::from_binder(binder).unwrap_or_else(|_| fatal());
    let mut session = TransportSession::new(NonZeroUsize::new(2).unwrap(), endpoint.slot);
    let _key = session
        .start(
            transport,
            "pokelinux-radio".into(),
            NonZeroUsize::new(8).unwrap(),
            Deadline::at(endpoint.until),
        )
        .unwrap_or_else(|_| fatal());
    let mut sent_revision = 0;
    let mut announced = false;
    loop {
        // Completion is joined only after is_finished. A permanently blocked
        // registration remains owned while this loop/parent retain control.
        let _completion = session.poll_registration();
        let snapshot = session.snapshot();
        if snapshot.revision > sent_revision
            && let Some(frame) =
                observation::from_snapshot(parent.identity(), &snapshot).unwrap_or_else(|_| fatal())
            && frame.revision > sent_revision
        {
            let bytes = frame.encode().unwrap_or_else(|_| fatal());
            // Every complete frame is smaller than Linux PIPE_BUF. Nonblock
            // writes are whole-frame or EAGAIN; no partial output queue or
            // detached writer can restore an obsolete snapshot later.
            match rustix::io::write(&output, &bytes) {
                Ok(count) if count == bytes.len() => {
                    sent_revision = frame.revision;
                    announced |= matches!(frame.kind, observation::Kind::Active { .. });
                }
                Err(Errno::AGAIN | Errno::INTR) => {}
                _ => fatal(),
            }
        }
        // Preserve the registration observation before draining callbacks that
        // may have arrived before its reply. Consume at most one ready event per
        // turn, and commit only through the existing owner publication gate.
        if announced {
            match session.prepare_callback_until(std::time::Instant::now()) {
                Ok(Some(event)) => match session.publish_callback(event) {
                    // Packet refusal preserves historical data; it does not fail the
                    // transport or authorize a new attempt. Terminal failures are
                    // still emitted from the current owner snapshot next turn.
                    Ok(_)
                    | Err(
                        OwnerError::Unsupported
                        | OwnerError::InvalidIndication
                        | OwnerError::Failed(_),
                    ) => {}
                    Err(_) => fatal(),
                },
                Ok(None) | Err(OwnerError::Failed(_)) => {}
                Err(_) => fatal(),
            }
        }
        // Keep session/watch/worker owned through terminal states. Parent Stop
        // or parent death ends this process; only its actual wait closes custody.
        std::thread::sleep(Duration::from_millis(2));
    }
}
