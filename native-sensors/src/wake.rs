//! Service-owned coordinator leases around transport reads and acknowledged disposition.
//! A read callback must perform only one bounded local queue operation/normalization.
use crate::{AckPlan, ClientToken, Error, Kind, Owner, Principal, Sample, Ticket};
use pokelinux_suspend::{Coordinator, Snapshot, WakeLock};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReadOutcome {
    Empty,
    Sample(Sample),
    Malformed,
    Uncertain,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AckOutcome {
    Full,
    WrittenAndSignalled,
    WrittenSignalFailed,
    Uncertain,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeError {
    Policy(Error),
    Acquire,
    Empty,
    Malformed,
    Uncertain,
    Full,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Acquiring,
    Reading,
    Received,
    Processed,
    AckUncertain,
    Quarantined,
}
struct Record {
    stage: Stage,
    lease: Option<WakeLock>,
}
struct State {
    policy: Owner,
    records: BTreeMap<Ticket, Record>,
}
struct Inner {
    state: Mutex<State>,
    coordinator: Coordinator,
}
impl Drop for Inner {
    fn drop(&mut self) {
        // Losing this service owner must fence the authority before automatic lease destruction.
        // This is local fail-closed coordination; a replacement process needs durable reconciliation.
        self.coordinator.stop();
    }
}
#[derive(Clone)]
pub struct WakeOwner(Arc<Inner>);
#[derive(Clone, Copy, Debug)]
pub struct WakeSnapshot {
    pub pending: usize,
    pub quarantined: usize,
    pub acquiring: usize,
    pub native: Snapshot,
}
impl From<Error> for WakeError {
    fn from(value: Error) -> Self {
        Self::Policy(value)
    }
}
impl WakeOwner {
    pub fn new(epoch: u64, max_pending: usize, coordinator: Coordinator) -> Self {
        Self(Arc::new(Inner {
            state: Mutex::new(State {
                policy: Owner::new(epoch, max_pending),
                records: BTreeMap::new(),
            }),
            coordinator,
        }))
    }
    pub fn grant(&self, principal: Principal) {
        self.0.state.lock().unwrap().policy.grant(principal);
    }
    pub fn subscribe(&self, principal: Principal, kind: Kind) -> Result<ClientToken, WakeError> {
        Ok(self
            .0
            .state
            .lock()
            .unwrap()
            .policy
            .subscribe(principal, kind)?)
    }
    pub fn revoke(&self, principal: Principal) {
        self.0.state.lock().unwrap().policy.revoke(principal);
    }
    pub fn snapshot(&self) -> WakeSnapshot {
        let state = self.0.state.lock().unwrap();
        WakeSnapshot {
            pending: state.policy.pending(),
            quarantined: state
                .records
                .values()
                .filter(|r| matches!(r.stage, Stage::Quarantined | Stage::AckUncertain))
                .count(),
            acquiring: state
                .records
                .values()
                .filter(|r| r.stage == Stage::Acquiring)
                .count(),
            native: self.0.coordinator.snapshot(),
        }
    }
    fn current(state: &State, ticket: Ticket) -> bool {
        state.policy.work.get(&ticket).is_some_and(|work| {
            state.policy.alive
                && work.epoch == state.policy.epoch
                && !work.quarantined
                && state
                    .policy
                    .clients
                    .get(&work.client)
                    .is_some_and(|client| {
                        client.active && state.policy.grants.contains(&client.principal)
                    })
        })
    }
    fn unread(&self, ticket: Ticket) {
        let record = {
            let mut state = self.0.state.lock().unwrap();
            state.policy.work.remove(&ticket);
            state.records.remove(&ticket)
        };
        // Lease destruction can wait for the coordinator gate; never hold the sensor mutex.
        drop(record);
    }
    pub fn read_wake(
        &self,
        token: ClientToken,
        epoch: u64,
        read: impl FnOnce() -> ReadOutcome,
    ) -> Result<Ticket, WakeError> {
        let ticket = {
            let mut state = self.0.state.lock().unwrap();
            let ticket = state.policy.reserve(token, epoch, true)?;
            state.records.insert(
                ticket,
                Record {
                    stage: Stage::Acquiring,
                    lease: None,
                },
            );
            ticket
        };
        let lease = match self.0.coordinator.acquire() {
            Ok(lease) => lease,
            Err(_) => {
                self.unread(ticket);
                return Err(WakeError::Acquire);
            }
        };
        {
            let mut state = self.0.state.lock().unwrap();
            if !Self::current(&state, ticket) {
                drop(state);
                self.unread(ticket);
                drop(lease);
                return Err(WakeError::Policy(Error::Stale));
            }
            let record = state.records.get_mut(&ticket).unwrap();
            record.lease = Some(lease);
            record.stage = Stage::Reading;
        }
        // No transport read occurs until the real native lease is held and admission rechecked.
        let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(read)) {
            Ok(outcome) => outcome,
            Err(_) => {
                let mut state = self.0.state.lock().unwrap();
                let _ = state.policy.reject_decoding(ticket);
                state.records.get_mut(&ticket).unwrap().stage = Stage::Quarantined;
                drop(state);
                self.0.coordinator.stop();
                return Err(WakeError::Uncertain);
            }
        };
        let mut state = self.0.state.lock().unwrap();
        match outcome {
            ReadOutcome::Empty => {
                let record = state.records.remove(&ticket);
                state.policy.work.remove(&ticket);
                drop(state);
                drop(record);
                Err(WakeError::Empty)
            }
            ReadOutcome::Sample(sample) => {
                if state.policy.decoded(ticket, sample).is_err() {
                    state.records.get_mut(&ticket).unwrap().stage = Stage::Quarantined;
                    return Err(WakeError::Uncertain);
                }
                state.records.get_mut(&ticket).unwrap().stage = Stage::Received;
                Ok(ticket)
            }
            ReadOutcome::Malformed | ReadOutcome::Uncertain => {
                state.policy.reject_decoding(ticket)?;
                state.records.get_mut(&ticket).unwrap().stage = Stage::Quarantined;
                Err(if outcome == ReadOutcome::Malformed {
                    WakeError::Malformed
                } else {
                    WakeError::Uncertain
                })
            }
        }
    }
    pub fn processed(&self, ticket: Ticket) -> Result<bool, WakeError> {
        let mut state = self.0.state.lock().unwrap();
        let eligible = state.policy.processed(ticket)?;
        let record = state.records.get_mut(&ticket).ok_or(Error::Unknown)?;
        if !matches!(record.stage, Stage::Quarantined | Stage::AckUncertain) {
            record.stage = Stage::Processed;
        }
        // Processing alone does not transfer the service's actual native wake responsibility.
        Ok(eligible)
    }
    pub fn acknowledge(
        &self,
        tickets: &[Ticket],
        send: impl FnOnce(AckPlan) -> AckOutcome,
    ) -> Result<(), WakeError> {
        let plan = {
            let mut state = self.0.state.lock().unwrap();
            state.policy.prepare_ack(tickets)?
        };
        let outcome = send(plan);
        let mut state = self.0.state.lock().unwrap();
        match outcome {
            AckOutcome::Full => {
                state.policy.abort_ack(plan)?;
                Err(WakeError::Full)
            }
            AckOutcome::WrittenAndSignalled => {
                if state.policy.commit_ack(plan).is_err() {
                    for ticket in tickets {
                        if let Some(record) = state.records.get_mut(ticket) {
                            record.stage = Stage::AckUncertain;
                        }
                    }
                    return Err(WakeError::Uncertain);
                }
                let retired = tickets
                    .iter()
                    .filter_map(|ticket| state.records.remove(ticket))
                    .collect::<Vec<_>>();
                drop(state);
                drop(retired);
                Ok(())
            }
            AckOutcome::WrittenSignalFailed | AckOutcome::Uncertain => {
                for ticket in tickets {
                    if let Some(record) = state.records.get_mut(ticket) {
                        record.stage = Stage::AckUncertain;
                    }
                }
                Err(WakeError::Uncertain)
            }
        }
    }
    pub fn reinitialize(&self, epoch: u64) {
        let mut state = self.0.state.lock().unwrap();
        state.policy.reinitialize(epoch);
        for record in state.records.values_mut() {
            if record.stage != Stage::Acquiring {
                record.stage = Stage::Quarantined;
            }
        }
    }
    pub fn backend_died(&self) {
        {
            let mut state = self.0.state.lock().unwrap();
            state.policy.backend_died();
            for record in state.records.values_mut() {
                if record.stage != Stage::Acquiring {
                    record.stage = Stage::Quarantined;
                }
            }
        }
        // Source loss is unresolved even if no current ticket can identify queued work.
        self.0.coordinator.stop();
    }
}
