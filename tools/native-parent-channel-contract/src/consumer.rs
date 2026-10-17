//! Independent public retained-channel consumer. This intentionally references
//! missing production methods directly; there are no future impls/traits/shims.
#![forbid(unsafe_code)]
mod common;
mod raw_controls;
use common::*;
use pokelinux_process_owner::{
    bootstrap::{BoundParent, MAX_MESSAGE_BYTES, WORKER_SELECTOR},
    supervisor::{ProcessIdentity, StartError, Supervisor},
};
use raw_controls as raw;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

fn emit(text: &str) -> io::Result<()> {
    writeln!(io::stdout(), "{text}")?;
    io::stdout().flush()
}
fn receive_parent(
    owner: &mut Supervisor,
    identity: ProcessIdentity,
    until: u64,
) -> io::Result<Vec<u8>> {
    loop {
        before(until)?;
        if let Some(bytes) = owner.receive_message(identity, deadline(until))? {
            return Ok(bytes);
        }
        pause();
    }
}
fn receive_worker(bound: &mut BoundParent, until: u64) -> io::Result<Vec<u8>> {
    loop {
        before(until)?;
        if let Some(bytes) = bound.receive_message(deadline(until))? {
            return Ok(bytes);
        }
        pause();
    }
}
fn other(identity: ProcessIdentity) -> ProcessIdentity {
    let mut bytes = *identity.as_bytes();
    bytes[0] ^= 1;
    ProcessIdentity::from_bytes(bytes)
}
fn number(text: &str) -> io::Result<u64> {
    text.parse().map_err(|_| io::ErrorKind::InvalidInput.into())
}
fn hold(bound: BoundParent) -> ! {
    // Keep exact parent references and !Send guard on the worker main thread.
    let _parent = bound;
    loop {
        std::thread::park();
    }
}
fn child(args: &[String]) -> io::Result<()> {
    require(args.len() >= 4, "worker mode/until/cleanup missing")?;
    let mode = args[0].as_str();
    let operation_until = number(&args[1])?;
    let cleanup_until = number(&args[2])?;
    // Only hostile injection workers retain a test duplicate of inherited fd0.
    // It is never returned by BoundParent or any production API.
    let hostile = if mode == "inject" {
        Some(rustix::io::fcntl_dupfd_cloexec(io::stdin(), 3)?)
    } else {
        None
    };
    let mut bound = BoundParent::from_stdin()?;
    require(
        std::fs::read_link("/proc/self/fd/0")? == std::path::Path::new("/dev/null"),
        "original fd0 still inherited",
    )?;
    match mode {
        "roundtrip" => {
            refusal(
                bound.send_message(deadline(operation_until), &[]),
                io::ErrorKind::InvalidInput,
                "worker empty payload",
            )?;
            refusal(
                bound.send_message(deadline(operation_until), &vec![0; MAX_MESSAGE_BYTES + 1]),
                io::ErrorKind::InvalidInput,
                "worker oversized public send",
            )?;
            emit("ROUNDTRIP-BODY")?;
            let mut first = receive_worker(&mut bound, operation_until)?;
            require(first == b"k", "worker received wrong original copy")?;
            // Copy leaves this call before source overwrite and destruction.
            bound.send_message(deadline(operation_until), &first)?;
            first.fill(0x66);
            drop(first);
            let maximum = receive_worker(&mut bound, operation_until)?;
            require(
                maximum == vec![0x5a; MAX_MESSAGE_BYTES],
                "worker maximum packet changed",
            )?;
            bound.send_message(deadline(operation_until), &maximum)?;
            emit("ROUNDTRIP-COPIES-COMPLETE")?;
            hold(bound)
        }
        "inject" => {
            require(args.len() == 5, "inject case/fifo missing")?;
            let case = &args[3];
            raw::inject(
                case,
                hostile.as_ref().ok_or(io::ErrorKind::InvalidData)?,
                Path::new(&args[4]),
                operation_until,
                cleanup_until,
                MAX_MESSAGE_BYTES,
            )?;
            emit(&format!("INJECTED {case}"))?;
            drop(hostile);
            hold(bound)
        }
        "victim" => {
            // args here are normalized by main for the authored raw parent.
            let case = &args[3];
            let descriptors_before = raw::fd_count()?;
            emit(&format!("VICTIM-POST-GO {case}"))?;
            loop {
                before(operation_until)?;
                match bound.receive_message(deadline(operation_until)) {
                    Ok(None) => pause(),
                    Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                        // Observe closure while guard/process are still alive.
                        require(
                            raw::fd_count()? <= descriptors_before,
                            "worker leaked rejected rights",
                        )?;
                        emit(&format!("VICTIM-REFUSED {case} InvalidData no-fd-growth"))?;
                        return Ok(());
                    }
                    other => {
                        return Err(io::Error::other(format!(
                            "worker accepted hostile packet: {other:?}"
                        )));
                    }
                }
            }
        }
        "expire" => {
            bound.send_message(deadline(operation_until), b"queued-before-original-expiry")?;
            emit("EXPIRY-ARMED")?;
            until(plus(operation_until, 25_000_000)?, cleanup_until)?;
            refusal(
                bound.send_message(deadline(operation_until), b"expired-worker-send"),
                io::ErrorKind::TimedOut,
                "worker original send deadline",
            )?;
            refusal(
                bound.receive_message(deadline(operation_until)),
                io::ErrorKind::TimedOut,
                "worker original receive deadline",
            )?;
            emit("EXPIRY-WORKER-REFUSED-SEND-AND-RECEIVE")?;
            hold(bound)
        }
        "hold" => {
            bound.send_message(deadline(operation_until), b"queued-status-before-stop")?;
            emit("HELD-WITHOUT-RECEIVE")?;
            hold(bound)
        }
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}

