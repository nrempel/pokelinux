//! One cooperative Samsung radioExternal transport process. Raw observations are
//! historical backend data, never decoded IMS state or a successful phone action.
use pokelinux_process_owner::{
    bootstrap::{BoundParent, WORKER_SELECTOR},
    deadline::{BootClock, Clock, Deadline},
};
use pokelinux_radio::{
    observation,
    operator::{self, ChildMessage, HistoryResponse, Operation, ParentMessage, RequestOutcome},
    owner::{OwnerError, Phase},
    request::Outcome,
    worker::TransportSession,
};
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
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.get(1).is_some_and(|s| s == "--intent-writer") {
        if args.len() != 6
            || args[0] != WORKER_SELECTOR
            || args[2] != "--journal"
            || args[4] != "--until-ns"
        {
            std::process::exit(64);
        }
        let until = args[5]
            .parse::<u64>()
            .unwrap_or_else(|_| std::process::exit(64));
        let mut parent = BoundParent::from_stdin().unwrap_or_else(|_| std::process::exit(73));
        let code = if operator::run_intent_writer(
            std::path::Path::new(&args[3]),
            Deadline::at(Duration::from_nanos(until)),
            &mut parent,
        )
        .is_ok()
        {
            0
        } else {
            70
        };
        std::process::exit(code);
    }
    let endpoint = Endpoint::arguments().unwrap_or_else(|_| std::process::exit(64));
    let mut parent = BoundParent::from_stdin().unwrap_or_else(|_| std::process::exit(73));
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
    let key = session
        .start(
            transport,
            "pokelinux-radio".into(),
            NonZeroUsize::new(8).unwrap(),
            Deadline::at(endpoint.until),
        )
        .unwrap_or_else(|_| fatal());
    let mut sent_revision = 0;
    let mut announced = false;
    let mut operation: Option<Operation> = None;
    let mut known_not_sent = false;
    let mut message_revision = 0u64;
    let mut last_request: Option<(RequestOutcome, Option<HistoryResponse>)> = None;
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
        // Main alone supplies one durable, exact permit on the retained role
        // channel. Decode/copy before fresh operation admission; no raw dispatch.
        match parent.receive_message(Deadline::at(endpoint.until)) {
            Ok(Some(bytes)) => {
                let ParentMessage::Dispatch {
                    operation: admitted,
                    config,
                } = ParentMessage::decode(&bytes).unwrap_or_else(|_| fatal())
                else {
                    fatal()
                };
                if operation.is_some()
                    || admitted.incarnation != parent.identity()
                    || endpoint.slot != 1
                {
                    fatal();
                }
                let current = session.snapshot();
                if !matches!(current.phase,Phase::Active{client_id} if client_id==admitted.client_id)
                {
                    fatal();
                }
                // Occupy the one permanent local attempt before preparation. A
                // late permit proves no native invocation occurred here; it does
                // not end this process/session or authorize another operation.
                operation = Some(admitted.clone());
                let deadline = Deadline::at(Duration::from_nanos(admitted.deadline_ns));
                if deadline.elapsed(BootClock.now()) {
                    known_not_sent = true;
                } else {
                    match session.prepare_add_pdn(&key, &config.as_add_pdn(), deadline) {
                        Ok(prepared) => {
                            // Allocations/encoding may consume time. The existing
                            // request thread also samples its clock at permission.
                            if deadline.elapsed(BootClock.now()) {
                                drop(prepared);
                                known_not_sent = true;
                            } else if session.submit_request(prepared).is_err() {
                                known_not_sent = true;
                            }
                        }
                        Err(_) => known_not_sent = true,
                    }
                }
            }
            Ok(None) => {}
            Err(_) => fatal(),
        }
        // Only finished native calls are joined. Blocked Binder work and its
        // endpoint/watch remain owned by this process until parent disposal.
        let _completed_request = session.poll_request();
        if let Some(op) = &operation {
            let snapshot = session.snapshot();
            let next = if let Some(receipt) = snapshot.request {
                let outcome = match receipt.outcome {
                    Outcome::Queued => None,
                    Outcome::NotSent => Some(RequestOutcome::NotSent),
                    Outcome::OutcomeUnknown => Some(RequestOutcome::OutcomeUnknown),
                };
                outcome.map(|outcome| {
                    let response = receipt.response.map(|r| HistoryResponse {
                        vendor_error: r.vendor_error(),
                        payload_hex: operator::hex(r.payload()),
                        ingress_sequence: r.ingress_sequence(),
                        received_at_ns: r
                            .received_at()
                            .as_nanos()
                            .try_into()
                            .unwrap_or_else(|_| fatal()),
                        publication_revision: r.revision(),
                    });
                    (outcome, response)
                })
            } else if known_not_sent {
                Some((RequestOutcome::NotSent, None))
            } else {
                None
            };
            if let Some(next) = next
                && last_request.as_ref() != Some(&next)
            {
                let publication = next.1.as_ref().map_or(0, |r| r.publication_revision);
                let revision = message_revision
                    .checked_add(1)
                    .unwrap_or_else(|| fatal())
                    .max(publication);
                let message = ChildMessage::Request {
                    operation: op.clone(),
                    revision,
                    outcome: next.0,
                    response: next.1.clone(),
                };
                let bytes = message.encode().unwrap_or_else(|_| fatal());
                // Exactly one send. Failure preserves uncertainty and ends the
                // role; no stored packet/retry can later restore authority.
                parent
                    .send_message(Deadline::at(endpoint.until), &bytes)
                    .unwrap_or_else(|_| fatal());
                message_revision = revision;
                last_request = Some(next);
            }
        }
        // Keep session/watch/worker owned through terminal states. Parent Stop
        // or parent death ends this process; only its actual wait closes custody.
        std::thread::sleep(Duration::from_millis(2));
    }
}
