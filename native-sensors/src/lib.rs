//! Device-neutral sensor admission, authorization and wake-credit ownership.
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Principal(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientToken(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ticket(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Acceleration,
    Location,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Permission,
    Stale,
    Capacity,
    Unknown,
    NotProcessed,
    Duplicate,
    Quarantined,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub timestamp_ns: i64,
    pub acceleration: [f32; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AckPlan {
    pub nonce: u64,
    pub epoch: u64,
    pub count: i32,
}
struct Client {
    principal: Principal,
    epoch: u64,
    active: bool,
}
struct Work {
    client: ClientToken,
    epoch: u64,
    wake: bool,
    sample: Option<Sample>,
    processed: bool,
    ack: Option<u64>,
    quarantined: bool,
}
/// Transport supplies authenticated principals; no raw sensor handle or vendor layout enters here.
pub struct Owner {
    epoch: u64,
    alive: bool,
    max_pending: usize,
    next_client: u64,
    next_ticket: u64,
    next_ack: u64,
    grants: BTreeSet<Principal>,
    clients: BTreeMap<ClientToken, Client>,
    work: BTreeMap<Ticket, Work>,
    plans: BTreeMap<u64, Vec<Ticket>>,
}
impl Owner {
    pub fn new(epoch: u64, max_pending: usize) -> Self {
        Self {
            epoch,
            alive: true,
            max_pending,
            next_client: 1,
            next_ticket: 1,
            next_ack: 1,
            grants: BTreeSet::new(),
            clients: BTreeMap::new(),
            work: BTreeMap::new(),
            plans: BTreeMap::new(),
        }
    }
    pub fn grant(&mut self, principal: Principal) {
        self.grants.insert(principal);
    }
    pub fn subscribe(&mut self, principal: Principal, kind: Kind) -> Result<ClientToken, Error> {
        if !self.alive || kind != Kind::Acceleration || !self.grants.contains(&principal) {
            return Err(Error::Permission);
        }
        if self.clients.len() >= 8 {
            return Err(Error::Capacity);
        }
        let token = ClientToken(self.next_client);
        self.next_client += 1;
        self.clients.insert(
            token,
            Client {
                principal,
                epoch: self.epoch,
                active: true,
            },
        );
        Ok(token)
    }
    pub fn can_admit(&self, token: ClientToken, epoch: u64) -> Result<(), Error> {
        if !self.alive || epoch != self.epoch {
            return Err(Error::Stale);
        }
        let client = self.clients.get(&token).ok_or(Error::Permission)?;
        if !client.active || client.epoch != epoch || !self.grants.contains(&client.principal) {
            return Err(Error::Permission);
        }
        if self.work.len() >= self.max_pending {
            return Err(Error::Capacity);
        }
        Ok(())
    }
    /// Reserve bounded service ownership before a transport read advances its counter.
    /// The reader holds the service mutex across reserve/read/decoded or reject_decoding.
    pub fn reserve(&mut self, token: ClientToken, epoch: u64, wake: bool) -> Result<Ticket, Error> {
        self.can_admit(token, epoch)?;
        let ticket = Ticket(self.next_ticket);
        self.next_ticket += 1;
        self.work.insert(
            ticket,
            Work {
                client: token,
                epoch,
                wake,
                sample: None,
                processed: false,
                ack: None,
                quarantined: false,
            },
        );
        Ok(ticket)
    }
    /// Used only when the synchronized queue read returned false without consuming data.
    pub fn cancel_unread(&mut self, ticket: Ticket) -> Result<(), Error> {
        let work = self.work.get(&ticket).ok_or(Error::Unknown)?;
        if work.sample.is_some() || work.processed || work.quarantined || work.ack.is_some() {
            return Err(Error::Duplicate);
        }
        self.work.remove(&ticket);
        Ok(())
    }
    pub fn decoded(&mut self, ticket: Ticket, sample: Sample) -> Result<(), Error> {
        let work = self.work.get_mut(&ticket).ok_or(Error::Unknown)?;
        if !self.alive || work.epoch != self.epoch || work.quarantined {
            return Err(Error::Stale);
        }
        if work.sample.is_some() {
            return Err(Error::Duplicate);
        }
        if sample.timestamp_ns <= 0 || sample.acceleration.iter().any(|n| !n.is_finite()) {
            work.quarantined = true;
            return Err(Error::Unknown);
        }
        work.sample = Some(sample);
        Ok(())
    }
    /// Raw receipt was owned but could not be normalized: keep its wake credit quarantined.
    pub fn reject_decoding(&mut self, ticket: Ticket) -> Result<(), Error> {
        let work = self.work.get_mut(&ticket).ok_or(Error::Unknown)?;
        if work.sample.is_some() || work.processed || work.ack.is_some() {
            return Err(Error::Duplicate);
        }
        work.quarantined = true;
        Ok(())
    }
    pub fn processed(&mut self, ticket: Ticket) -> Result<bool, Error> {
        let work = self.work.get_mut(&ticket).ok_or(Error::Unknown)?;
        if work.processed {
            return Err(Error::Duplicate);
        }
        if work.sample.is_none() {
            return Err(Error::Unknown);
        }
        work.processed = true;
        let client = self.clients.get(&work.client).ok_or(Error::Permission)?;
        Ok(self.alive
            && !work.quarantined
            && work.epoch == self.epoch
            && client.active
            && self.grants.contains(&client.principal))
    }
    pub fn prepare_ack(&mut self, tickets: &[Ticket]) -> Result<AckPlan, Error> {
        if !self.alive {
            return Err(Error::Quarantined);
        }
        if tickets.is_empty() {
            return Err(Error::Unknown);
        }
        let mut seen = BTreeSet::new();
        let mut count = 0;
        for ticket in tickets {
            if !seen.insert(*ticket) {
                return Err(Error::Duplicate);
            }
            let work = self.work.get(ticket).ok_or(Error::Unknown)?;
            if work.epoch != self.epoch || work.quarantined {
                return Err(Error::Stale);
            }
            if work.ack.is_some() {
                return Err(Error::Duplicate);
            }
            if !work.processed {
                return Err(Error::NotProcessed);
            }
            if work.wake {
                count += 1;
            }
        }
        if count == 0 {
            return Err(Error::Unknown);
        }
        let nonce = self.next_ack;
        self.next_ack += 1;
        for ticket in tickets {
            self.work.get_mut(ticket).unwrap().ack = Some(nonce);
        }
        self.plans.insert(nonce, tickets.to_vec());
        Ok(AckPlan {
            nonce,
            epoch: self.epoch,
            count,
        })
    }
    pub fn commit_ack(&mut self, plan: AckPlan) -> Result<(), Error> {
        if !self.alive || plan.epoch != self.epoch {
            return Err(Error::Stale);
        }
        let tickets = self.plans.get(&plan.nonce).ok_or(Error::Unknown)?;
        if plan.count
            != tickets
                .iter()
                .filter(|id| self.work.get(id).is_some_and(|w| w.wake))
                .count() as i32
        {
            return Err(Error::Unknown);
        }
        let tickets = self.plans.remove(&plan.nonce).unwrap();
        for ticket in tickets {
            self.work.remove(&ticket);
        }
        Ok(())
    }
    pub fn abort_ack(&mut self, plan: AckPlan) -> Result<(), Error> {
        let tickets = self.plans.remove(&plan.nonce).ok_or(Error::Unknown)?;
        for ticket in tickets {
            self.work.get_mut(&ticket).ok_or(Error::Unknown)?.ack = None;
        }
        Ok(())
    }
    pub fn revoke(&mut self, principal: Principal) {
        self.grants.remove(&principal);
        for client in self.clients.values_mut() {
            if client.principal == principal {
                client.active = false;
            }
        }
    }
    pub fn reinitialize(&mut self, epoch: u64) {
        self.epoch = epoch;
        for work in self.work.values_mut() {
            work.quarantined = true;
        }
        for client in self.clients.values_mut() {
            client.active = false;
        }
        self.plans.clear();
    }
    pub fn backend_died(&mut self) {
        self.alive = false;
        for work in self.work.values_mut() {
            work.quarantined = true;
        }
        for client in self.clients.values_mut() {
            client.active = false;
        }
    }
    pub fn wake_responsibilities(&self) -> usize {
        self.work.values().filter(|work| work.wake).count()
    }
    pub fn pending(&self) -> usize {
        self.work.len()
    }
}

pub mod wake;