fn roundtrip(runtime: &Path) -> io::Result<()> {
    private_dir(runtime)?;
    let began = now();
    let operation_until = plus(began, 7_000_000_000)?;
    let cleanup_until = plus(began, 9_000_000_000)?;
    let mut owner = Supervisor::new(runtime.join("admission"));
    let mut worker = command(
        "roundtrip",
        &[
            operation_until.to_string(),
            cleanup_until.to_string(),
            "unused".into(),
        ],
    )?;
    let identity = start(&mut owner, &mut worker, cleanup_until)?;
    // After-start+3.05s is strictly later than Prepared's immutable 3s cutoff.
    let after_started_at = now();
    let result = (|| -> io::Result<()> {
        let after_bootstrap_until = plus(after_started_at, 3_050_000_000)?;
        refusal(
            owner.send_message(identity, deadline(operation_until), b"before-ready"),
            io::ErrorKind::PermissionDenied,
            "initializing channel dispatch",
        )?;
        refusal(
            owner.receive_message(identity, deadline(operation_until)),
            io::ErrorKind::PermissionDenied,
            "initializing channel receive",
        )?;
        ready(&mut owner, identity, operation_until)?;
        let mut output = Vec::new();
        require(
            line(&mut owner, &mut output, operation_until)? == "ROUNDTRIP-BODY\n",
            "roundtrip body absent",
        )?;
        require(
            owner
                .receive_message(identity, deadline(operation_until))?
                .is_none(),
            "idle receive not empty",
        )?;
        refusal(
            owner.send_message(identity, deadline(operation_until), &[]),
            io::ErrorKind::InvalidInput,
            "parent empty payload",
        )?;
        refusal(
            owner.send_message(
                identity,
                deadline(operation_until),
                &vec![0; MAX_MESSAGE_BYTES + 1],
            ),
            io::ErrorKind::InvalidInput,
            "parent oversized public send",
        )?;
        until(after_bootstrap_until, operation_until)?;
        let sent_at = now();
        let mut original = b"k".to_vec();
        owner.send_message(identity, deadline(operation_until), &original)?;
        original.fill(0xcc);
        drop(original);
        let received = receive_parent(&mut owner, identity, operation_until)?;
        require(
            received == b"k",
            "send/receive did not own original copied bytes",
        )?;
        let mut maximum = vec![0x5a; MAX_MESSAGE_BYTES];
        owner.send_message(identity, deadline(operation_until), &maximum)?;
        maximum.fill(0x99);
        drop(maximum);
        let reply = receive_parent(&mut owner, identity, operation_until)?;
        require(
            reply == vec![0x5a; MAX_MESSAGE_BYTES],
            "maximum packet changed",
        )?;
        require(
            received == b"k",
            "second receive mutated first owned packet",
        )?;
        require(
            line(&mut owner, &mut output, operation_until)? == "ROUNDTRIP-COPIES-COMPLETE\n",
            "worker copy barrier absent",
        )?;
        owner.stop()?;
        killed(reap(&mut owner, cleanup_until)?)?;
        journal_unchanged(&mut owner, &mut worker, runtime, identity)?;
        println!(
            "G1 retained-actual-pair copied-roundtrip bytes1..8192 startup-cutoff-passed sent_at_ns={sent_at} app_until_ns={operation_until}"
        );
        Ok(())
    })();
    dispose(&mut owner, cleanup_until)?;
    result
}

