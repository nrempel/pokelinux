pub mod backend_process;
pub mod pipewire;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Media,
    Call,
    Capture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Identity {
    pub uid: u32,
    pub connection: u64,
}

struct Lease {
    owner: Identity,
    role: Role,
    route: String,
}

pub struct Authority {
    call_uid: u32,
    app_uid: u32,
    capture_allowed: bool,
    next_id: u64,
    leases: BTreeMap<u64, Lease>,
}

impl Authority {
    pub fn new(call_uid: u32, app_uid: u32, capture_allowed: bool) -> Self {
        Self {
            call_uid,
            app_uid,
            capture_allowed,
            next_id: 1,
            leases: BTreeMap::new(),
        }
    }

    pub fn acquire(
        &mut self,
        peer: Identity,
        role: Role,
        route: &str,
    ) -> Result<u64, &'static str> {
        if peer.uid != self.call_uid && peer.uid != self.app_uid {
            return Err("denied");
        }
        if role == Role::Call && peer.uid != self.call_uid {
            return Err("denied");
        }
        if role == Role::Capture && !self.capture_allowed {
            return Err("capture_revoked");
        }
        // Applications share a bounded budget; they cannot consume call-owner capacity.
        if self.leases.len() >= 64
            || (peer.uid == self.app_uid
                && self
                    .leases
                    .values()
                    .filter(|lease| lease.owner.uid == self.app_uid)
                    .count()
                    >= 32)
        {
            return Err("lease_limit");
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("lease_limit")?;
        self.leases.insert(
            id,
            Lease {
                owner: peer,
                role,
                route: route.into(),
            },
        );
        Ok(id)
    }

    pub fn release(&mut self, peer: Identity, id: u64) -> Result<(), &'static str> {
        if self.leases.get(&id).is_none_or(|lease| lease.owner != peer) {
            return Err("denied");
        }
        self.leases.remove(&id);
        Ok(())
    }

    pub fn role(&self, peer: Identity, id: u64) -> Result<Role, &'static str> {
        let lease = self.leases.get(&id).ok_or("unknown_lease")?;
        if lease.owner != peer {
            return Err("denied");
        }
        Ok(lease.role)
    }

    pub fn set_route(&mut self, peer: Identity, id: u64, route: &str) -> Result<(), &'static str> {
        let lease = self.leases.get_mut(&id).ok_or("unknown_lease")?;
        if lease.owner != peer {
            return Err("denied");
        }
        lease.route = route.into();
        Ok(())
    }

    pub fn revoke_capture(&mut self, peer: Identity) -> Result<(), &'static str> {
        self.set_capture_permission(peer, false)
    }

    pub fn set_capture_permission(
        &mut self,
        peer: Identity,
        allowed: bool,
    ) -> Result<(), &'static str> {
        if peer.uid != self.call_uid {
            return Err("denied");
        }
        self.capture_allowed = allowed;
        if !allowed {
            self.leases.retain(|_, lease| lease.role != Role::Capture);
        }
        Ok(())
    }

    pub fn is_privileged(&self, peer: Identity) -> bool {
        peer.uid == self.call_uid
    }
    pub fn is_allowed(&self, peer: Identity) -> bool {
        peer.uid == self.call_uid || peer.uid == self.app_uid
    }

    pub fn desired(&self) -> Vec<(u64, Role, String)> {
        let playback = self
            .leases
            .iter()
            .rev()
            .find(|(_, lease)| lease.role == Role::Call)
            .or_else(|| {
                self.leases
                    .iter()
                    .rev()
                    .find(|(_, lease)| lease.role == Role::Media)
            });
        let mut result = Vec::new();
        if let Some((id, lease)) = playback {
            result.push((*id, lease.role, lease.route.clone()));
        }
        result.extend(
            self.leases
                .iter()
                .filter(|(_, lease)| lease.role == Role::Capture)
                .map(|(id, lease)| (*id, lease.role, lease.route.clone())),
        );
        result
    }

    pub fn cancel_all(&mut self) {
        self.capture_allowed = false;
        self.leases.clear();
    }

    pub fn disconnect(&mut self, peer: Identity) -> bool {
        let before = self.leases.len();
        self.leases.retain(|_, lease| lease.owner != peer);
        self.leases.len() != before
    }
}

pub fn validate_service_uids(owner: u32, call: u32, app: u32) -> bool {
    owner != call && owner != app && call != app
}
