//! Actor-owned permission and connection custody. The provider alone owns routing.
use crate::{
    Reply,
    audio::{Event, PreviewRequest, PreviewState, Provider, Request, State},
    authority::{Authority, MediaLeaseStamp, MediaStamp},
    boottime_ns,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Unsent,
    Sent,
    ReplyReady,
}
enum Kind {
    Effect { stamp: MediaStamp, preview: bool },
    Stop { id: u64, until: u64 },
}
struct Pending {
    kind: Kind,
    phase: Phase,
}
impl Pending {
    fn id(&self) -> u64 {
        match &self.kind {
            Kind::Effect { stamp, .. } => stamp.intent().id,
            Kind::Stop { id, .. } => *id,
        }
    }
    fn valid(&self, authority: &Authority, backend: &str) -> bool {
        match &self.kind {
            Kind::Effect { stamp, .. } => authority.current_media(stamp, backend, boottime_ns()),
            Kind::Stop { until, .. } => boottime_ns() < *until,
        }
    }
}
struct Active {
    connection: u64,
    approved: Option<MediaLeaseStamp>,
    pending: Option<Pending>,
}
#[derive(Default)]
pub(crate) struct Session {
    active: Option<Active>,
    cleanup_sequence: u64,
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
            state.policy_settled = false;
            state.preview = PreviewState::Idle;
        }
        state
    }
    pub(crate) fn retire(&mut self, provider: &mut dyn Provider) {
        self.active = None;
        provider.revoke();
    }
    fn check(&self, connection: u64, route: &str, provider: &dyn Provider) -> Result<(), Reply> {
        if !self.available_to(connection) {
            return Err(Reply::Denied);
        }
        let state = provider.state();
        if !state.connected || !state.available {
            return Err(Reply::Unavailable);
        }
        if !provider.routes().iter().any(|r| r.id == route) {
            return Err(Reply::Denied);
        }
        Ok(())
    }
    fn install(&mut self, connection: u64, stamp: MediaStamp, preview: bool) -> u64 {
        let until = stamp.intent().until_ns;
        let active = self.active.get_or_insert(Active {
            connection,
            approved: None,
            pending: None,
        });
        active.pending = Some(Pending {
            kind: Kind::Effect { stamp, preview },
            phase: Phase::Unsent,
        });
        until
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
        self.check(connection, &request.route, provider)?;
        let stamp = authority.begin_media(
            pid,
            uid,
            provider.incarnation(),
            &request.route,
            request.ttl_ms,
            boottime_ns(),
        )?;
        Ok(self.install(connection, stamp, false))
    }
    pub(crate) fn begin_preview(
        &mut self,
        connection: u64,
        pid: u32,
        uid: u32,
        request: PreviewRequest,
        authority: &mut Authority,
        provider: &dyn Provider,
    ) -> Result<u64, Reply> {
        self.check(connection, &request.route, provider)?;
        let stamp = authority.begin_media_preview(
            pid,
            uid,
            provider.incarnation(),
            &request,
            boottime_ns(),
        )?;
        Ok(self.install(connection, stamp, true))
    }
    /// Cleanup belongs to the existing connection, including in the background.
    /// No new playback stamp or grant is minted. Existing lease invalidation
    /// still closes the provider before any later response can publish.
    pub(crate) fn stop_preview(&mut self, connection: u64) -> Result<u64, Reply> {
        if !self.owns(connection) || !self.available_to(connection) {
            return Err(Reply::Denied);
        }
        let until = boottime_ns()
            .checked_add(1_000_000_000)
            .ok_or(Reply::Unavailable)?;
        self.cleanup_sequence = self
            .cleanup_sequence
            .checked_add(1)
            .ok_or(Reply::Unavailable)?;
        self.active.as_mut().unwrap().pending = Some(Pending {
            kind: Kind::Stop {
                id: self.cleanup_sequence,
                until,
            },
            phase: Phase::Unsent,
        });
        Ok(until)
    }
    /// Fence before consuming provider replies and after every lifecycle update.
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
        if active
            .pending
            .as_ref()
            .is_some_and(|p| !p.valid(authority, provider.incarnation()))
        {
            let p = active.pending.as_ref().unwrap();
            if p.phase == Phase::Unsent && matches!(&p.kind, Kind::Effect { .. }) {
                // An unsent replacement preserves the earlier approved lease.
                self.active.as_mut().unwrap().pending = None;
            } else {
                // Expired cleanup also withdraws ownership; it cannot disappear
                // behind a negative read-only status or leave playback running.
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
    /// Caller has just checked exact owning-client EOF/extra input. Poll cannot dispatch.
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
        let result = match &pending.kind {
            Kind::Effect {
                stamp,
                preview: false,
            } => provider.submit(stamp.intent()),
            Kind::Effect {
                stamp,
                preview: true,
            } => provider.play(stamp.intent()),
            Kind::Stop { id, until } => provider.stop_preview(*id, *until),
        };
        match result {
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
            Event::Selected { id, .. } | Event::Preview { id, .. } | Event::Rejected { id } => *id,
            _ => unreachable!(),
        };
        let matching_kind = matches!(
            (&pending.kind, &event),
            (Kind::Effect { preview: false, .. }, Event::Selected { .. })
                | (
                    Kind::Effect { preview: true, .. } | Kind::Stop { .. },
                    Event::Preview { .. }
                )
                | (_, Event::Rejected { .. })
        );
        if pending.phase != Phase::Sent || pending.id() != id || !matching_kind {
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
            Event::Preview { state, .. } => {
                self.active
                    .as_mut()
                    .unwrap()
                    .pending
                    .as_mut()
                    .unwrap()
                    .phase = Phase::ReplyReady;
                Some((connection, Reply::MediaPreview { state }))
            }
            Event::Rejected { .. } => {
                if matches!(
                    self.active.as_ref().unwrap().pending.as_ref().unwrap().kind,
                    Kind::Stop { .. }
                ) {
                    self.retire(provider);
                } else {
                    self.active.as_mut().unwrap().pending = None;
                }
                Some((connection, Reply::Unavailable))
            }
            _ => unreachable!(),
        }
    }
    /// Recheck immediately before every app send, including real EAGAIN retries.
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
        match active.pending.take().unwrap().kind {
            Kind::Effect { stamp, .. } => {
                match authority.approve_media(&stamp, provider.incarnation(), boottime_ns()) {
                    Some(stamp) => active.approved = Some(stamp),
                    None => self.retire(provider),
                }
            }
            Kind::Stop { .. } => {}
        }
    }
}
