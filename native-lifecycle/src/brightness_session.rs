//! One actor-owned explicit brightness intent and qualified reply publication.
use crate::{
    Reply,
    authority::{Authority, BrightnessStamp},
    boottime_ns,
    brightness::{Event, Provider, Request, State},
};
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Unsent,
    Sent,
    ReplyReady,
    Published,
}
struct Active {
    connection: u64,
    stamp: BrightnessStamp,
    phase: Phase,
    reply_until: u64,
}
#[derive(Default)]
pub(crate) struct Session {
    active: Option<Active>,
}
impl Session {
    pub(crate) fn pending(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.phase != Phase::Published)
    }
    pub(crate) fn owns(&self, c: u64) -> bool {
        self.active.as_ref().is_some_and(|a| a.connection == c)
    }
    pub(crate) fn pending_on(&self, c: u64) -> bool {
        self.owns(c) && self.pending()
    }
    pub(crate) fn available_to(&self, c: u64) -> bool {
        self.active
            .as_ref()
            .is_none_or(|a| a.connection == c && a.phase == Phase::Published)
    }
    pub(crate) fn needs_drive(&self, c: u64) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.connection == c && matches!(a.phase, Phase::Unsent | Phase::Sent))
    }
    pub(crate) fn state(&self, c: u64, p: &dyn Provider) -> State {
        let mut s = p.state().current(boottime_ns());
        if self.active.as_ref().is_some_and(|a| a.connection != c) {
            s.requested_percent = None;
        }
        s
    }
    pub(crate) fn retire(&mut self, p: &mut dyn Provider) {
        self.active = None;
        p.revoke();
    }
    pub(crate) fn begin(
        &mut self,
        c: u64,
        pid: u32,
        uid: u32,
        r: Request,
        a: &mut Authority,
        p: &dyn Provider,
    ) -> Result<u64, Reply> {
        if !self.available_to(c) {
            return Err(Reply::Denied);
        }
        let state = p.state();
        if !state.is_current(boottime_ns()) || state.pending {
            return Err(Reply::Unavailable);
        }
        if r.percent > 100 || r.expected_revision == 0 || r.expected_revision != state.revision {
            return Err(Reply::Denied);
        }
        let stamp = a.begin_brightness(pid, uid, p.incarnation(), r, boottime_ns())?;
        let until = stamp.intent().until_ns;
        self.active = Some(Active {
            connection: c,
            stamp,
            phase: Phase::Unsent,
            reply_until: 0,
        });
        Ok(until)
    }
    pub(crate) fn fence(
        &mut self,
        a: &Authority,
        p: &mut dyn Provider,
        stopping: bool,
        present: impl Fn(u64) -> bool,
    ) -> Option<(u64, Reply)> {
        let active = self.active.as_ref()?;
        let c = active.connection;
        let pending = active.phase != Phase::Published;
        let state = p.state();
        let now = boottime_ns();
        let unsent_stale = active.phase == Phase::Unsent
            && (!state.is_current(now)
                || state.pending
                || state.revision != active.stamp.intent().expected_revision);
        let readback_expired = active.phase == Phase::ReplyReady && now >= active.reply_until;
        if stopping
            || !present(c)
            || !state.connected
            || unsent_stale
            || readback_expired
            || !a.current_brightness(&active.stamp, p.incarnation(), now)
        {
            self.retire(p);
            return pending.then_some((c, Reply::Denied));
        }
        None
    }
    pub(crate) fn drive(&mut self, c: u64, a: &Authority, p: &mut dyn Provider) -> Option<Reply> {
        if !self.needs_drive(c) {
            return None;
        }
        if let Some((_, r)) = self.fence(a, p, false, |_| true) {
            return Some(r);
        }
        let active = self.active.as_mut()?;
        if active.phase != Phase::Unsent {
            return None;
        }
        match p.submit(active.stamp.intent()) {
            Ok(true) => {
                active.phase = Phase::Sent;
                None
            }
            _ => {
                self.retire(p);
                Some(Reply::Unavailable)
            }
        }
    }
    pub(crate) fn observe(
        &mut self,
        event: Event,
        a: &Authority,
        p: &mut dyn Provider,
    ) -> Option<(u64, Reply)> {
        if matches!(&event, Event::Observed) {
            return None;
        }
        if matches!(&event, Event::Unavailable) {
            let c = self
                .active
                .as_ref()
                .filter(|a| a.phase != Phase::Published)
                .map(|a| a.connection);
            self.retire(p);
            return c.map(|c| (c, Reply::Unavailable));
        }
        let active = self.active.as_ref()?;
        let c = active.connection;
        let id = match &event {
            Event::Completed { id, .. } | Event::Rejected { id } => *id,
            _ => unreachable!(),
        };
        if active.phase != Phase::Sent || active.stamp.intent().id != id {
            self.retire(p);
            return Some((c, Reply::Unavailable));
        }
        if let Some(reply) = self.fence(a, p, false, |_| true) {
            return Some(reply);
        }
        match event {
            Event::Completed { state, .. } => {
                let intent = self.active.as_ref().unwrap().stamp.intent();
                if !state.is_current(boottime_ns())
                    || state.pending
                    || state.revision <= intent.expected_revision
                    || state.requested_percent != Some(intent.percent)
                {
                    self.retire(p);
                    return Some((c, Reply::Unavailable));
                }
                let active = self.active.as_mut().unwrap();
                active.reply_until = state
                    .until_ns
                    .parse()
                    .expect("validated canonical deadline");
                active.phase = Phase::ReplyReady;
                Some((c, Reply::BrightnessCompleted { state }))
            }
            Event::Rejected { .. } => {
                self.retire(p);
                Some((c, Reply::Unavailable))
            }
            _ => unreachable!(),
        }
    }
    pub(crate) fn revalidate_reply(
        &mut self,
        c: u64,
        a: &Authority,
        p: &mut dyn Provider,
        stopping: bool,
    ) -> Option<Reply> {
        if !self.owns(c) {
            return None;
        }
        self.fence(a, p, stopping, |_| true).map(|(_, r)| r)
    }
    pub(crate) fn published(&mut self, c: u64) {
        if let Some(active) = self
            .active
            .as_mut()
            .filter(|a| a.connection == c && a.phase == Phase::ReplyReady)
        {
            active.phase = Phase::Published;
        }
    }
}
