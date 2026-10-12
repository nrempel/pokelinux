//! One bounded owner. Vendor Binder/frame/stream identifiers belong in adapters.
use std::{
    collections::{BTreeMap, BTreeSet},
    os::fd::OwnedFd,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Generation {
    pub session: u64,
    pub configuration: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaptureId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LeaseId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Receipt {
    Pending,
    Accepted,
    Rejected,
    Uncertain,
    InvalidCount,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerError {
    WrongGeneration,
    Capacity,
    Duplicate,
    Unknown,
    NotReusable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub receipt: Receipt,
    pub returned: bool,
    pub capture_complete: bool,
    pub fence_ready: bool,
    pub publication_eligible: bool,
    pub reusable: bool,
    pub quarantined: bool,
    pub returns: u32,
}
struct Capture {
    generation: Generation,
    leases: Vec<LeaseId>,
    returned: BTreeSet<LeaseId>,
    fences_ready: BTreeSet<LeaseId>,
    receipt: Receipt,
    shutter: bool,
    final_metadata: bool,
    failed: bool,
}
struct Lease {
    generation: Generation,
    capture: CaptureId,
    _buffer: OwnedFd,
    returned: bool,
    fence_ready: bool,
    fence: Option<OwnedFd>,
    failed: bool,
    quarantined: bool,
    returns: u32,
}
/// Mutex users must never execute Binder or wait for a fence while holding it.
/// Publication eligibility is a policy result; adapters still require hardware qualification.
pub struct Owner {
    active: Generation,
    publication: bool,
    max_leases: usize,
    max_captures: usize,
    leases: BTreeMap<LeaseId, Lease>,
    captures: BTreeMap<CaptureId, Capture>,
}
impl Owner {
    pub fn new(active: Generation, max_leases: usize, max_captures: usize) -> Self {
        Self {
            active,
            publication: true,
            max_leases,
            max_captures,
            leases: BTreeMap::new(),
            captures: BTreeMap::new(),
        }
    }
    pub fn admit(
        &mut self,
        generation: Generation,
        id: CaptureId,
        lease: LeaseId,
        buffer: OwnedFd,
    ) -> Result<(), OwnerError> {
        if generation != self.active || !self.publication {
            return Err(OwnerError::WrongGeneration);
        }
        if self.captures.contains_key(&id) {
            return Err(OwnerError::Duplicate);
        }
        if self.captures.len() >= self.max_captures || self.leases.len() >= self.max_leases {
            return Err(OwnerError::Capacity);
        }
        if self.leases.contains_key(&lease) {
            return Err(OwnerError::Duplicate);
        }
        self.leases.insert(
            lease,
            Lease {
                generation,
                capture: id,
                _buffer: buffer,
                returned: false,
                fence_ready: false,
                fence: None,
                failed: false,
                quarantined: false,
                returns: 0,
            },
        );
        self.captures.insert(
            id,
            Capture {
                generation,
                leases: vec![lease],
                returned: BTreeSet::new(),
                fences_ready: BTreeSet::new(),
                receipt: Receipt::Pending,
                shutter: false,
                final_metadata: false,
                failed: false,
            },
        );
        Ok(())
    }
    pub fn reuse(
        &mut self,
        generation: Generation,
        id: CaptureId,
        lease: LeaseId,
    ) -> Result<(), OwnerError> {
        if generation != self.active || !self.publication {
            return Err(OwnerError::WrongGeneration);
        }
        let previous = self.leases.get(&lease).ok_or(OwnerError::Unknown)?.capture;
        if !self.snapshot(previous, lease)?.reusable {
            return Err(OwnerError::NotReusable);
        }
        if self.captures.len() >= self.max_captures {
            return Err(OwnerError::Capacity);
        }
        if self.captures.contains_key(&id) {
            return Err(OwnerError::Duplicate);
        }
        let state = self.leases.get_mut(&lease).unwrap();
        state.capture = id;
        state.returned = false;
        state.fence_ready = false;
        state.fence = None;
        state.failed = false;
        self.captures.insert(
            id,
            Capture {
                generation,
                leases: vec![lease],
                returned: BTreeSet::new(),
                fences_ready: BTreeSet::new(),
                receipt: Receipt::Pending,
                shutter: false,
                final_metadata: false,
                failed: false,
            },
        );
        Ok(())
    }
    pub fn receipt(
        &mut self,
        generation: Generation,
        captures: &[CaptureId],
        count: Option<i32>,
    ) -> Result<(), OwnerError> {
        let mut seen = BTreeSet::new();
        for id in captures {
            if !seen.insert(*id) {
                return Err(OwnerError::Duplicate);
            }
            let capture = self.captures.get(id).ok_or(OwnerError::Unknown)?;
            if capture.generation != generation {
                return Err(OwnerError::WrongGeneration);
            }
            if capture.receipt != Receipt::Pending {
                return Err(OwnerError::Duplicate);
            }
        }
        let valid = count.is_some_and(|n| n >= 0 && n as usize <= captures.len());
        for (index, id) in captures.iter().enumerate() {
            let capture = self.captures.get_mut(id).unwrap();
            capture.receipt = match count {
                None => Receipt::Uncertain,
                Some(_) if !valid => Receipt::InvalidCount,
                Some(n) if index < n as usize => Receipt::Accepted,
                Some(_) => Receipt::Rejected,
            };
            if capture.receipt == Receipt::InvalidCount {
                for lease in &capture.leases {
                    self.leases.get_mut(lease).unwrap().quarantined = true;
                }
            }
        }
        Ok(())
    }
    pub fn returned(
        &mut self,
        generation: Generation,
        id: CaptureId,
        lease: LeaseId,
        failed: bool,
        fence: Option<OwnedFd>,
    ) -> Result<bool, OwnerError> {
        let state = self.leases.get_mut(&lease).ok_or(OwnerError::Unknown)?;
        if state.generation != generation || state.capture != id {
            return Err(OwnerError::WrongGeneration);
        }
        if state.returned {
            return Ok(false);
        }
        state.returned = true;
        state.returns += 1;
        state.failed = failed;
        // A returned buffer remains inaccessible until its owned release fence is ready.
        state.fence_ready = fence.is_none();
        state.fence = fence;
        let capture = self.captures.get_mut(&id).ok_or(OwnerError::Unknown)?;
        capture.returned.insert(lease);
        if state.fence_ready {
            capture.fences_ready.insert(lease);
        }
        Ok(true)
    }
    pub fn final_metadata(
        &mut self,
        generation: Generation,
        id: CaptureId,
    ) -> Result<(), OwnerError> {
        let capture = self.capture_mut(generation, id)?;
        capture.final_metadata = true;
        Ok(())
    }
    pub fn shutter(&mut self, generation: Generation, id: CaptureId) -> Result<(), OwnerError> {
        let capture = self.capture_mut(generation, id)?;
        capture.shutter = true;
        Ok(())
    }
    pub fn failed(&mut self, generation: Generation, id: CaptureId) -> Result<(), OwnerError> {
        let capture = self.capture_mut(generation, id)?;
        capture.failed = true;
        Ok(())
    }
    fn capture_mut(
        &mut self,
        generation: Generation,
        id: CaptureId,
    ) -> Result<&mut Capture, OwnerError> {
        let capture = self.captures.get_mut(&id).ok_or(OwnerError::Unknown)?;
        if capture.generation != generation {
            return Err(OwnerError::WrongGeneration);
        }
        Ok(capture)
    }
    /// Duplicate only for a bounded probe outside the owner's critical section.
    pub fn fence_probe(
        &self,
        id: CaptureId,
        lease: LeaseId,
    ) -> Result<Option<OwnedFd>, OwnerError> {
        let state = self.leases.get(&lease).ok_or(OwnerError::Unknown)?;
        if state.capture != id {
            return Err(OwnerError::WrongGeneration);
        }
        state
            .fence
            .as_ref()
            .map(|fd| fd.try_clone().map_err(|_| OwnerError::Unknown))
            .transpose()
    }
    pub fn fence_signaled(
        &mut self,
        generation: Generation,
        id: CaptureId,
        lease: LeaseId,
    ) -> Result<(), OwnerError> {
        let state = self.leases.get_mut(&lease).ok_or(OwnerError::Unknown)?;
        if state.generation != generation || state.capture != id || !state.returned {
            return Err(OwnerError::WrongGeneration);
        }
        state.fence_ready = true;
        state.fence = None;
        self.captures
            .get_mut(&id)
            .ok_or(OwnerError::Unknown)?
            .fences_ready
            .insert(lease);
        Ok(())
    }
    pub fn revoke(&mut self) {
        self.publication = false;
        for state in self.leases.values_mut() {
            state.quarantined = true;
        }
    }
    /// Reaping a process supplies no buffer-reset proof. Unknown leases remain held.
    pub fn service_died(&mut self) {
        self.revoke();
    }
    pub fn replace_configuration(&mut self, next: Generation) -> Result<(), OwnerError> {
        self.revoke();
        self.active = next;
        if self
            .leases
            .values()
            .any(|state| state.quarantined || !state.returned || !state.fence_ready)
        {
            return Err(OwnerError::NotReusable);
        }
        self.publication = true;
        Ok(())
    }
    pub fn snapshot(&self, id: CaptureId, lease: LeaseId) -> Result<Snapshot, OwnerError> {
        let capture = self.captures.get(&id).ok_or(OwnerError::Unknown)?;
        let state = self.leases.get(&lease).ok_or(OwnerError::Unknown)?;
        if !capture.leases.contains(&lease) {
            return Err(OwnerError::Unknown);
        }
        let current = state.capture == id && state.generation == capture.generation;
        let all_returned = capture
            .leases
            .iter()
            .all(|key| capture.returned.contains(key));
        let complete =
            all_returned && ((capture.shutter && capture.final_metadata) || capture.failed);
        let reusable = current && state.returned && state.fence_ready && !state.quarantined;
        Ok(Snapshot {
            receipt: capture.receipt,
            returned: capture.returned.contains(&lease),
            capture_complete: complete,
            fence_ready: capture.fences_ready.contains(&lease),
            publication_eligible: self.publication
                && capture.generation == self.active
                && complete
                && reusable
                && !capture.failed
                && !state.failed
                && matches!(capture.receipt, Receipt::Accepted | Receipt::Uncertain),
            reusable,
            quarantined: state.quarantined,
            returns: state.returns,
        })
    }
    pub fn retained_leases(&self) -> usize {
        self.leases.len()
    }
}