fn parent_hostile(runtime: &Path) -> io::Result<()> {
    private_dir(runtime)?;
    for case in ["wrong-sender", "rights", "many-rights", "oversize"] {
        let case_runtime = runtime.join(case);
        private_dir(&case_runtime)?;
        let began = now();
        let operation_until = plus(began, 4_000_000_000)?;
        let cleanup_until = plus(began, 6_000_000_000)?;
        let fifo = case_runtime.join("rights-fifo");
        let reader = raw::fifo_reader(&fifo)?;
        let mut owner = Supervisor::new(case_runtime.join("admission"));
        let mut worker = command(
            "inject",
            &[
                operation_until.to_string(),
                cleanup_until.to_string(),
                case.into(),
                fifo.to_str().ok_or(io::ErrorKind::InvalidInput)?.into(),
            ],
        )?;
        let identity = start(&mut owner, &mut worker, cleanup_until)?;
        let result = (|| -> io::Result<()> {
            ready(&mut owner, identity, operation_until)?;
            let mut output = Vec::new();
            require(
                line(&mut owner, &mut output, operation_until)? == format!("INJECTED {case}\n"),
                "injection was not submitted",
            )?;
            let has_rights = case == "rights" || case == "many-rights";
            if has_rights {
                raw::rights_queued(&reader)?;
            }
            let descriptors_before = raw::fd_count()?;
            refusal(
                owner.receive_message(identity, deadline(operation_until)),
                io::ErrorKind::InvalidData,
                "parent hostile record refusal",
            )?;
            require(
                raw::fd_count()? <= descriptors_before,
                "parent leaked rejected rights",
            )?;
            if has_rights {
                raw::rights_closed(&reader)?;
            }
            owner.stop()?;
            killed(reap(&mut owner, cleanup_until)?)?;
            journal_unchanged(&mut owner, &mut worker, &case_runtime, identity)?;
            println!("G2 parent-refused actual-{case} received-rights-closed actual-wait9");
            Ok(())
        })();
        dispose(&mut owner, cleanup_until)?;
        result?;
    }
    Ok(())
}

