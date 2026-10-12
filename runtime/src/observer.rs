//! Read-only observation ordering and presentation leases. No command authority.
//! The daemon must drive expiry independently of blocked network work and call
//! invalidate on shutdown and resume before accepting more data. Transport
//! errors go through failed so a superseded failure cannot clear newer state.
use crate::{
    Identity,
    watch::{Cursor, Snapshot, SourceState, WatchQuery},
};
use std::time::Duration;

pub struct Pending {
    query: WatchQuery,
    started: Duration,
    short: bool,
}
impl Pending {
    pub fn query(&self) -> &WatchQuery {
        &self.query
    }
}
#[derive(Clone)]
pub struct Lease {
    id: Identity,
    continuity: Identity,
    expires: Duration,
    snapshot: Snapshot,
}
impl Lease {
    pub fn id(&self) -> &Identity {
        &self.id
    }
    /// Uninterrupted confirmed ringing set. Unlike id(), this survives a
    /// renewal, but never a freshness gap, source loss or changed call episode.
    pub fn continuity(&self) -> &Identity {
        &self.continuity
    }
    pub fn expires(&self) -> Duration {
        self.expires
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}
#[derive(Default)]
pub struct Observer {
    active: Option<Identity>,
    cursor: Option<Cursor>,
    lease: Option<Lease>,
    confirm: bool,
}
fn identity() -> Identity {
    Identity(uuid::Uuid::new_v4().simple().to_string())
}
impl Observer {
    /// All times supplied to this state machine must use CLOCK_BOOTTIME.
    pub fn begin(&mut self, now: Duration) -> Pending {
        if self
            .lease
            .as_ref()
            .is_some_and(|lease| now >= lease.expires)
        {
            self.lease = None;
            self.confirm = true;
        }
        let wait = if self.confirm {
            0
        } else if self.lease.is_some() {
            1000
        } else {
            25000
        };
        let id = identity();
        self.active = Some(id.clone());
        Pending {
            query: WatchQuery::new(id, self.cursor.clone(), wait).expect("fixed watch wait"),
            started: now,
            short: wait <= 1000,
        }
    }
    /// True means this reply belonged to the active request. False ignores a
    /// superseded reply without changing newer state. Snapshot decoding must use
    /// pending.query().id(), as the typed TLS watch method does.
    pub fn complete(
        &mut self,
        pending: Pending,
        snapshot: Snapshot,
        now: Duration,
    ) -> Result<bool, &'static str> {
        if self.active.as_ref() != Some(pending.query.id()) {
            return Ok(false);
        }
        self.active = None;
        let budget = Duration::from_secs(if pending.short { 5 } else { 30 });
        let Some(deadline) = pending.started.checked_add(budget) else {
            self.invalidate();
            return Err("observation deadline overflow");
        };
        if now < pending.started || now >= deadline {
            self.invalidate();
            return Err("observation expired");
        }
        let next = snapshot.cursor();
        if self.cursor.as_ref().is_some_and(|old| {
            old.generation() == next.generation() && next.revision() < old.revision()
        }) {
            self.invalidate();
            return Err("observation revision regressed");
        }
        self.cursor = Some(next);
        if snapshot.source_state() != SourceState::Observed || snapshot.ringing().is_empty() {
            self.lease = None;
            self.confirm = false;
        } else if pending.short {
            let continuity = self
                .lease
                .as_ref()
                .filter(|old| {
                    now < old.expires
                        && old.snapshot.cursor().generation() == snapshot.cursor().generation()
                        && old.snapshot.ringing().len() == snapshot.ringing().len()
                        && old.snapshot.ringing().iter().all(|call| {
                            snapshot.ringing().iter().any(|next| {
                                call.id() == next.id() && call.episode() == next.episode()
                            })
                        })
                })
                .map(|old| old.continuity.clone())
                .unwrap_or_else(identity);
            self.lease = Some(Lease {
                id: identity(),
                continuity,
                expires: deadline,
                snapshot,
            });
            self.confirm = false;
        } else {
            self.lease = None;
            self.confirm = true;
        }
        Ok(true)
    }
    /// Expired data is never visible, even if its timer has not run yet.
    pub fn lease(&self, now: Duration) -> Option<&Lease> {
        self.lease.as_ref().filter(|lease| now < lease.expires)
    }
    /// A delayed timer for an old lease cannot withdraw a renewed lease.
    pub fn expire(&mut self, id: &Identity, now: Duration) -> bool {
        if self
            .lease
            .as_ref()
            .is_some_and(|lease| &lease.id == id && now >= lease.expires)
        {
            self.lease = None;
            self.confirm = true;
            true
        } else {
            false
        }
    }
    /// A transport error belongs to its request, just like a successful reply.
    pub fn failed(&mut self, pending: Pending) -> bool {
        if self.active.as_ref() != Some(pending.query.id()) {
            return false;
        }
        self.invalidate();
        true
    }
    /// Current timer target, including an overdue lease awaiting withdrawal.
    pub fn expiry(&self) -> Option<(Identity, Duration)> {
        self.lease
            .as_ref()
            .map(|lease| (lease.id.clone(), lease.expires))
    }
    pub fn invalidate(&mut self) {
        self.active = None;
        self.cursor = None;
        self.lease = None;
        self.confirm = false;
    }
}
