//! Main owns admission and ordering; regular-file work belongs to the finite writer.
use super::{Client, Config, DISPOSAL, Service, TURN, event, now, ns, private_directory};
use pokelinux_process_owner::{
    bootstrap::{WorkerCommand, WorkerState},
    deadline::Deadline,
    supervisor::{AdmissionReceipt, ProcessIdentity, StartError, Supervisor},
};
use pokelinux_radio::{
    operator::{
        self, Admit, AdmitReply, CanonicalConfig, ChildMessage, Operation, ParentMessage,
        RequestHistory, RequestOutcome, SyncReceipt,
    },
    receiver::ReceiverFailure,
    status::{Command, Connection, Listener, Phase, Reply, Status},
};
use rustix::fs::FileType;
use std::{
    fs,
    io::{self, Read},
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum WriterStage {
    Initializing,
    WaitingJob,
    AwaitAck,
    AwaitReap,
    Reaped,
}

pub(super) struct Control {
    writer: Supervisor,
    writer_identity: ProcessIdentity,
    writer_pid: u32,
    radio_launch: AdmissionReceipt,
    writer_launch: AdmissionReceipt,
    config: CanonicalConfig,
    payload_length: u32,
    next_intent: [u8; 16],
    until: Duration,
    stage: WriterStage,
    ready: bool,
    pub(super) client_id: Option<i32>,
    operation: Option<Operation>,
    admitted_at_ns: u64,
    synced: Option<SyncReceipt>,
    writer_waited_at: Option<Duration>,
    permit_attempted: bool,
    blocked: bool,
    worker_revision: u64,
}
impl Control {
    pub(super) fn occupied(&self) -> bool {
        self.writer.occupied()
    }
    pub(super) fn fence(&mut self) {
        self.blocked = true;
    }
    pub(super) fn stop(&mut self) -> io::Result<()> {
        self.blocked = true;
        self.writer.stop()
    }
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn marker(journal: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", journal.display()))
}
fn existing_marker(journal: &Path) -> io::Result<bool> {
    // This check precedes configuration, random input, spawn and Binder lookup.
    for path in [
        journal.to_path_buf(),
        marker(journal, ".writer"),
        marker(journal, ".add-pdn"),
    ] {
        match fs::symlink_metadata(path) {
            Ok(_) => return Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(false)
}
fn refused() {
    event("\"event\":\"start-refused\",\"journal_retained\":true,\"child_spawned\":false,\"reason\":\"registration-uncertain\"".into());
}
fn dispose_setup(owner: &mut Supervisor, writer: &mut Supervisor) {
    let until = now() + DISPOSAL;
    let _ = owner.stop();
    let _ = writer.stop();
    while owner.occupied() || writer.occupied() {
        let _ = owner.poll_reap();
        let _ = writer.poll_reap();
        if now() >= until {
            std::process::abort();
        }
        std::thread::sleep(TURN);
    }
}
impl Service {
    fn signal_stop(&mut self) {
        if pokelinux_kernel_identity::termination_requested() && !self.closing {
            self.retire(ReceiverFailure::Cancelled, true);
        }
    }
    fn reply(&self, admitted: bool, reason: &str) -> AdmitReply {
        AdmitReply {
            schema: 1,
            admitted,
            reason: reason.into(),
            incarnation: self.identity.hex(),
            attempt: 1,
            request_revision: self
                .state
                .request_history
                .as_ref()
                .map_or(0, |v| v.request_revision),
        }
    }
    fn admit(&mut self, command: &Admit, source: &Connection) -> AdmitReply {
        if self.control.operation.is_some() {
            return self.reply(false, "operation-slot-occupied");
        }
        if command.incarnation != self.identity.hex() {
            return self.reply(false, "stale-incarnation");
        }
        if command.slot != self.slot || self.slot != 1 {
            return self.reply(false, "unsupported-route");
        }
        if command.payload_sha256 != operator::hex(&self.control.config.digest()) {
            return self.reply(false, "payload-mismatch");
        }
        if command.deadline_ns > ns(self.control.until) {
            return self.reply(false, "deadline-outside-session");
        }
        if ns(now()) >= command.deadline_ns {
            return self.reply(false, "expired-before-admission");
        }
        if self.closing
            || self.cleanup_until.is_some()
            || self.control.blocked
            || !self.control.ready
            || self.state.phase != Phase::ObservedActive
            || self.state.failure.is_some()
            || self.control.stage != WriterStage::WaitingJob
            || !self.owner.admits(self.identity)
            || !self.control.writer.admits(self.control.writer_identity)
        {
            return self.reply(false, "session-unavailable");
        }
        let Some(client_id) = self.control.client_id.filter(|id| *id >= 0) else {
            return self.reply(false, "session-unavailable");
        };
        // The exact socket origin must still be live at final admission. This
        // authorizes this one command, not an app grant or future retry.
        if !source.is_live().unwrap_or(false) {
            return self.reply(false, "origin-unavailable");
        }
        self.signal_stop();
        if self.closing {
            return self.reply(false, "session-unavailable");
        }
        let admitted = now();
        if admitted >= self.control.until || ns(admitted) >= command.deadline_ns {
            return self.reply(false, "expired-before-admission");
        }
        let operation = Operation {
            incarnation: self.identity,
            intent: self.control.next_intent,
            deadline_ns: command.deadline_ns,
            client_id,
            payload_sha256: self.control.config.digest(),
            payload_length: self.control.payload_length,
        };
        self.state.request_history = Some(RequestHistory {
            request_revision: 1,
            intent: operator::hex(&operation.intent),
            incarnation: self.identity.hex(),
            attempt: 1,
            serial: 1,
            client_id,
            request: 105,
            slot: 1,
            payload_sha256: operator::hex(&operation.payload_sha256),
            deadline_ns: operation.deadline_ns,
            outcome: RequestOutcome::OutcomeUnknown,
            response: None,
        });
        self.control.operation = Some(operation.clone());
        self.control.admitted_at_ns = ns(admitted);
        self.control.stage = WriterStage::AwaitAck;
        // Reservation survives every outcome, including a failed or uncertain
        // send. The durable launch markers already prohibit a successor.
        let result = ParentMessage::Write {
            operation: operation.clone(),
            writer: self.control.writer_identity,
        }
        .encode()
        .and_then(|bytes| {
            self.control.writer.send_message(
                self.control.writer_identity,
                Deadline::at(Duration::from_nanos(operation.deadline_ns)),
                &bytes,
            )
        });
        if result.is_err() {
            self.retire(ReceiverFailure::InternalFailure, false);
        }
        self.reply(true, "admitted-pending-durability")
    }
    pub(super) fn reap_writer(&mut self) {
        match self.control.writer.poll_reap() {
            Ok(Some(status)) => {
                let waited = now();
                let cutoff = self.cleanup_until;
                if cutoff.is_some_and(|until| waited >= until) {
                    self.fault = true;
                }
                event(format!(
                    "\"event\":\"writer-reap\",\"writer_pid\":{},\"writer_incarnation\":\"{}\",\"incarnation\":\"{}\",\"wait_signal\":{},\"wait_code\":{},\"waited_at_ns\":{},\"original_cleanup_until_ns\":{},\"occupied\":{},\"admitted\":{}",
                    self.control.writer_pid,
                    self.control.writer_identity.hex(),
                    self.identity.hex(),
                    status.signal().map_or("null".into(), |v| v.to_string()),
                    status.code().map_or("null".into(), |v| v.to_string()),
                    ns(waited),
                    cutoff.map_or("null".into(), |v| ns(v).to_string()),
                    self.control.writer.occupied(),
                    self.control.writer.admits(self.control.writer_identity)
                ));
                let expected = self.control.stage == WriterStage::AwaitReap
                    && self.control.synced.is_some()
                    && status.code() == Some(0)
                    && !self.control.blocked
                    && self.cleanup_until.is_none();
                self.control.stage = WriterStage::Reaped;
                if expected {
                    self.control.writer_waited_at = Some(waited);
                } else if self.cleanup_until.is_none() {
                    self.retire(ReceiverFailure::InternalFailure, false);
                }
            }
            Ok(None) => {}
            Err(_) => {
                self.fault = true;
                self.retire(ReceiverFailure::InternalFailure, true);
            }
        }
    }
    fn poll_control(&mut self) -> io::Result<()> {
        if self.cleanup_until.is_some() || self.closing {
            return Ok(());
        }
        if self.control.stage == WriterStage::Initializing {
            match self.control.writer.poll_worker()? {
                WorkerState::Ready => self.control.stage = WriterStage::WaitingJob,
                WorkerState::Initializing => {}
                WorkerState::Terminal => return Err(invalid()),
            }
        }
        if !self.control.ready
            && self.owner.admits(self.identity)
            && self.control.stage == WriterStage::WaitingJob
            && self.control.writer.admits(self.control.writer_identity)
        {
            self.control.ready = true;
            let r = self.control.radio_launch;
            let w = self.control.writer_launch;
            event(format!(
                "\"event\":\"control-ready\",\"main_pid\":{},\"worker_pid\":{},\"writer_pid\":{},\"incarnation\":\"{}\",\"writer_incarnation\":\"{}\",\"until_ns\":{},\"config_sha256\":\"{}\",\"payload_sha256\":\"{}\",\"payload_length\":{},\"radio_launch_sync_ns\":{},\"radio_launch_device\":{},\"radio_launch_inode\":{},\"radio_launch_uid\":{},\"radio_launch_mode\":{},\"writer_launch_sync_ns\":{},\"writer_launch_device\":{},\"writer_launch_inode\":{},\"writer_launch_uid\":{},\"writer_launch_mode\":{},\"retained_worker_channel\":true,\"retained_writer_channel\":true",
                std::process::id(),
                self.worker_pid,
                self.control.writer_pid,
                self.identity.hex(),
                self.control.writer_identity.hex(),
                ns(self.control.until),
                operator::hex(&self.control.config.source_sha256()),
                operator::hex(&self.control.config.digest()),
                self.control.payload_length,
                r.sync_completed_before_spawn_at_boottime_ns,
                r.journal_device,
                r.journal_inode,
                r.journal_uid,
                r.journal_mode,
                w.sync_completed_before_spawn_at_boottime_ns,
                w.journal_device,
                w.journal_inode,
                w.journal_uid,
                w.journal_mode
            ));
        }
        if matches!(
            self.control.stage,
            WriterStage::WaitingJob | WriterStage::AwaitAck
        ) {
            let received = self.control.writer.receive_message(
                self.control.writer_identity,
                Deadline::at(self.control.until),
            )?;
            if let Some(bytes) = received {
                let ChildMessage::Synced {
                    operation,
                    writer,
                    receipt,
                } = ChildMessage::decode(&bytes)?
                else {
                    return Err(invalid());
                };
                if self.control.stage != WriterStage::AwaitAck
                    || self.control.operation.as_ref() != Some(&operation)
                    || writer != self.control.writer_identity
                    || receipt.uid != rustix::process::geteuid().as_raw()
                    || FileType::from_raw_mode(receipt.mode) != FileType::RegularFile
                    || receipt.mode & 0o7777 != 0o600
                    || receipt.inode == 0
                    || receipt.completed_at_ns < self.control.admitted_at_ns
                    || receipt.completed_at_ns >= operation.deadline_ns
                    || receipt.completed_at_ns > ns(now())
                {
                    return Err(invalid());
                }
                let current = now();
                if self.control.blocked
                    || current >= self.control.until
                    || ns(current) >= operation.deadline_ns
                    || !self.owner.admits(self.identity)
                    || self.state.phase != Phase::ObservedActive
                    || self.control.client_id != Some(operation.client_id)
                {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                event(format!(
                    "\"event\":\"intent-synced\",\"incarnation\":\"{}\",\"writer_incarnation\":\"{}\",\"intent\":\"{}\",\"deadline_ns\":{},\"payload_sha256\":\"{}\",\"payload_length\":{},\"sync_completed_at_ns\":{},\"device\":{},\"inode\":{},\"uid\":{},\"mode\":{}",
                    self.identity.hex(),
                    writer.hex(),
                    operator::hex(&operation.intent),
                    operation.deadline_ns,
                    operator::hex(&operation.payload_sha256),
                    operation.payload_length,
                    receipt.completed_at_ns,
                    receipt.device,
                    receipt.inode,
                    receipt.uid,
                    receipt.mode
                ));
                self.control.synced = Some(receipt);
                let finish = ParentMessage::Finish {
                    operation: operation.clone(),
                    writer,
                }
                .encode()?;
                // ACK is accepted while its origin is live; only this lifecycle
                // instruction lets it exit. Neither ACK nor exit alone dispatches.
                self.control.writer.send_message(
                    writer,
                    Deadline::at(Duration::from_nanos(operation.deadline_ns)),
                    &finish,
                )?;
                self.control.stage = WriterStage::AwaitReap;
            }
        }
        if self.owner.admits(self.identity)
            && let Some(bytes) = self
                .owner
                .receive_message(self.identity, Deadline::at(self.control.until))?
        {
            let ChildMessage::Request {
                operation,
                revision,
                outcome,
                response,
            } = ChildMessage::decode(&bytes)?
            else {
                return Err(invalid());
            };
            if !self.control.permit_attempted
                || self.control.blocked
                || self.control.operation.as_ref() != Some(&operation)
                || revision == 0
                || revision <= self.control.worker_revision
            {
                return Err(invalid());
            }
            if let Some(value) = &response {
                value.validate()?;
                if outcome != RequestOutcome::OutcomeUnknown
                    || value.received_at_ns >= operation.deadline_ns
                    || value.received_at_ns > ns(now())
                    || value.publication_revision > revision
                {
                    return Err(invalid());
                }
            }
            let history = self.state.request_history.as_mut().ok_or_else(invalid)?;
            if history.outcome == RequestOutcome::NotSent && outcome != RequestOutcome::NotSent
                || history.response.is_some() && history.response != response
                || outcome == RequestOutcome::NotSent && response.is_some()
            {
                return Err(invalid());
            }
            history.request_revision = history
                .request_revision
                .checked_add(1)
                .ok_or_else(invalid)?;
            history.outcome = outcome;
            history.response = response;
            self.control.worker_revision = revision;
        }
        Ok(())
    }
    fn maybe_dispatch(&mut self) -> io::Result<()> {
        self.signal_stop();
        if self.closing
            || self.cleanup_until.is_some()
            || self.control.blocked
            || self.control.permit_attempted
        {
            return Ok(());
        }
        let (Some(operation), Some(receipt), Some(waited)) = (
            self.control.operation.clone(),
            self.control.synced.clone(),
            self.control.writer_waited_at,
        ) else {
            return Ok(());
        };
        if self.control.stage != WriterStage::Reaped
            || !self.owner.admits(self.identity)
            || self.state.phase != Phase::ObservedActive
            || self.control.client_id != Some(operation.client_id)
        {
            return Err(invalid());
        }
        let admitted = now();
        if admitted >= self.control.until || ns(admitted) >= operation.deadline_ns {
            self.control.blocked = true;
            return Ok(());
        }
        let bytes = ParentMessage::Dispatch {
            operation: operation.clone(),
            config: self.control.config.clone(),
        }
        .encode()?;
        // A newly observed manager Stop fences before the irreversible send.
        // A signal arriving after this check cannot undo a send already begun.
        self.signal_stop();
        if self.closing {
            return Ok(());
        }
        self.control.permit_attempted = true;
        // A failed/late send can already have delivered. This flag never resets.
        self.owner.send_message(
            self.identity,
            Deadline::at(Duration::from_nanos(operation.deadline_ns)),
            &bytes,
        )?;
        let sent = now();
        event(format!(
            "\"event\":\"permit-released\",\"incarnation\":\"{}\",\"writer_incarnation\":\"{}\",\"intent\":\"{}\",\"deadline_ns\":{},\"session_until_ns\":{},\"writer_waited_at_ns\":{},\"sync_completed_at_ns\":{},\"admitted_at_ns\":{},\"sent_at_ns\":{}",
            self.identity.hex(),
            self.control.writer_identity.hex(),
            operator::hex(&operation.intent),
            operation.deadline_ns,
            ns(self.control.until),
            ns(waited),
            receipt.completed_at_ns,
            ns(admitted),
            ns(sent)
        ));
        Ok(())
    }
}

pub(super) fn run(config: Config) -> io::Result<i32> {
    // Main remains single-threaded. Keep the shared process-owned latch alive
    // through every setup disposal and both actual child waits.
    let _termination = pokelinux_kernel_identity::TerminationGuard::install()?;
    if existing_marker(&config.journal)? {
        refused();
        return Ok(78);
    }
    private_directory(&config.runtime)?;
    private_directory(config.journal.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    let canonical = operator::load_config()?;
    let payload_length = canonical
        .payload()?
        .len()
        .try_into()
        .map_err(|_| invalid())?;
    let mut intent = [0; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut intent)?;
    let mut owner = Supervisor::new(config.journal.clone());
    let mut writer = Supervisor::new(marker(&config.journal, ".writer"));
    let mut worker = WorkerCommand::new(&config.worker)?;
    let until_text = ns(config.until).to_string();
    let slot_text = config.slot.to_string();
    for value in [
        "--device",
        &config.device,
        "--service",
        &config.service,
        "--slot",
        &slot_text,
        "--until-ns",
        &until_text,
    ] {
        worker.arg(value);
    }
    if pokelinux_kernel_identity::termination_requested() {
        return Ok(0);
    }
    let identity = match owner.start_worker(&mut worker) {
        Ok(id) => id,
        Err(error) => {
            let uncertain = error == StartError::AdmissionUncertain;
            dispose_setup(&mut owner, &mut writer);
            if uncertain {
                refused();
                return Ok(78);
            }
            return Ok(70);
        }
    };
    let worker_pid = owner.child_pid().expect("retained worker");
    event(format!(
        "\"event\":\"worker-start\",\"main_pid\":{},\"worker_pid\":{},\"incarnation\":\"{}\",\"until_ns\":{},\"selected_slot\":{}",
        std::process::id(),
        worker_pid,
        identity.hex(),
        ns(config.until),
        config.slot
    ));
    if pokelinux_kernel_identity::termination_requested() {
        dispose_setup(&mut owner, &mut writer);
        return Ok(0);
    }
    let operation_path = marker(&config.journal, ".add-pdn");
    let setup = (|| -> io::Result<_> {
        let mut job = WorkerCommand::new(&config.worker)?;
        job.arg("--intent-writer")
            .arg("--journal")
            .arg(&operation_path)
            .arg("--until-ns")
            .arg(&until_text);
        let writer_identity = writer.start_worker(&mut job).map_err(|_| invalid())?;
        let writer_pid = writer.child_pid().ok_or_else(invalid)?;
        let radio_launch = owner.admission_receipt().ok_or_else(invalid)?;
        let writer_launch = writer.admission_receipt().ok_or_else(invalid)?;
        let listener = Listener::bind(&config.runtime.join("status.sock"))?;
        Ok((
            writer_identity,
            writer_pid,
            radio_launch,
            writer_launch,
            listener,
        ))
    })();
    let (writer_identity, writer_pid, radio_launch, writer_launch, listener) = match setup {
        Ok(value) => value,
        Err(_) => {
            dispose_setup(&mut owner, &mut writer);
            return Ok(70);
        }
    };
    event(format!(
        "\"event\":\"writer-start\",\"main_pid\":{},\"writer_pid\":{},\"writer_incarnation\":\"{}\",\"incarnation\":\"{}\",\"until_ns\":{}",
        std::process::id(),
        writer_pid,
        writer_identity.hex(),
        identity.hex(),
        ns(config.until)
    ));
    let mut service = Service {
        owner,
        identity,
        worker_pid,
        slot: config.slot,
        state: Status {
            phase: Phase::Initializing,
            identity: Some(identity),
            revision: 0,
            failure: None,
            last_observation: None,
            request_history: None,
        },
        input: Vec::new(),
        cleanup_until: None,
        closing: false,
        fault: false,
        raw_seen: false,
        control: Control {
            writer,
            writer_identity,
            writer_pid,
            radio_launch,
            writer_launch,
            config: canonical,
            payload_length,
            next_intent: intent,
            until: config.until,
            stage: WriterStage::Initializing,
            ready: false,
            client_id: None,
            operation: None,
            admitted_at_ns: 0,
            synced: None,
            writer_waited_at: None,
            permit_attempted: false,
            blocked: false,
            worker_revision: 0,
        },
    };
    let mut clients: Vec<Client> = Vec::new();
    loop {
        service.signal_stop();
        service.reap();
        if now() >= config.until && service.cleanup_until.is_none() {
            service.retire(ReceiverFailure::Expired, false);
        }
        if service.poll_output().is_err() || service.poll_control().is_err() {
            service.retire(ReceiverFailure::InternalFailure, false);
        }
        if !service.closing {
            for _ in 0..2 {
                match listener.accept() {
                    Ok(Some(connection)) if clients.len() < 8 => clients.push(Client {
                        connection,
                        until: now() + Duration::from_millis(500),
                        command: None,
                        admit_reply: None,
                    }),
                    Ok(Some(_)) => {}
                    Ok(None) => break,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => break,
                    Err(e) if e.kind() == io::ErrorKind::PermissionDenied => continue,
                    Err(_) => {
                        service.fault = true;
                        service.retire(ReceiverFailure::InternalFailure, true);
                        break;
                    }
                }
            }
        }
        clients.retain_mut(|client| {
            if now() >= client.until {
                return false;
            }
            if client.command.is_none() {
                match client.connection.receive() {
                    Ok(Some(command)) => {
                        match &command {
                            Command::Stop => service.retire(ReceiverFailure::Cancelled, true),
                            Command::Admit(body) => {
                                client.admit_reply = Some(service.admit(body, &client.connection))
                            }
                            Command::Status => {}
                        }
                        client.command = Some(command);
                    }
                    Ok(None) => return true,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => return true,
                    Err(_) => return false,
                }
            }
            // Status/Stop snapshots are recomputed after fencing and every
            // backpressure turn. Admit's one-use verdict is never re-executed.
            service.reap();
            if now() >= config.until && service.cleanup_until.is_none() {
                service.retire(ReceiverFailure::Expired, false);
            }
            let reply = match client.command.as_ref() {
                Some(Command::Status) => Reply::Status(service.state.clone()),
                Some(Command::Stop) => Reply::StopAck(service.state.clone()),
                Some(Command::Admit(_)) => {
                    Reply::Admit(client.admit_reply.clone().expect("one-use verdict"))
                }
                None => unreachable!(),
            };
            match client.connection.send(&reply) {
                Ok(()) => false,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    true
                }
                Err(_) => false,
            }
        });
        // Stop handling precedes the final irreversible permit gate this turn.
        service.reap();
        if service.maybe_dispatch().is_err() {
            service.retire(ReceiverFailure::InternalFailure, false);
        }
        if service.closing
            && !service.owner.occupied()
            && !service.control.occupied()
            && !clients
                .iter()
                .any(|client| matches!(client.command.as_ref(), Some(Command::Stop)))
        {
            return Ok(if service.fault { 70 } else { 0 });
        }
        std::thread::sleep(TURN);
    }
}