fn worker_hostile(runtime: &Path) -> io::Result<()> {
    private_dir(runtime)?;
    for case in ["wrong-sender", "rights", "many-rights", "oversize"] {
        let case_runtime = runtime.join(case);
        private_dir(&case_runtime)?;
        let began = now();
        let operation_until = plus(began, 4_000_000_000)?;
        let cleanup_until = plus(began, 6_000_000_000)?;
        let fifo = case_runtime.join("rights-fifo");
        let reader = raw::fifo_reader(&fifo)?;
        let (channel, mut child, identity) =
            raw::manual_child(case, operation_until, cleanup_until)?;
        let result = (|| -> io::Result<()> {
            let mut output = Vec::new();
            require(
                raw::child_line(&mut child, &mut output, operation_until)?
                    == format!("VICTIM-POST-GO {case}\n"),
                "actual victim body did not enter after GO",
            )?;
            raw::inject(
                case,
                &channel,
                &fifo,
                operation_until,
                cleanup_until,
                MAX_MESSAGE_BYTES,
            )?;
            require(
                raw::child_line(&mut child, &mut output, operation_until)?
                    == format!("VICTIM-REFUSED {case} InvalidData no-fd-growth\n"),
                "worker did not report exact packet refusal while guard alive",
            )?;
            // This check is extra closure evidence; the live child's fd-table
            // check above is what distinguishes API closure from process exit.
            if case == "rights" || case == "many-rights" {
                raw::rights_closed(&reader)?;
            }
            let status = child.wait_before(operation_until)?;
            require(status.code() == Some(0), "worker hostile control failed")?;
            println!(
                "G3 worker-refused actual-{case} identity={} actual-wait0",
                identity.hex()
            );
            Ok(())
        })();
        child.close()?;
        result?;
    }
    Ok(())
}

fn expiry(runtime: &Path) -> io::Result<()> {
    private_dir(runtime)?;
    let began = now();
    let operation_until = plus(began, 1_000_000_000)?;
    let cleanup_until = plus(began, 4_000_000_000)?;
    let mut owner = Supervisor::new(runtime.join("admission"));
    let mut worker = command(
        "expire",
        &[
            operation_until.to_string(),
            cleanup_until.to_string(),
            "unused".into(),
        ],
    )?;
    let identity = start(&mut owner, &mut worker, cleanup_until)?;
    let result = (|| -> io::Result<()> {
        ready(&mut owner, identity, operation_until)?;
        let mut output = Vec::new();
        require(
            line(&mut owner, &mut output, operation_until)? == "EXPIRY-ARMED\n",
            "expiry sender not armed",
        )?;
        // No timer callback or poll_worker call will service either app deadline.
        // Queue a reciprocal record before the unchanged original deadline.
        owner.send_message(
            identity,
            deadline(operation_until),
            b"queued-parent-before-expiry",
        )?;
        until(plus(operation_until, 25_000_000)?, cleanup_until)?;
        refusal(
            owner.receive_message(identity, deadline(operation_until)),
            io::ErrorKind::TimedOut,
            "parent consumed queued status after original expiry",
        )?;
        refusal(
            owner.send_message(identity, deadline(operation_until), b"expired-parent-send"),
            io::ErrorKind::TimedOut,
            "parent original send deadline",
        )?;
        require(
            line(&mut owner, &mut output, cleanup_until)?
                == "EXPIRY-WORKER-REFUSED-SEND-AND-RECEIVE\n",
            "worker expiry checks depended on timer delivery",
        )?;
        require(owner.occupied(), "operation timeout detached worker")?;
        owner.stop()?;
        killed(reap(&mut owner, cleanup_until)?)?;
        journal_unchanged(&mut owner, &mut worker, runtime, identity)?;
        println!(
            "G4 original-expiry both-directions queued-record-refusal no-timer-delivery app_until_ns={operation_until} sampled_ns={}",
            now()
        );
        Ok(())
    })();
    dispose(&mut owner, cleanup_until)?;
    result
}

