//! Portable request arbitration. This crate never controls a kernel or device.
//! The service supplies authenticated owner identities, a boot-time clock and
//! one serialized backend. Logical profile values contain no vendor paths.
pub mod clock;
use clock::BootTime;
use std::{collections::BTreeMap, sync::Arc, time::Duration};

pub const MAX_RESOURCES: usize = 32;
pub const MAX_VALUES: usize = 32;
pub const MAX_LEASES: usize = 1024;
pub const MAX_OWNER_LEASES: usize = 64;

/// A fresh service-assigned identity, not an unqualified OS PID or app input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwnerId(pub u64);

/// Earlier values win. Default is selected only when no lease remains.
#[derive(Clone, Debug)]
pub struct ResourcePolicy {
    pub key: String,
    pub values: Vec<String>,
    pub default_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expiry {
    At(BootTime),
    UntilCancelled,
}
impl Expiry {
    pub fn after(now: BootTime, duration: Duration) -> Result<Self, Error> {
        now.checked_after(duration)
            .map(Self::At)
            .ok_or(Error::DeadlineOverflow)
    }
    fn active(self, now: BootTime) -> bool {
        match self {
            Self::At(end) => now < end,
            Self::UntilCancelled => true,
        }
    }
    fn extends(self, old: Self) -> bool {
        match (self, old) {
            (Self::UntilCancelled, Self::At(_)) => true,
            (Self::At(new), Self::At(old)) => new > old,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LeaseId {
    number: u64,
    scope: Arc<()>,
}
#[derive(Clone, Debug)]
pub struct Application {
    resource: String,
    value: String,
    number: u64,
    epoch: u64,
    scope: Arc<()>,
}
impl Application {
    pub fn resource(&self) -> &str {
        &self.resource
    }
    pub fn value(&self) -> &str {
        &self.value
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppliedState {
    Unknown,
    Confirmed(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub desired: String,
    pub applied: AppliedState,
    /// Historical successful application in this backend generation.
    pub last_confirmed: Option<String>,
    pub pending: bool,
    pub failed: bool,
    pub retry: RetryState,
    pub backend_epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyResult {
    Applied,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryState {
    Ready,
    After(BootTime),
    Exhausted,
}
impl RetryState {
    fn blocked(self, now: BootTime) -> bool {
        match self {
            Self::Ready => false,
            Self::After(at) => now < at,
            Self::Exhausted => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidPolicy,
    UnknownResource,
    UnknownValue,
    UnknownLease,
    WrongOwner,
    StaleToken,
    LeaseCapacity,
    OwnerCapacity,
    ExpiryElapsed,
    DeadlineOverflow,
    ClockRegressed,
    IdentityExhausted,
}

struct Lease {
    owner: OwnerId,
    resource: String,
    value: usize,
    expiry: Expiry,
}
struct Pending {
    number: u64,
    value: usize,
    epoch: u64,
}
struct Resource {
    policy: ResourcePolicy,
    desired: usize,
    applied: Option<usize>,
    last_confirmed: Option<usize>,
    pending: Option<Pending>,
    retry: RetryState,
    failed: bool,
    epoch: u64,
}

/// Single-owner component. Plans are serialized per logical resource; a
/// deployment must additionally serialize the actual backend IO and reclaim it.
pub struct Arbiter {
    resources: BTreeMap<String, Resource>,
    leases: BTreeMap<u64, Lease>,
    scope: Arc<()>,
    next_identity: u64,
    last_time: BootTime,
    retry_delay: Duration,
}
impl Arbiter {
    pub fn new(policies: Vec<ResourcePolicy>, retry_delay: Duration) -> Result<Self, Error> {
        if policies.is_empty() || policies.len() > MAX_RESOURCES || retry_delay.is_zero() {
            return Err(Error::InvalidPolicy);
        }
        let mut resources = BTreeMap::new();
        for policy in policies {
            if !logical_key(&policy.key)
                || policy.values.is_empty()
                || policy.values.len() > MAX_VALUES
                || policy.default_index >= policy.values.len()
                || policy.values.iter().any(|v| !logical_key(v))
                || policy
                    .values
                    .iter()
                    .enumerate()
                    .any(|(i, v)| policy.values[..i].contains(v))
                || resources.contains_key(&policy.key)
            {
                return Err(Error::InvalidPolicy);
            }
            resources.insert(
                policy.key.clone(),
                Resource {
                    desired: policy.default_index,
                    policy,
                    applied: None,
                    last_confirmed: None,
                    pending: None,
                    retry: RetryState::Ready,
                    failed: false,
                    epoch: 0,
                },
            );
        }
        Ok(Self {
            resources,
            leases: BTreeMap::new(),
            scope: Arc::new(()),
            next_identity: 0,
            last_time: BootTime::default(),
            retry_delay,
        })
    }
    pub fn advance(&mut self, now: BootTime) -> Result<(), Error> {
        if now < self.last_time {
            return Err(Error::ClockRegressed);
        }
        self.last_time = now;
        self.leases.retain(|_, lease| lease.expiry.active(now));
        self.refresh();
        Ok(())
    }
    pub fn acquire(
        &mut self,
        owner: OwnerId,
        resource: &str,
        value: &str,
        expiry: Expiry,
        now: BootTime,
    ) -> Result<LeaseId, Error> {
        self.advance(now)?;
        if !expiry.active(now) {
            return Err(Error::ExpiryElapsed);
        }
        let node = self.resources.get(resource).ok_or(Error::UnknownResource)?;
        let value = node
            .policy
            .values
            .iter()
            .position(|v| v == value)
            .ok_or(Error::UnknownValue)?;
        if self.leases.len() >= MAX_LEASES {
            return Err(Error::LeaseCapacity);
        }
        if self.leases.values().filter(|l| l.owner == owner).count() >= MAX_OWNER_LEASES {
            return Err(Error::OwnerCapacity);
        }
        let number = self.identity()?;
        self.leases.insert(
            number,
            Lease {
                owner,
                resource: resource.to_owned(),
                value,
                expiry,
            },
        );
        self.refresh();
        Ok(LeaseId {
            number,
            scope: Arc::clone(&self.scope),
        })
    }
    pub fn renew(
        &mut self,
        owner: OwnerId,
        id: &LeaseId,
        expiry: Expiry,
        now: BootTime,
    ) -> Result<bool, Error> {
        self.advance(now)?;
        self.check_scope(&id.scope)?;
        if !expiry.active(now) {
            return Err(Error::ExpiryElapsed);
        }
        let lease = self.leases.get_mut(&id.number).ok_or(Error::UnknownLease)?;
        if lease.owner != owner {
            return Err(Error::WrongOwner);
        }
        let extended = expiry.extends(lease.expiry);
        if extended {
            lease.expiry = expiry;
        }
        Ok(extended)
    }
    pub fn cancel(&mut self, owner: OwnerId, id: &LeaseId, now: BootTime) -> Result<(), Error> {
        self.advance(now)?;
        self.check_scope(&id.scope)?;
        let lease = self.leases.get(&id.number).ok_or(Error::UnknownLease)?;
        if lease.owner != owner {
            return Err(Error::WrongOwner);
        }
        self.leases.remove(&id.number);
        self.refresh();
        Ok(())
    }
    /// Called by the service after authenticated disconnect/revocation.
    pub fn cancel_owner(&mut self, owner: OwnerId, now: BootTime) -> Result<usize, Error> {
        self.advance(now)?;
        let before = self.leases.len();
        self.leases.retain(|_, lease| lease.owner != owner);
        self.refresh();
        Ok(before - self.leases.len())
    }
    pub fn snapshot(&mut self, resource: &str, now: BootTime) -> Result<Snapshot, Error> {
        self.advance(now)?;
        let r = self.resources.get(resource).ok_or(Error::UnknownResource)?;
        Ok(Snapshot {
            desired: r.policy.values[r.desired].clone(),
            applied: r.applied.map_or(AppliedState::Unknown, |i| {
                AppliedState::Confirmed(r.policy.values[i].clone())
            }),
            last_confirmed: r.last_confirmed.map(|i| r.policy.values[i].clone()),
            pending: r.pending.is_some(),
            failed: r.failed,
            retry: r.retry,
            backend_epoch: r.epoch,
        })
    }
    pub fn begin_apply(
        &mut self,
        resource: &str,
        now: BootTime,
    ) -> Result<Option<Application>, Error> {
        self.advance(now)?;
        let r = self.resources.get(resource).ok_or(Error::UnknownResource)?;
        if r.pending.is_some() || r.applied == Some(r.desired) || r.retry.blocked(now) {
            return Ok(None);
        }
        let number = self.identity()?;
        let r = self
            .resources
            .get_mut(resource)
            .expect("validated resource");
        r.pending = Some(Pending {
            number,
            value: r.desired,
            epoch: r.epoch,
        });
        r.applied = None;
        Ok(Some(Application {
            resource: resource.to_owned(),
            value: r.policy.values[r.desired].clone(),
            number,
            epoch: r.epoch,
            scope: Arc::clone(&self.scope),
        }))
    }
    pub fn finish_apply(
        &mut self,
        application: &Application,
        result: ApplyResult,
        now: BootTime,
    ) -> Result<(), Error> {
        self.advance(now)?;
        self.check_scope(&application.scope)?;
        let r = self
            .resources
            .get_mut(&application.resource)
            .ok_or(Error::StaleToken)?;
        let pending = r.pending.as_ref().ok_or(Error::StaleToken)?;
        if pending.number != application.number || pending.epoch != application.epoch {
            return Err(Error::StaleToken);
        }
        let pending = r.pending.take().expect("checked pending application");
        match result {
            ApplyResult::Applied => {
                r.applied = Some(pending.value);
                r.last_confirmed = r.applied;
                r.failed = false;
                r.retry = RetryState::Ready;
            }
            ApplyResult::Failed => {
                r.applied = None;
                r.failed = true;
                r.retry = now
                    .checked_after(self.retry_delay)
                    .map_or(RetryState::Exhausted, RetryState::After);
            }
        }
        Ok(())
    }
    /// Fences callbacks, not physical writes. The caller must stop old backend IO.
    pub fn backend_lost(&mut self, resource: &str, now: BootTime) -> Result<(), Error> {
        self.advance(now)?;
        let r = self
            .resources
            .get_mut(resource)
            .ok_or(Error::UnknownResource)?;
        r.epoch = r.epoch.checked_add(1).ok_or(Error::IdentityExhausted)?;
        r.pending = None;
        r.applied = None;
        r.last_confirmed = None;
        r.failed = false;
        r.retry = RetryState::Ready;
        Ok(())
    }
    /// Event-loop deadline only. It neither creates a timer nor wakes hardware.
    pub fn next_deadline(&self) -> Option<BootTime> {
        self.leases
            .values()
            .filter_map(|lease| match lease.expiry {
                Expiry::At(at) => Some(at),
                Expiry::UntilCancelled => None,
            })
            .chain(self.resources.values().filter_map(|r| {
                if r.pending.is_none()
                    && let RetryState::After(at) = r.retry
                {
                    Some(at)
                } else {
                    None
                }
            }))
            .min()
    }
    pub fn lease_count(&self) -> usize {
        self.leases.len()
    }
    fn identity(&mut self) -> Result<u64, Error> {
        self.next_identity = self
            .next_identity
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?;
        Ok(self.next_identity)
    }
    fn check_scope(&self, scope: &Arc<()>) -> Result<(), Error> {
        if Arc::ptr_eq(&self.scope, scope) {
            Ok(())
        } else {
            Err(Error::StaleToken)
        }
    }
    fn refresh(&mut self) {
        for resource in self.resources.values_mut() {
            resource.desired = desired_index(&resource.policy, self.leases.values());
        }
    }
}
fn logical_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn desired_index<'a>(policy: &ResourcePolicy, leases: impl Iterator<Item = &'a Lease>) -> usize {
    leases
        .filter(|lease| lease.resource == policy.key)
        .map(|lease| lease.value)
        .min()
        .unwrap_or(policy.default_index)
}
