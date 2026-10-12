//! Single in-flight desktop notification and delayed-reply freshness fencing.
//! This does not send D-Bus messages or grant call-control authority. The owner
//! must retain the original bus connection and close handles by unique owner,
//! never route a handle over a reconnected bus or a replacement server's
//! well-known name. It must also drive independent expiry of visible handles.
use crate::{Identity, observer::Lease, watch::Snapshot};
use std::time::Duration;

pub struct NotifyRequest {
    token: Identity,
    owner: String,
    continuity: Identity,
    expires: Duration,
    presented: Snapshot,
}
impl NotifyRequest {
    pub fn expires(&self) -> Duration {
        self.expires
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn timeout_at(&self, now: Duration) -> Option<i32> {
        let remaining: i32 = self.expires.checked_sub(now)?.as_millis().try_into().ok()?;
        (remaining > 0).then_some(remaining)
    }
}
pub struct NotificationHandle {
    owner: String,
    id: u32,
    expires: Duration,
    presented: Snapshot,
}
impl NotificationHandle {
    /// Exact call episodes represented when this notification was requested.
    /// A delayed dismissal must not suppress calls first observed afterward.
    pub fn presented(&self) -> &Snapshot {
        &self.presented
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn expires(&self) -> Duration {
        self.expires
    }
}
pub enum Completion {
    Visible(NotificationHandle),
    Close(NotificationHandle),
    /// A zero notification ID is invalid and cannot be closed.
    NoNotification,
}
struct Pending {
    token: Identity,
    eligible: bool,
}
#[derive(Default)]
pub struct Notifications {
    pending: Option<Pending>,
}
fn unique_owner(owner: &str) -> bool {
    owner.len() <= 255
        && owner.strip_prefix(':').is_some_and(|name| {
            name.contains('.')
                && name.split('.').all(|part| {
                    !part.is_empty()
                        && part
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                })
        })
}
impl Notifications {
    /// `allowed` combines current lock, suppression and episode-dismissal policy.
    /// The caller must re-evaluate that policy before completing the request.
    pub fn begin(
        &mut self,
        owner: &str,
        lease: &Lease,
        allowed: bool,
        now: Duration,
    ) -> Option<NotifyRequest> {
        if self.pending.is_some() || !allowed || !unique_owner(owner) {
            return None;
        }
        let remaining = lease.expires().checked_sub(now)?;
        let timeout_ms: i32 = remaining.as_millis().try_into().ok()?;
        // Zero means no expiration in the notification protocol. Never emit it,
        // including when a positive sub-millisecond lease rounds down to zero.
        if timeout_ms == 0 {
            return None;
        }
        let token = Identity(uuid::Uuid::new_v4().simple().to_string());
        self.pending = Some(Pending {
            token: token.clone(),
            eligible: true,
        });
        Some(NotifyRequest {
            token,
            owner: owner.into(),
            continuity: lease.continuity().clone(),
            expires: lease.expires(),
            presented: lease.snapshot().clone(),
        })
    }
    pub fn complete(
        &mut self,
        request: NotifyRequest,
        id: u32,
        current_owner: Option<&str>,
        current_lease: Option<&Lease>,
        allowed: bool,
        now: Duration,
    ) -> Completion {
        let ours = self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.token == request.token);
        let eligible = self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.eligible);
        if ours {
            self.pending = None;
        }
        if id == 0 {
            return Completion::NoNotification;
        }
        let valid = ours
            && eligible
            && allowed
            && current_owner == Some(request.owner.as_str())
            && now < request.expires
            && current_lease.is_some_and(|lease| {
                lease.continuity() == &request.continuity && now < lease.expires()
            });
        let handle = NotificationHandle {
            owner: request.owner,
            id,
            expires: request.expires,
            presented: request.presented,
        };
        if valid {
            Completion::Visible(handle)
        } else {
            Completion::Close(handle)
        }
    }
    /// Revoke adoption without freeing capacity or losing late cleanup. Call on
    /// lock/permission loss, bus-owner transitions, suspend and shutdown.
    pub fn invalidate(&mut self) {
        if let Some(pending) = &mut self.pending {
            pending.eligible = false;
        }
    }
    /// Release capacity only when the owner can prove Notify was not sent.
    /// A timeout or error after sending begins is NOT such proof: invalidate
    /// and retain unresolved capacity for late cleanup instead.
    pub fn not_sent(&mut self, request: NotifyRequest) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.token == request.token)
        {
            self.pending = None;
        }
    }
}