fn backpressure_stop(runtime: &Path) -> io::Result<()> {
    private_dir(runtime)?;
    let began = now();
    let operation_until = plus(began, 5_000_000_000)?;
    let cleanup_until = plus(began, 7_000_000_000)?;
    let mut owner = Supervisor::new(runtime.join("admission"));
    let mut worker = command(
        "hold",
        &[
            operation_until.to_string(),
            cleanup_until.to_string(),
            "unused".into(),
        ],
    )?;
    let identity = start(&mut owner, &mut worker, cleanup_until)?;
    let result = (|| -> io::Result<()> {
        ready(&mut owner, identity, operation_until)?;
        let mut output = Vec::new();
        require(
            line(&mut owner, &mut output, operation_until)? == "HELD-WITHOUT-RECEIVE\n",
            "held worker absent",
        )?;
        refusal(
            owner.send_message(
                other(identity),
                deadline(operation_until),
                b"wrong-identity",
            ),
            io::ErrorKind::PermissionDenied,
            "wrong caller identity dispatch",
        )?;
        refusal(
            owner.receive_message(other(identity), deadline(operation_until)),
            io::ErrorKind::PermissionDenied,
            "wrong caller identity receive",
        )?;
        // Distinct fixture records to reach actual kernel EAGAIN, not a retry of
        // a failed application operation. No send is retried after WouldBlock.
        let mut records = 0usize;
        let bytes = vec![0x42; MAX_MESSAGE_BYTES];
        loop {
            before(operation_until)?;
            match owner.send_message(identity, deadline(operation_until), &bytes) {
                Ok(()) => {
                    records += 1;
                    require(
                        records <= 2048,
                        "actual socket queue failed to backpressure within fixed cap",
                    )?;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                other => return Err(io::Error::other(format!("actual bounded send: {other:?}"))),
            }
        }
        require(records > 0, "no successful current-identity channel send")?;
        owner.stop()?;
        require(
            owner.occupied() && !owner.admits(identity),
            "Stop lost owned child or retained admission",
        )?;
        refusal(
            owner.send_message(identity, deadline(operation_until), b"after-stop"),
            io::ErrorKind::PermissionDenied,
            "Stop dispatch fence before wait",
        )?;
        refusal(
            owner.receive_message(identity, deadline(operation_until)),
            io::ErrorKind::PermissionDenied,
            "Stop queued receive fence before wait",
        )?;
        require(
            owner.start_worker(&mut worker) == Err(StartError::Occupied),
            "Stop released unwaited capacity",
        )?;
        killed(reap(&mut owner, cleanup_until)?)?;
        refusal(
            owner.send_message(identity, deadline(operation_until), b"after-wait"),
            io::ErrorKind::PermissionDenied,
            "reaped dispatch fence",
        )?;
        refusal(
            owner.receive_message(identity, deadline(operation_until)),
            io::ErrorKind::PermissionDenied,
            "reaped receive fence",
        )?;
        journal_unchanged(&mut owner, &mut worker, runtime, identity)?;
        println!(
            "G5 actual-EAGAIN records={records} wrong-identity-refusal Stop-before-wait queued-refusal actual-wait9 durable-refusal"
        );
        Ok(())
    })();
    dispose(&mut owner, cleanup_until)?;
    result
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|value| value == "--raw-sender") {
        return raw::sender_entry();
    }
    if args.first().is_some_and(|value| value == WORKER_SELECTOR) {
        // The reciprocal raw parent uses selector,victim,case,until,cleanup;
        // normalize those arguments without changing any original deadline.
        if args.get(1).is_some_and(|value| value == "victim") {
            require(args.len() == 5, "manual victim arguments")?;
            return child(&[
                "victim".into(),
                args[3].clone(),
                args[4].clone(),
                args[2].clone(),
            ]);
        }
        return child(&args[1..]);
    }
    require(
        args.len() == 1,
        "usage: parent-channel-consumer NEW_PRIVATE_RUNTIME",
    )?;
    require(MAX_MESSAGE_BYTES == 8192, "frozen maximum changed")?;
    let runtime = PathBuf::from(&args[0]);
    private_dir(&runtime)?;
    roundtrip(&runtime.join("g1-roundtrip"))?;
    parent_hostile(&runtime.join("g2-parent-hostile"))?;
    worker_hostile(&runtime.join("g3-worker-hostile"))?;
    expiry(&runtime.join("g4-original-expiry"))?;
    backpressure_stop(&runtime.join("g5-backpressure-stop"))?;
    println!("RETAINED-CHANNEL-CONSUMER all-five-groups actual-owned-processes-closed");
    Ok(())
}
