//! Actor-owned permission and connection custody. The provider alone owns routing.
use crate::{
    Reply,
    audio::{Event, Provider, Request, State},
    authority::{Authority, MediaLeaseStamp, MediaStamp},
    boottime_ns,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Unsent,
    Sent,
    ReplyReady,
}
struct Pending {
    stamp: MediaStamp,
    phase: Phase,
}
struct Active {
    connection: u64,
    approved: Option<MediaLeaseStamp>,
    pending: Option<Pending>,
}
#[derive(Default)]
pub(crate) struct Session {
    active: Option<Active>,
}
impl Session {
    pub(crate) fn pending(&self) -> bool {
        self.active.as_ref().is_some_and(|a| a.pending.is_some())
    }
    pub(crate) fn available_to(&self, connection: u64) -> bool {
        self.active
            .as_ref()
            .is_none_or(|a| a.connection == connection && a.pending.is_none())
    }
    pub(crate) fn owns(&self, connection: u64) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.connection == connection)
    }
    pub(crate) fn state(&self, connection: u64, provider: &dyn Provider) -> State {
        let mut state = provider.state();
        if self
            .active
            .as_ref()
            .is_some_and(|a| a.connection != connection)
        {
            state.requested = None;
            state.applied = None;
            state.pending = false;
        }
        state
    }
    pub(crate) fn retire(&mut self, provider: &mut dyn Provider) {
        self.active = None;
        provider.revoke();
    }
    pub(crate) fn begin(
        &mut self,
        connection: u64,
        pid: u32,
        uid: u32,
        request: Request,
        authority: &mut Authority,
        provider: &dyn Provider,
    ) -> Result<u64, Reply> {
        if !self.available_to(connection) {
            return Err(Reply::Denied);
        }
        let state = provider.state();
        if !state.connected || !state.available {
            return Err(Reply::Unavailable);
        }
        if !provider.routes().iter().any(|r| r.id == request.route) {
            return Err(Reply::Denied);
        }
        let stamp = authority.begin_media(
            pid,
            uid,
            provider.incarnation(),
            &request.route,
            request.ttl_ms,
            boottime_ns(),
        )?;
        let until = stamp.intent().until_ns;
        let active = self.active.get_or_insert(Active {
            connection,
            approved: None,
            pending: None,
        });
        active.pending = Some(Pending {
            stamp,
            phase: Phase::Unsent,
        });
        Ok(until)
    }
    /// Fence before consuming provider replies and after every lifecycle update.
    /// A cancelled unsent replacement preserves the earlier approved lease.
    pub(crate) fn fence(
        &mut self,
        authority: &Authority,
        provider: &mut dyn Provider,
        stopping: bool,
        present: impl Fn(u64) -> bool,
    ) -> Option<(u64, Reply)> {
        let active = self.active.as_ref()?;
        let connection = active.connection;
        let pending = active.pending.is_some();
        if stopping
            || !present(connection)
            || !provider.state().connected
            || active
                .approved
                .as_ref()
                .is_some_and(|stamp| !authority.current_media_lease(stamp, provider.incarnation()))
        {
            self.retire(provider);
            return pending.then_some((connection, Reply::Denied));
        }
        if active.pending.as_ref().is_some_and(|p| {
            !authority.current_media(&p.stamp, provider.incarnation(), boottime_ns())
        }) {
            if active.pending.as_ref().unwrap().phase == Phase::Unsent {
                self.active.as_mut().unwrap().pending = None;
            } else {
                // Bytes crossed the boundary; closing withdraws this connection,
                // without claiming they never caused an effect or remote cleanup.
                self.retire(provider);
            }
            return Some((connection, Reply::Denied));
        }
        None
    }
    pub(crate) fn pending_on(&self, connection: u64) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.connection == connection && a.pending.is_some())
    }
    pub(crate) fn needs_drive(&self, connection: u64) -> bool {
        self.active.as_ref().is_some_and(|a| {
            a.connection == connection
                && a.pending
                    .as_ref()
                    .is_some_and(|p| p.phase != Phase::ReplyReady)
        })
    }
    /// Caller has just checked exact owning-client EOF/extra input. No provider
    /// polling path can dispatch; each retry checks the original stamp again.
    pub(crate) fn drive(
        &mut self,
        connection: u64,
        authority: &Authority,
        provider: &mut dyn Provider,
    ) -> Option<Reply> {
        if !self.needs_drive(connection) {
            return None;
        }
        if let Some((_, reply)) = self.fence(authority, provider, false, |_| true) {
            return Some(reply);
        }
        let pending = self.active.as_mut()?.pending.as_mut()?;
        if pending.phase != Phase::Unsent {
            return None;
        }
        match provider.submit(pending.stamp.intent()) {
            Ok(true) => {
                pending.phase = Phase::Sent;
                None
            }
            Ok(false) => None,
            Err(_) => {
                self.retire(provider);
                Some(Reply::Unavailable)
            }
        }
    }
    pub(crate) fn observe(
        &mut self,
        event: Event,
        authority: &Authority,
        provider: &mut dyn Provider,
    ) -> Option<(u64, Reply)> {
        if matches!(&event, Event::Observed) {
            return None;
        }
        if matches!(&event, Event::Unavailable) {
            let connection = self
                .active
                .as_ref()
                .filter(|a| a.pending.is_some())
                .map(|a| a.connection);
            self.retire(provider);
            return connection.map(|c| (c, Reply::Unavailable));
        }
        let active = self.active.as_ref()?;
        let connection = active.connection;
        let pending = active.pending.as_ref()?;
        let id = match &event {
            Event::Selected { id, .. } | Event::Rejected { id } => *id,
            _ => unreachable!(),
        };
        if pending.phase != Phase::Sent || pending.stamp.intent().id != id {
            self.retire(provider);
            return Some((connection, Reply::Unavailable));
        }
        if let Some(reply) = self.fence(authority, provider, false, |_| true) {
            return Some(reply);
        }
        match event {
            Event::Selected { state, .. } => {
                self.active
                    .as_mut()
                    .unwrap()
                    .pending
                    .as_mut()
                    .unwrap()
                    .phase = Phase::ReplyReady;
                Some((connection, Reply::MediaSelected { state }))
            }
            Event::Rejected { .. } => {
                self.active.as_mut().unwrap().pending = None;
                Some((connection, Reply::Unavailable))
            }
            _ => unreachable!(),
        }
    }
    /// A queued positive response is rechecked immediately before every send,
    /// including retries after an actual app-socket EAGAIN.
    pub(crate) fn revalidate_reply(
        &mut self,
        connection: u64,
        authority: &Authority,
        provider: &mut dyn Provider,
        stopping: bool,
    ) -> Option<Reply> {
        if !self.owns(connection) {
            return None;
        }
        self.fence(authority, provider, stopping, |_| true)
            .map(|(_, r)| r)
    }
    pub(crate) fn published(
        &mut self,
        connection: u64,
        authority: &Authority,
        provider: &mut dyn Provider,
    ) {
        let Some(active) = self.active.as_mut().filter(|a| a.connection == connection) else {
            return;
        };
        if !active
            .pending
            .as_ref()
            .is_some_and(|p| p.phase == Phase::ReplyReady)
        {
            return;
        }
        let pending = active.pending.take().unwrap();
        match authority.approve_media(&pending.stamp, provider.incarnation(), boottime_ns()) {
            Some(stamp) => active.approved = Some(stamp),
            None => self.retire(provider),
        }
    }
}
