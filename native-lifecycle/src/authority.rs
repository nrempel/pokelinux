use crate::storage::{Durable, Grant};
use crate::{AppProfile, AppRequest, MAX_PERMITS, MAX_RECEIPTS, Permission, Reply, fresh_id};
use rustix::{
    event::{PollFd, PollFlags, poll},
    fd::OwnedFd,
};
use std::{collections::BTreeMap, io};

struct Principal {
    uid: u32,
    pid: u32,
    generation: u64,
    background: bool,
    background_intent: bool,
    alive: OwnedFd,
}
/// Internal, non-cloneable stamp; never serialized or supplied by an app.
pub(crate) struct ReadStamp {
    app: String,
    generation: u64,
    pid: u32,
    uid: u32,
    policy: u64,
    lifecycle: u64,
    backend: String,
    permission: Permission,
    until: u64,
    session: Option<SessionRead>,
}
/// Effect authority stays inside this actor, separate from durable history.
/// It is neither cloneable nor serializable and cannot be reconstructed by an app.
pub(crate) struct CaptureStamp {
    authority: ReadStamp,
    intent: crate::camera::Intent,
}
impl CaptureStamp {
    pub(crate) fn intent(&self) -> crate::camera::Intent {
        self.intent
    }
}
/// A pending media change: current foreground authority, never an app token.
pub(crate) struct MediaStamp {
    authority: ReadStamp,
    intent: crate::audio::Intent,
    revocation: u64,
}
pub(crate) type MediaIntent = crate::audio::Intent;
/// Approved media may continue in the background. It still belongs to this
/// exact live app generation, backend connection and unrevoked route grant.
pub(crate) struct MediaLeaseStamp {
    app: String,
    generation: u64,
    pid: u32,
    uid: u32,
    backend: String,
    revocation: u64,
}
impl MediaStamp {
    pub(crate) fn intent(&self) -> &MediaIntent {
        &self.intent
    }
}
/// One explicit brightness effect, scoped to current policy/session and backend.
pub(crate) struct BrightnessStamp {
    authority: ReadStamp,
    intent: crate::brightness::Intent,
}
impl BrightnessStamp {
    pub(crate) fn intent(&self) -> &crate::brightness::Intent {
        &self.intent
    }
}
struct SessionRead {
    source: String,
    revision: u64,
    until: u64,
}
struct Permit {
    app: String,
    generation: u64,
    policy: u64,
    lifecycle: u64,
    permission: Permission,
    until: u64,
}
pub(crate) enum Gate {
    Grant,
    Lock,
    RequiredLock,
    Restore {
        attempt: u64,
        lock_epoch: u64,
    },
    Admit {
        app: String,
        generation: u64,
        permission: Permission,
        receipt: String,
    },
}
pub(crate) struct WriteIntent {
    pub next: Durable,
    pub until: u64,
    policy: u64,
    lifecycle: u64,
    gate: Gate,
}
pub(crate) enum Decision {
    Immediate(Reply),
    Persist(WriteIntent),
}
const MAX_AUTHENTICATION_ATTEMPT_NS: u64 = 30_000_000_000;
/// Inert, non-cloneable identity for one actor-owned attempt. It supplies no
/// worker dispatch, authentication result, receipt or restoration authority.
// Unwired internal primitive until an independently qualified broker exists.
#[allow(dead_code)]
pub(crate) struct AuthenticationAttempt {
    actor: String,
    id: u64,
}
/// Capacity-only closure stamp, never an authentication result. Its fields and
/// construction are private; there is no production producer before the actual
/// bound broker/whole-owner closure boundary is independently qualified.
#[allow(dead_code)]
pub(crate) struct ClosedAuthenticationAttempt {
    actor: String,
    id: u64,
    until: u64,
}
#[cfg(test)]
impl ClosedAuthenticationAttempt {
    /// Authored closure facts only. This verifies no broker/process cleanup.
    pub(crate) fn fixture(attempt: &AuthenticationAttempt, until: u64) -> Self {
        Self {
            actor: attempt.actor.clone(),
            id: attempt.id,
            until,
        }
    }
}
/// Sealed authenticated-and-closed result. No production constructor or ingress
/// exists until the real whole-owner authentication boundary is qualified.
#[allow(dead_code)]
pub(crate) struct AuthenticatedCompletion {
    closed: ClosedAuthenticationAttempt,
}
#[cfg(test)]
impl AuthenticatedCompletion {
    /// Authored authentication/closure facts only; no PAM or cleanup evidence.
    pub(crate) fn fixture(attempt: &AuthenticationAttempt, until: u64) -> Self {
        Self {
            closed: ClosedAuthenticationAttempt::fixture(attempt, until),
        }
    }
}
#[derive(PartialEq)]
enum RestorationPhase {
    Armed,
    Clearing,
}
struct RestorationSlot {
    context: AuthenticationSlot,
    armed_at: u64,
    after_sequence: u64,
    phase: RestorationPhase,
}
struct AuthenticationSlot {
    id: u64,
    source: String,
    generation: u64,
    locker: crate::session::OwnedFocus,
    lock_epoch: u64,
    until: u64,
    retired: bool,
}
pub struct Authority {
    profiles: Vec<AppProfile>,
    principals: BTreeMap<String, Principal>,
    permits: BTreeMap<String, Permit>,
    pub(crate) durable: Durable,
    incarnation: String,
    sequence: u64,
    generation: u64,
    policy: u64,
    lifecycle: u64,
    locked: bool,
    suspended: bool,
    degraded: bool,
    ready: bool,
    session_required: bool,
    session: Option<crate::session::Snapshot>,
    session_source: Option<String>,
    session_failed: bool,
    source_observation: crate::session::SourceObservation,
    root_lock_intent: bool,
    reconcile_after: u64,
    session_view: Option<pokelinux_kernel_identity::ProcessView>,
    lock_admission_epoch: u64,
    authentication_sequence: u64,
    authentication: Option<AuthenticationSlot>,
    restoration: Option<RestorationSlot>,
    required_lock: Option<(u64, bool)>,
    media_revocations: BTreeMap<String, u64>,
}
impl Authority {
    pub(crate) fn loading(profiles: Vec<AppProfile>) -> io::Result<Self> {
        Ok(Self {
            profiles,
            principals: BTreeMap::new(),
            permits: BTreeMap::new(),
            durable: Durable::default(),
            incarnation: fresh_id()?,
            sequence: 0,
            generation: 0,
            policy: 0,
            lifecycle: 0,
            locked: true,
            suspended: false,
            degraded: false,
            ready: false,
            session_required: false,
            session: None,
            session_source: None,
            session_failed: false,
            source_observation: crate::session::SourceObservation::default(),
            root_lock_intent: true,
            reconcile_after: 0,
            session_view: None,
            lock_admission_epoch: 0,
            authentication_sequence: 0,
            authentication: None,
            restoration: None,
            required_lock: None,
            media_revocations: BTreeMap::new(),
        })
    }
    pub(crate) fn require_session(&mut self) -> io::Result<()> {
        self.session_view = Some(pokelinux_kernel_identity::ProcessView::open()?);
        self.session_required = true;
        self.source_observation.configure();
        self.locked = true;
        for p in self.principals.values_mut() {
            p.background = true;
        }
        self.lifecycle(None, None, None)
    }
    fn fresh_session(&self, now: u64) -> Option<&crate::session::Snapshot> {
        self.session.as_ref().filter(|s| {
            !self.session_failed && s.sampled_at_ns > self.reconcile_after && now < s.until
        })
    }
    fn retire_authentication_attempt(&mut self) {
        if self
            .restoration
            .as_ref()
            .is_some_and(|r| r.phase == RestorationPhase::Armed)
        {
            // Authentication already proved closure. No physical owner remains;
            // dropping this retired result never creates a new result or lease.
            self.restoration = None;
        } else if let Some(restoration) = &mut self.restoration {
            // A submitted write stays owned until its actual completion.
            restoration.context.retired = true;
        }
        if let Some(attempt) = &mut self.authentication {
            // Retirement is sticky and never supplies proof of broker closure.
            // Keep the slot and locker until the separate matched closure path.
            attempt.retired = true;
        }
    }
    fn authentication_context_current(
        &self,
        attempt: &AuthenticationSlot,
        sample: &crate::session::Snapshot,
        now: u64,
    ) -> bool {
        self.ready()
            && self.session_required
            && !self.suspended
            && self.locked
            && self.root_lock_intent
            && self.durable.lock_pending
            && !self.session_failed
            && !attempt.retired
            && self.lock_admission_epoch == attempt.lock_epoch
            && now < attempt.until
            && sample.sampled_at_ns > self.reconcile_after
            && now < sample.until
            && sample.source == attempt.source
            && sample.locked
            && sample.lock_generation == Some(attempt.generation)
            && sample.protocol_unlock.is_none()
            && sample.locker.as_ref().is_some_and(|locker| {
                // Shared Arc identity or matching metadata alone cannot prove
                // that either retained descriptor still names this live task.
                attempt.locker.metadata == locker.metadata
                    && self.session_view.as_ref().is_some_and(|view| {
                        view.matches(
                            attempt.locker.pidfd(),
                            locker.pidfd(),
                            attempt.locker.metadata.pid,
                        )
                        .unwrap_or(false)
                    })
            })
            // Identity reads/polls may consume time. Recheck both absolute
            // limits after the final kernel liveness check as well.
            && crate::boottime_ns() < attempt.until
            && crate::boottime_ns() < sample.until
    }
    fn refresh_authentication_lifetime(&mut self, now: u64) {
        let now = now.max(crate::boottime_ns());
        if self.restoration.as_ref().is_some_and(|restoration| {
            self.session
                .as_ref()
                .is_none_or(|sample| !self.restoration_context_current(restoration, sample, now))
        }) {
            self.retire_authentication_attempt();
        }
        if self.authentication.as_ref().is_some_and(|attempt| {
            self.session
                .as_ref()
                .is_none_or(|sample| !self.authentication_context_current(attempt, sample, now))
        }) {
            self.retire_authentication_attempt();
        }
    }
    /// Reserve one inert handle with its original absolute BOOTTIME deadline.
    /// Occupancy persists after expiry/retirement until a separate trusted
    /// closure stamp releases it; this primitive creates no cleanup evidence.
    #[allow(dead_code)] // Deliberately not wired to production actor ingress yet.
    pub(crate) fn begin_authentication_attempt(
        &mut self,
        now: u64,
        until: u64,
    ) -> Result<AuthenticationAttempt, Reply> {
        if !self.ready()
            || self.authentication.is_some()
            || self.restoration.is_some()
            || self.required_lock.is_some()
        {
            return Err(Reply::Unavailable);
        }
        let actual_now = crate::boottime_ns();
        if now > actual_now
            || actual_now >= until
            || until
                .checked_sub(now)
                .is_none_or(|budget| budget == 0 || budget > MAX_AUTHENTICATION_ATTEMPT_NS)
        {
            return Err(Reply::Denied);
        }
        let id = match self.authentication_sequence.checked_add(1) {
            Some(id) => id,
            None => {
                self.invalidate();
                return Err(Reply::Unavailable);
            }
        };
        let sample = self.fresh_session(actual_now).ok_or(Reply::Denied)?;
        let attempt = AuthenticationSlot {
            id,
            source: sample.source.clone(),
            generation: sample
                .lock_generation
                .filter(|g| *g != 0)
                .ok_or(Reply::Denied)?,
            locker: sample.locker.clone().ok_or(Reply::Denied)?,
            lock_epoch: self.lock_admission_epoch,
            until,
            retired: false,
        };
        if !self.authentication_context_current(&attempt, sample, actual_now) {
            return Err(Reply::Denied);
        }
        self.authentication_sequence = attempt.id;
        let handle = AuthenticationAttempt {
            actor: self.incarnation.clone(),
            id: attempt.id,
        };
        self.authentication = Some(attempt);
        Ok(handle)
    }
    /// Release only matching, independently closed capacity. Expiry or a lost
    /// Source cannot keep already proven resources occupied, but release never
    /// renews Source, starts an attempt, arms unlock or changes durable policy.
    /// A future producer must prove whole-owner closure even after the original
    /// result authority expires; an expiring result token alone is insufficient.
    #[allow(dead_code)] // No production completion producer or ingress yet.
    pub(crate) fn release_closed_authentication(
        &mut self,
        closed: ClosedAuthenticationAttempt,
    ) -> bool {
        let matches = closed.actor == self.incarnation
            && self
                .authentication
                .as_ref()
                .is_some_and(|slot| slot.id == closed.id && slot.until == closed.until);
        if matches {
            // Preserve sequence, lock epoch, retirement/Source state, grants,
            // history and pending intent. New begin performs its own admission.
            self.authentication = None;
        }
        matches
    }
    /// Check and latch lifetime only. A current handle is not an authentication
    /// result and cannot arm a candidate or clear durable pending intent.
    #[allow(dead_code)] // Deliberately not wired to production actor ingress yet.
    pub(crate) fn current_authentication_attempt(
        &mut self,
        attempt: &AuthenticationAttempt,
        now: u64,
    ) -> bool {
        if attempt.actor != self.incarnation
            || self
                .authentication
                .as_ref()
                .is_none_or(|slot| slot.id != attempt.id)
        {
            return false;
        }
        self.refresh_authentication_lifetime(now);
        self.authentication
            .as_ref()
            .is_some_and(|slot| !slot.retired)
    }
    fn restoration_context_current(
        &self,
        restoration: &RestorationSlot,
        sample: &crate::session::Snapshot,
        now: u64,
    ) -> bool {
        let attempt = &restoration.context;
        self.ready()
            && self.session_required
            && !self.suspended
            && self.locked
            && self.root_lock_intent
            && self.durable.lock_pending
            && !self.session_failed
            && !attempt.retired
            && self.lock_admission_epoch == attempt.lock_epoch
            && now < attempt.until
            && sample.sampled_at_ns > self.reconcile_after
            && now < sample.until
            && sample.source == attempt.source
            && if sample.locked {
                sample.lock_generation == Some(attempt.generation)
                    && sample.protocol_unlock.is_none()
                    && sample.locker.as_ref().is_some_and(|locker| {
                        // Historical retained identity permits the exact protocol
                        // event to arrive after locker exit. Death is no event.
                        attempt.locker.same_reference(locker)
                            || self
                                .session_view
                                .as_ref()
                                .is_some_and(|view| attempt.locker.equivalent(locker, view))
                    })
            } else {
                Self::restoration_event_matches(restoration, sample)
            }
            && crate::boottime_ns() < attempt.until
            && crate::boottime_ns() < sample.until
    }
    fn restoration_event_matches(
        restoration: &RestorationSlot,
        sample: &crate::session::Snapshot,
    ) -> bool {
        !sample.locked
            && sample.protocol_unlock.as_ref().is_some_and(|event| {
                event.generation == restoration.context.generation
                    && event.sequence > restoration.after_sequence
                    && event.observed_at_ns > restoration.armed_at
            })
    }
    /// Consume the exact closed authenticated result and retain its original
    /// context atomically. Capacity release is deliberately a separate type.
    #[allow(dead_code)] // No qualified production producer or actor ingress yet.
    pub(crate) fn arm_authentication(
        &mut self,
        completion: AuthenticatedCompletion,
        now: u64,
    ) -> bool {
        let closed = completion.closed;
        if closed.actor != self.incarnation
            || self
                .authentication
                .as_ref()
                .is_none_or(|slot| slot.id != closed.id || slot.until != closed.until)
        {
            return false;
        }
        self.refresh_authentication_lifetime(now);
        let actual_now = crate::boottime_ns();
        let accepted = now <= actual_now
            && self.authentication.as_ref().is_some_and(|attempt| {
                self.session.as_ref().is_some_and(|sample| {
                    self.authentication_context_current(attempt, sample, actual_now)
                })
            });
        // The matched completion owns whole-owner closure even when its authority
        // expired. No release-then-relookup can lose the accepted context.
        let context = self.authentication.take().unwrap();
        if !accepted {
            return false;
        }
        let sample = self.session.as_ref().unwrap();
        let armed_at = crate::boottime_ns();
        if armed_at >= context.until || armed_at >= sample.until {
            return false;
        }
        self.restoration = Some(RestorationSlot {
            context,
            armed_at,
            after_sequence: sample.sequence,
            phase: RestorationPhase::Armed,
        });
        true
    }
    pub(crate) fn plan_authenticated_clear(&mut self, now: u64) -> Decision {
        self.refresh_authentication_lifetime(now);
        let Some(restoration) = self.restoration.as_mut() else {
            return Decision::Immediate(Reply::Denied);
        };
        if restoration.phase != RestorationPhase::Armed
            || !self
                .session
                .as_ref()
                .is_some_and(|sample| Self::restoration_event_matches(restoration, sample))
        {
            return Decision::Immediate(Reply::Denied);
        }
        restoration.phase = RestorationPhase::Clearing;
        let mut next = self.durable.clone();
        next.lock_pending = false;
        Decision::Persist(WriteIntent {
            next,
            until: restoration
                .context
                .until
                .min(now.saturating_add(1_000_000_000)),
            policy: self.policy,
            lifecycle: self.lifecycle,
            gate: Gate::Restore {
                attempt: restoration.context.id,
                lock_epoch: restoration.context.lock_epoch,
            },
        })
    }
    fn matches_focus(&self, p: &Principal, focus: &crate::session::OwnedFocus) -> bool {
        let metadata = &focus.metadata;
        p.pid == metadata.pid
            && p.uid == metadata.uid
            && metadata.gid == p.uid
            && self.session_view.as_ref().is_some_and(|view| {
                view.matches(&p.alive, focus.pidfd(), p.pid)
                    .unwrap_or(false)
            })
    }
    fn same_peer(
        &self,
        left: Option<&crate::session::OwnedFocus>,
        right: Option<&crate::session::OwnedFocus>,
    ) -> bool {
        match (left, right) {
            (None, None) => true,
            (Some(left), Some(right)) => {
                left.same_reference(right)
                    || self
                        .session_view
                        .as_ref()
                        .is_some_and(|view| left.equivalent(right, view))
            }
            _ => false,
        }
    }
    pub(crate) fn source_provider_failed(
        &mut self,
        kind: io::ErrorKind,
        rejection: Option<crate::session::SourceRejection>,
    ) {
        self.source_observation.fail(
            crate::session::SourceFailureCategory::ProviderError,
            Some(kind),
            rejection,
        );
    }
    pub(crate) fn source_stopping(&mut self) {
        self.source_observation.stopping();
    }
    pub(crate) fn fail_session(&mut self) -> io::Result<()> {
        self.source_observation.fail(
            crate::session::SourceFailureCategory::AuthorityRejected,
            None,
            None,
        );
        self.retire_authentication_attempt();
        if !self.session_failed {
            self.session_failed = true;
            self.session = None;
            self.lifecycle(None, None, None)?;
        }
        self.locked = true;
        for p in self.principals.values_mut() {
            p.background = true;
        }
        Ok(())
    }
    pub(crate) fn missing_session(&mut self) -> io::Result<()> {
        // Explicit resume discards the current sample, not source establishment.
        // None after an established source is terminal even during reconciliation.
        if self.session_source.is_some() {
            self.source_observation.fail(
                crate::session::SourceFailureCategory::MissingAfterEstablished,
                None,
                None,
            );
            return self.fail_session();
        }
        self.locked = true;
        for p in self.principals.values_mut() {
            p.background = true;
        }
        Ok(())
    }
    pub(crate) fn observe_session(
        &mut self,
        s: crate::session::Snapshot,
        now: u64,
    ) -> io::Result<()> {
        // Check the previous lease/liveness before a later observation can
        // replace it, even when no independent timer or current query ran.
        self.refresh_authentication_lifetime(now);
        if self.session_failed {
            return self.fail_session();
        }
        let canonical = s.source.len() == 32
            && s.source
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !canonical
            || s.revision == 0
            || s.sequence == 0
            || s.sampled_at_ns > now
            || s.sampled_at_ns
                .checked_add(crate::session::SAMPLE_BUDGET_NS)
                != Some(s.until)
            || now >= s.until
            || (s.locked && s.focus.is_some())
            || (!s.locked && s.locker.is_some())
            || !crate::session::valid_protocol_facts(
                s.locked,
                s.sequence,
                s.sampled_at_ns,
                s.lock_generation,
                s.protocol_unlock.as_ref(),
            )
            || self
                .session_source
                .as_ref()
                .is_some_and(|id| id != &s.source)
            || self.session.as_ref().is_some_and(|old| {
                s.sequence < old.sequence
                    || s.revision < old.revision
                    || s.sampled_at_ns < old.sampled_at_ns
                    || (s.sequence == old.sequence
                        && (s.revision != old.revision
                            || s.sampled_at_ns != old.sampled_at_ns
                            || s.until != old.until
                            || s.locked != old.locked
                            || !self.same_peer(s.focus.as_ref(), old.focus.as_ref())
                            || !self.same_peer(s.locker.as_ref(), old.locker.as_ref())
                            || s.lock_generation != old.lock_generation
                            || s.protocol_unlock != old.protocol_unlock))
                    || (s.revision == old.revision
                        && (s.locked != old.locked
                            || s.focus.as_ref().map(|f| &f.metadata)
                                != old.focus.as_ref().map(|f| &f.metadata)
                            || !self.same_peer(s.locker.as_ref(), old.locker.as_ref())
                            || s.lock_generation != old.lock_generation
                            || s.protocol_unlock != old.protocol_unlock))
            })
        {
            return self.fail_session();
        }
        // Explicit resume/session reconciliation needs a newly sampled packet.
        // Cached but otherwise fresh pre-resume facts cannot restore authority.
        if s.sampled_at_ns <= self.reconcile_after {
            self.retire_authentication_attempt();
            self.session = None;
            self.locked = true;
            for p in self.principals.values_mut() {
                p.background = true;
            }
            return Ok(());
        }
        if self.authentication.as_ref().is_some_and(|attempt| {
            !self.authentication_context_current(attempt, &s, now.max(crate::boottime_ns()))
        }) {
            self.retire_authentication_attempt();
        }
        if self.restoration.as_ref().is_some_and(|restoration| {
            !self.restoration_context_current(restoration, &s, now.max(crate::boottime_ns()))
        }) {
            self.retire_authentication_attempt();
        }
        let locked = self.root_lock_intent || s.locked;
        let selected = if !locked {
            s.focus.as_ref().and_then(|focus| {
                self.principals
                    .iter()
                    .find(|(_, p)| self.matches_focus(p, focus))
                    .map(|(app, _)| app.clone())
            })
        } else {
            None
        };
        let changed = self.session.as_ref().is_none_or(|old| {
            old.revision != s.revision || !self.same_peer(old.focus.as_ref(), s.focus.as_ref())
        }) || self.locked != locked
            || self.principals.iter().any(|(app, p)| {
                p.background != (p.background_intent || selected.as_ref() != Some(app))
            });
        if changed {
            self.lifecycle(None, None, None)?;
        }
        for (app, p) in &mut self.principals {
            p.background = p.background_intent || selected.as_ref() != Some(app);
        }
        self.locked = locked;
        self.session_source.get_or_insert_with(|| s.source.clone());
        self.source_observation.accept(s.sequence, s.sampled_at_ns);
        self.session = Some(s);
        Ok(())
    }
    pub(crate) fn activate(&mut self, durable: Durable) {
        self.retire_authentication_attempt();
        // Load the pending intent before ready can authorize any Source fact.
        self.root_lock_intent = durable.lock_pending;
        self.locked |= durable.lock_pending;
        self.durable = durable;
        self.ready = true;
    }
    pub(crate) fn plan_lock(&mut self, now: u64) -> Decision {
        let clearing = self
            .restoration
            .as_ref()
            .is_some_and(|r| r.phase == RestorationPhase::Clearing);
        // Revoke current authority before considering owned storage capacity.
        // Refusal/uncertainty still denies; no successful ACK precedes a sync.
        if self.lifecycle(Some(true), None, None).is_err() || !self.ready() {
            return Decision::Immediate(Reply::Unavailable);
        }
        self.lock_admission_epoch = match self.lock_admission_epoch.checked_add(1) {
            Some(epoch) => epoch,
            None => {
                self.invalidate();
                return Decision::Immediate(Reply::Unavailable);
            }
        };
        if clearing || self.required_lock.is_some() {
            // The first unresolved new lock owns its own bounded storage budget.
            // Further lock requests cannot extend it or acknowledge cached true.
            self.required_lock
                .get_or_insert((now.saturating_add(1_000_000_000), false));
            return Decision::Immediate(Reply::Unavailable);
        }
        if self.durable.lock_pending {
            return Decision::Immediate(Reply::Ok);
        }
        let mut next = self.durable.clone();
        next.lock_pending = true;
        Decision::Persist(WriteIntent {
            next,
            until: now.saturating_add(1_000_000_000),
            policy: self.policy,
            lifecycle: self.lifecycle,
            gate: Gate::Lock,
        })
    }
    pub(crate) fn required_lock_write_pending(&self) -> bool {
        self.required_lock.is_some()
    }
    pub(crate) fn take_required_lock_write(&mut self, now: u64) -> Option<WriteIntent> {
        if !self.ready() || self.restoration.is_some() {
            return None;
        }
        let (until, dispatched) = self.required_lock?;
        if dispatched {
            return None;
        }
        if now >= until || crate::boottime_ns() >= until {
            self.invalidate();
            return None;
        }
        self.required_lock = Some((until, true));
        let mut next = self.durable.clone();
        next.lock_pending = true;
        Some(WriteIntent {
            next,
            until,
            policy: self.policy,
            lifecycle: self.lifecycle,
            gate: Gate::RequiredLock,
        })
    }
    pub(crate) fn invalidate(&mut self) {
        self.retire_authentication_attempt();
        self.degraded = true;
        self.permits.clear();
    }
    pub(crate) fn ready(&self) -> bool {
        self.ready && !self.degraded
    }
    /// Startup uses the same already accepted Source as admission. Unlock/focus
    /// are independent; policy loading, terminal failure and expiry refuse it.
    pub(crate) fn startup_source_until(&self, now: u64) -> Option<u64> {
        if !self.ready() || !self.session_required {
            return None;
        }
        self.fresh_session(now).map(|sample| sample.until)
    }
    pub(crate) fn session_terminal(&self) -> bool {
        self.session_failed
    }
    pub fn register(&mut self, app: &str, pid: u32, alive: OwnedFd) -> io::Result<()> {
        if !self.ready() || !living(&alive)? {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let profile = self
            .profiles
            .iter()
            .find(|p| p.app == app)
            .ok_or(io::ErrorKind::PermissionDenied)?;
        self.generation = match self.generation.checked_add(1) {
            Some(v) => v,
            None => {
                self.invalidate();
                return Err(io::ErrorKind::Other.into());
            }
        };
        self.permits.retain(|_, p| p.app != app);
        self.principals.insert(
            app.into(),
            Principal {
                uid: profile.uid,
                pid,
                generation: self.generation,
                background: app == crate::messages::APP,
                background_intent: false,
                alive,
            },
        );
        Ok(())
    }
    pub fn revoke(&mut self, app: &str) {
        self.principals.remove(app);
        self.permits.retain(|_, p| p.app != app);
    }
    pub fn reap_dead(&mut self) -> io::Result<()> {
        let dead: Vec<_> = self
            .principals
            .iter()
            .filter_map(|(app, p)| match living(&p.alive) {
                Ok(true) => None,
                _ => Some(app.clone()),
            })
            .collect();
        for app in dead {
            self.revoke(&app);
        }
        Ok(())
    }
    pub(crate) fn plan_grant(
        &mut self,
        app: &str,
        permission: Permission,
        allowed: bool,
        now: u64,
    ) -> io::Result<WriteIntent> {
        if !self.ready() || !self.profiles.iter().any(|p| p.app == app) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        // Revocation fences memory before durable IO; failure stays unavailable.
        self.policy = match self.policy.checked_add(1) {
            Some(v) => v,
            None => {
                self.invalidate();
                return Err(io::ErrorKind::Other.into());
            }
        };
        self.permits.clear();
        if permission == Permission::MediaRoute && !allowed {
            let old = self.media_revocations.get(app).copied().unwrap_or(0);
            let Some(next) = old.checked_add(1) else {
                self.invalidate();
                return Err(io::ErrorKind::Other.into());
            };
            // Fence existing media before any durable write can block or fail.
            self.media_revocations.insert(app.into(), next);
        }
        let mut next = self.durable.clone();
        if let Some(g) = next
            .grants
            .iter_mut()
            .find(|g| g.app == app && g.permission == permission)
        {
            g.allowed = allowed;
        } else {
            next.grants.push(Grant {
                app: app.into(),
                permission,
                allowed,
            });
        }
        Ok(WriteIntent {
            next,
            until: now.checked_add(1_000_000_000).ok_or(io::ErrorKind::Other)?,
            policy: self.policy,
            lifecycle: self.lifecycle,
            gate: Gate::Grant,
        })
    }
    pub fn lifecycle(
        &mut self,
        locked: Option<bool>,
        suspended: Option<bool>,
        background: Option<(&str, bool)>,
    ) -> io::Result<()> {
        if locked.is_some() || suspended.is_some() {
            self.retire_authentication_attempt();
        }
        self.lifecycle = match self.lifecycle.checked_add(1) {
            Some(v) => v,
            None => {
                self.invalidate();
                return Err(io::ErrorKind::Other.into());
            }
        };
        self.permits.clear();
        if let Some(v) = locked {
            if self.session_required {
                self.root_lock_intent = v || self.durable.lock_pending;
                self.locked = self.root_lock_intent
                    || self
                        .fresh_session(crate::boottime_ns())
                        .is_none_or(|s| s.locked);
            } else {
                self.locked = v || self.durable.lock_pending;
            }
        }
        if let Some(v) = suspended {
            self.suspended = v;
            if self.session_required {
                self.reconcile_after = crate::boottime_ns();
                self.session = None;
                self.locked = true;
                for p in self.principals.values_mut() {
                    p.background = true;
                }
            }
        }
        if let Some((app, v)) = background {
            let p = self
                .principals
                .get_mut(app)
                .ok_or(io::ErrorKind::NotFound)?;
            p.background = v;
            if self.session_required {
                p.background_intent = v;
            }
        }
        Ok(())
    }
    fn allowed(&self, app: &str, permission: Permission) -> bool {
        !self.locked
            && !self.suspended
            && self.principals.get(app).is_some_and(|p| !p.background)
            && (!self.session_required
                || self.fresh_session(crate::boottime_ns()).is_some_and(|s| {
                    !self.root_lock_intent
                        && !s.locked
                        && s.focus.as_ref().is_some_and(|focus| {
                            self.principals
                                .get(app)
                                .is_some_and(|p| self.matches_focus(p, focus))
                        })
                }))
            && self
                .durable
                .grants
                .iter()
                .any(|g| g.app == app && g.permission == permission && g.allowed)
    }
    pub(crate) fn begin_read(
        &mut self,
        pid: u32,
        uid: u32,
        backend: &str,
        ttl_ms: u64,
        now: u64,
    ) -> Result<ReadStamp, Reply> {
        self.begin_stamp(pid, uid, backend, Permission::MessagesRead, ttl_ms, now)
    }
    fn begin_stamp(
        &mut self,
        pid: u32,
        uid: u32,
        backend: &str,
        permission: Permission,
        ttl_ms: u64,
        now: u64,
    ) -> Result<ReadStamp, Reply> {
        let _ = self.reap_dead();
        if !self.ready() {
            return Err(Reply::Unavailable);
        }
        let Some((app, p)) = self
            .principals
            .iter()
            .find(|(_, p)| p.pid == pid && p.uid == uid)
        else {
            return Err(Reply::Denied);
        };
        if !(1..=1000).contains(&ttl_ms) || !self.allowed(app, permission) {
            return Err(Reply::Denied);
        }
        Ok(ReadStamp {
            app: app.clone(),
            generation: p.generation,
            pid,
            uid,
            policy: self.policy,
            lifecycle: self.lifecycle,
            backend: backend.into(),
            permission,
            until: now.saturating_add(ttl_ms * 1_000_000),
            session: if self.session_required {
                let s = self.fresh_session(now).ok_or(Reply::Denied)?;
                Some(SessionRead {
                    source: s.source.clone(),
                    revision: s.revision,
                    until: s.until,
                })
            } else {
                None
            },
        })
    }
    pub(crate) fn current_read(&self, stamp: &ReadStamp, backend: &str, now: u64) -> bool {
        stamp.permission == Permission::MessagesRead && self.current_stamp(stamp, backend, now)
    }
    fn current_stamp(&self, stamp: &ReadStamp, backend: &str, now: u64) -> bool {
        self.ready()
            && now < stamp.until
            && self.policy == stamp.policy
            && self.lifecycle == stamp.lifecycle
            && stamp.backend == backend
            && (!self.session_required
                || stamp.session.as_ref().is_some_and(|stamp| {
                    now < stamp.until
                        && self.fresh_session(now).is_some_and(|s| {
                            s.source == stamp.source && s.revision == stamp.revision
                        })
                }))
            && self.allowed(&stamp.app, stamp.permission)
            && self.principals.get(&stamp.app).is_some_and(|p| {
                p.pid == stamp.pid
                    && p.uid == stamp.uid
                    && p.generation == stamp.generation
                    && living(&p.alive).unwrap_or(false)
            })
            && crate::boottime_ns() < stamp.until
            && (!self.session_required
                || stamp
                    .session
                    .as_ref()
                    .is_some_and(|s| crate::boottime_ns() < s.until))
    }
    pub(crate) fn begin_capture(
        &mut self,
        pid: u32,
        uid: u32,
        backend: &str,
        generation: pokelinux_camera::Generation,
        ttl_ms: u64,
        now: u64,
    ) -> Result<(CaptureStamp, WriteIntent), Reply> {
        if generation.session == 0 || generation.configuration == 0 {
            return Err(Reply::Unavailable);
        }
        let stamp = self.begin_stamp(pid, uid, backend, Permission::Capture, ttl_ms, now)?;
        if self.durable.receipts.len() >= MAX_RECEIPTS {
            return Err(Reply::Unavailable);
        }
        let Some(receipt) = self.id() else {
            self.invalidate();
            return Err(Reply::Unavailable);
        };
        let until = stamp
            .session
            .as_ref()
            .map_or(stamp.until, |s| stamp.until.min(s.until));
        let intent = crate::camera::Intent {
            capture: pokelinux_camera::CaptureId(self.sequence),
            generation,
            until_ns: until,
        };
        let mut next = self.durable.clone();
        next.receipts.push(receipt.clone());
        let write = WriteIntent {
            next,
            until,
            policy: stamp.policy,
            lifecycle: stamp.lifecycle,
            gate: Gate::Admit {
                app: stamp.app.clone(),
                generation: stamp.generation,
                permission: Permission::Capture,
                receipt,
            },
        };
        Ok((
            CaptureStamp {
                authority: stamp,
                intent,
            },
            write,
        ))
    }
    pub(crate) fn current_capture(
        &self,
        stamp: &CaptureStamp,
        backend: &str,
        generation: pokelinux_camera::Generation,
        now: u64,
    ) -> bool {
        stamp.intent.generation == generation
            && stamp.authority.permission == Permission::Capture
            && now < stamp.intent.until_ns
            && self.current_stamp(&stamp.authority, backend, now)
            && crate::boottime_ns() < stamp.intent.until_ns
    }
    pub(crate) fn brightness_allowed(&mut self, pid: u32, uid: u32, backend: &str) -> bool {
        self.begin_stamp(
            pid,
            uid,
            backend,
            Permission::DisplayBrightness,
            1000,
            crate::boottime_ns(),
        )
        .is_ok_and(|s| {
            s.app == crate::brightness::APP
                && s.uid == crate::brightness::UID
                && self.current_stamp(&s, backend, crate::boottime_ns())
        })
    }
    pub(crate) fn begin_brightness(
        &mut self,
        pid: u32,
        uid: u32,
        backend: &str,
        request: crate::brightness::Request,
        now: u64,
    ) -> Result<BrightnessStamp, Reply> {
        if request.percent > 100 || request.expected_revision == 0 || backend.is_empty() {
            return Err(Reply::Denied);
        }
        let authority = self.begin_stamp(
            pid,
            uid,
            backend,
            Permission::DisplayBrightness,
            request.ttl_ms,
            now,
        )?;
        if authority.app != crate::brightness::APP || authority.uid != crate::brightness::UID {
            return Err(Reply::Denied);
        }
        let Some(id) = self.sequence.checked_add(1) else {
            self.invalidate();
            return Err(Reply::Unavailable);
        };
        self.sequence = id;
        let until_ns = authority
            .session
            .as_ref()
            .map_or(authority.until, |s| authority.until.min(s.until));
        let stamp = BrightnessStamp {
            authority,
            intent: crate::brightness::Intent {
                id,
                percent: request.percent,
                expected_revision: request.expected_revision,
                until_ns,
            },
        };
        if !self.current_brightness(&stamp, backend, crate::boottime_ns()) {
            return Err(Reply::Denied);
        }
        Ok(stamp)
    }
    pub(crate) fn current_brightness(
        &self,
        stamp: &BrightnessStamp,
        backend: &str,
        now: u64,
    ) -> bool {
        stamp.authority.permission == Permission::DisplayBrightness
            && now < stamp.intent.until_ns
            && self.current_stamp(&stamp.authority, backend, now)
            && crate::boottime_ns() < stamp.intent.until_ns
    }
    pub(crate) fn media_allowed(&mut self, pid: u32, uid: u32, backend: &str) -> bool {
        match self.begin_stamp(
            pid,
            uid,
            backend,
            Permission::MediaRoute,
            1000,
            crate::boottime_ns(),
        ) {
            Ok(stamp) => self.current_stamp(&stamp, backend, crate::boottime_ns()),
            Err(_) => false,
        }
    }
    pub(crate) fn begin_media(
        &mut self,
        pid: u32,
        uid: u32,
        backend: &str,
        route: &str,
        ttl_ms: u64,
        now: u64,
    ) -> Result<MediaStamp, Reply> {
        if route.is_empty()
            || route.len() > 64
            || !route
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            || backend.is_empty()
        {
            return Err(Reply::Denied);
        }
        let authority = self.begin_stamp(pid, uid, backend, Permission::MediaRoute, ttl_ms, now)?;
        let Some(id) = self.sequence.checked_add(1) else {
            self.invalidate();
            return Err(Reply::Unavailable);
        };
        self.sequence = id;
        let until_ns = authority
            .session
            .as_ref()
            .map_or(authority.until, |s| authority.until.min(s.until));
        let revocation = self
            .media_revocations
            .get(&authority.app)
            .copied()
            .unwrap_or(0);
        let stamp = MediaStamp {
            authority,
            intent: MediaIntent {
                id,
                route: route.into(),
                until_ns,
            },
            revocation,
        };
        if !self.current_media(&stamp, backend, crate::boottime_ns()) {
            return Err(Reply::Denied);
        }
        Ok(stamp)
    }
    pub(crate) fn begin_media_preview(
        &mut self,
        pid: u32,
        uid: u32,
        backend: &str,
        request: &crate::audio::PreviewRequest,
        now: u64,
    ) -> Result<MediaStamp, Reply> {
        if request.until_boottime_ns <= now
            || now
                .checked_add(1_000_000_000)
                .is_none_or(|maximum| request.until_boottime_ns > maximum)
        {
            return Err(Reply::Denied);
        }
        let mut stamp = self.begin_media(pid, uid, backend, &request.route, request.ttl_ms, now)?;
        stamp.intent.until_ns = stamp.intent.until_ns.min(request.until_boottime_ns);
        stamp.authority.until = stamp.authority.until.min(request.until_boottime_ns);
        if !self.current_media(&stamp, backend, crate::boottime_ns()) {
            return Err(Reply::Denied);
        }
        Ok(stamp)
    }
    pub(crate) fn current_media(&self, stamp: &MediaStamp, backend: &str, now: u64) -> bool {
        stamp.authority.permission == Permission::MediaRoute
            && now < stamp.intent.until_ns
            && self
                .media_revocations
                .get(&stamp.authority.app)
                .copied()
                .unwrap_or(0)
                == stamp.revocation
            && self.current_stamp(&stamp.authority, backend, now)
            && crate::boottime_ns() < stamp.intent.until_ns
    }
    pub(crate) fn approve_media(
        &self,
        stamp: &MediaStamp,
        backend: &str,
        now: u64,
    ) -> Option<MediaLeaseStamp> {
        if !self.current_media(stamp, backend, now) {
            return None;
        }
        Some(MediaLeaseStamp {
            app: stamp.authority.app.clone(),
            generation: stamp.authority.generation,
            pid: stamp.authority.pid,
            uid: stamp.authority.uid,
            backend: stamp.authority.backend.clone(),
            revocation: stamp.revocation,
        })
    }
    pub(crate) fn current_media_lease(&self, lease: &MediaLeaseStamp, backend: &str) -> bool {
        self.ready()
            && lease.backend == backend
            && self.media_revocations.get(&lease.app).copied().unwrap_or(0) == lease.revocation
            && self
                .durable
                .grants
                .iter()
                .any(|g| g.app == lease.app && g.permission == Permission::MediaRoute && g.allowed)
            && self.principals.get(&lease.app).is_some_and(|p| {
                p.pid == lease.pid
                    && p.uid == lease.uid
                    && p.generation == lease.generation
                    && living(&p.alive).unwrap_or(false)
            })
    }
    fn id(&mut self) -> Option<String> {
        self.sequence = self.sequence.checked_add(1)?;
        Some(format!("{}-{:016x}", self.incarnation, self.sequence))
    }
    pub(crate) fn refuse_busy(&mut self, pid: u32, uid: u32, r: AppRequest) -> Reply {
        let _ = self.reap_dead();
        let Some(app) = self
            .principals
            .iter()
            .find(|(_, p)| p.pid == pid && p.uid == uid)
            .map(|(a, _)| a.clone())
        else {
            return Reply::Denied;
        };
        if let AppRequest::Admit { permit } = r
            && self.permits.get(&permit).is_some_and(|p| p.app == app)
        {
            self.permits.remove(&permit);
        }
        Reply::Unavailable
    }
    pub(crate) fn plan_request(
        &mut self,
        pid: u32,
        uid: u32,
        request: AppRequest,
        now: u64,
    ) -> Decision {
        let _ = self.reap_dead();
        self.permits.retain(|_, p| now < p.until);
        let immediate = |r| Decision::Immediate(r);
        if !self.ready() {
            return immediate(Reply::Unavailable);
        }
        let Some(app) = self
            .principals
            .iter()
            .find(|(_, p)| p.uid == uid && p.pid == pid)
            .map(|(a, _)| a.clone())
        else {
            return immediate(Reply::Denied);
        };
        match request {
            AppRequest::BrightnessSet { .. }
            | AppRequest::BrightnessStatus {}
            | AppRequest::MessagesRead { .. }
            | AppRequest::CameraCapture { .. }
            | AppRequest::MediaRoute { .. }
            | AppRequest::MediaPreviewPlay { .. }
            | AppRequest::MediaPreviewStop {}
            | AppRequest::MediaStatus { .. }
            | AppRequest::MediaReconnect {} => immediate(Reply::Unavailable),
            AppRequest::Query {} => immediate(Reply::Ok),
            AppRequest::Prepare { permission, ttl_ms } => {
                if matches!(
                    permission,
                    Permission::MessagesRead
                        | Permission::MediaRoute
                        | Permission::DisplayBrightness
                ) || !(1..=5000).contains(&ttl_ms)
                    || !self.allowed(&app, permission)
                    || self.permits.len() >= MAX_PERMITS
                    || self.permits.values().filter(|p| p.app == app).count() >= 4
                {
                    return immediate(Reply::Denied);
                }
                let Some(until) = now.checked_add(ttl_ms * 1_000_000) else {
                    return immediate(Reply::Unavailable);
                };
                let Some(id) = self.id() else {
                    self.invalidate();
                    return immediate(Reply::Unavailable);
                };
                self.permits.insert(
                    id.clone(),
                    Permit {
                        app: app.clone(),
                        generation: self.principals[&app].generation,
                        policy: self.policy,
                        lifecycle: self.lifecycle,
                        permission,
                        until,
                    },
                );
                immediate(Reply::Prepared { permit: id })
            }
            AppRequest::Admit { permit } => {
                if self.permits.get(&permit).is_none_or(|p| p.app != app) {
                    return immediate(Reply::Denied);
                }
                let p = self.permits.remove(&permit).unwrap();
                if now >= p.until
                    || p.generation != self.principals[&app].generation
                    || p.policy != self.policy
                    || p.lifecycle != self.lifecycle
                    || !self.allowed(&app, p.permission)
                {
                    return immediate(Reply::Denied);
                }
                if self.durable.receipts.len() >= MAX_RECEIPTS {
                    return immediate(Reply::Unavailable);
                }
                let Some(id) = self.id() else {
                    self.invalidate();
                    return immediate(Reply::Unavailable);
                };
                let mut next = self.durable.clone();
                next.receipts.push(id.clone());
                Decision::Persist(WriteIntent {
                    next,
                    until: p.until.min(now.saturating_add(1_000_000_000)),
                    policy: self.policy,
                    lifecycle: self.lifecycle,
                    gate: Gate::Admit {
                        app,
                        generation: p.generation,
                        permission: p.permission,
                        receipt: id,
                    },
                })
            }
        }
    }
    pub(crate) fn finish(&mut self, intent: WriteIntent, success: bool, now: u64) -> Reply {
        if !success {
            self.invalidate();
            return Reply::Unavailable;
        }
        if let Gate::Restore {
            attempt,
            lock_epoch,
        } = intent.gate
        {
            self.refresh_authentication_lifetime(now);
            let current = now < intent.until
                && self.policy == intent.policy
                && self.restoration.as_ref().is_some_and(|restoration| {
                    restoration.phase == RestorationPhase::Clearing
                        && restoration.context.id == attempt
                        && restoration.context.lock_epoch == lock_epoch
                        && self.session.as_ref().is_some_and(|sample| {
                            self.restoration_context_current(restoration, sample, now)
                                && Self::restoration_event_matches(restoration, sample)
                        })
                })
                && crate::boottime_ns() < intent.until;
            // Never pretend a synced false replacement was still true merely
            // because current publication was refused after the write.
            self.durable = intent.next;
            self.restoration = None;
            if !current {
                if self.required_lock.is_none()
                    || !self.ready()
                    || now >= intent.until
                    || crate::boottime_ns() >= intent.until
                {
                    self.invalidate();
                }
                return Reply::Unavailable;
            }
            self.root_lock_intent = false;
            if self
                .observe_session(self.session.as_ref().unwrap().clone(), now)
                .is_err()
                || self.session_terminal()
                || self.locked
            {
                self.invalidate();
                return Reply::Unavailable;
            }
            return Reply::Ok;
        }
        // A known durable commit is retained as history, even when current publication fails.
        self.durable = intent.next;
        if matches!(intent.gate, Gate::Lock | Gate::RequiredLock) {
            if matches!(intent.gate, Gate::RequiredLock) {
                self.required_lock = None;
            }
            // Source focus/background revisions may advance during the sync.
            // They cannot weaken this monotonic lock; no effect stamp is issued.
            self.root_lock_intent = true;
            self.locked = true;
            if self.ready() && now < intent.until && crate::boottime_ns() < intent.until {
                return Reply::Ok;
            }
            self.invalidate();
            return Reply::Unavailable;
        }
        if !self.ready()
            || now >= intent.until
            || self.policy != intent.policy
            || self.lifecycle != intent.lifecycle
        {
            self.invalidate();
            return Reply::Unavailable;
        }
        match intent.gate {
            Gate::Lock | Gate::RequiredLock | Gate::Restore { .. } => {
                unreachable!("lock completion handled above")
            }
            Gate::Grant => Reply::Ok,
            Gate::Admit {
                app,
                generation,
                permission,
                receipt,
            } => {
                if !self.principals.get(&app).is_some_and(|p| {
                    p.generation == generation && living(&p.alive).unwrap_or(false)
                }) || !self.allowed(&app, permission)
                {
                    self.invalidate();
                    return Reply::Unavailable;
                }
                Reply::AdmittedUncertain { receipt }
            }
        }
    }
    pub fn is_degraded(&self) -> bool {
        self.degraded
    }
    pub(crate) fn snapshot(&mut self, occupied: bool, busy: bool) -> Reply {
        let _ = self.reap_dead();
        Reply::Snapshot {
            source: self.source_observation.clone(),
            principals: self.principals.len(),
            permits: self.permits.len(),
            receipts: self.durable.receipts.len(),
            locked: self.locked,
            suspended: self.suspended,
            degraded: self.degraded,
            ready: self.ready(),
            storage_occupied: occupied,
            storage_busy: busy,
        }
    }
}
pub fn living(fd: &OwnedFd) -> io::Result<bool> {
    let mut fds = [PollFd::new(fd, PollFlags::IN)];
    poll(
        &mut fds,
        Some(&rustix::event::Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )?;
    Ok(fds[0].revents().is_empty())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Disk;
    use std::path::PathBuf;
    #[test]
    fn resume_does_not_erase_established_source_loss_history() {
        // Internal policy boundary: actual BOOTTIME, typed owned-provider facts.
        // This does not claim an IPC source emitted None or a physical resume.
        let mut a = Authority::loading(vec![]).unwrap();
        a.activate(Durable::default());
        a.require_session().unwrap();
        let source = "11111111111111111111111111111111".to_string();
        let sample = crate::boottime_ns();
        a.observe_session(
            crate::session::Snapshot {
                source: source.clone(),
                revision: 1,
                sequence: 1,
                sampled_at_ns: sample,
                until: sample + crate::session::SAMPLE_BUDGET_NS,
                locked: false,
                focus: None,
                locker: None,
                lock_generation: None,
                protocol_unlock: None,
            },
            crate::boottime_ns(),
        )
        .unwrap();
        assert!(!a.locked);
        a.lifecycle(None, Some(false), None).unwrap();
        a.missing_session().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let sample = crate::boottime_ns();
        a.observe_session(
            crate::session::Snapshot {
                source,
                revision: 1,
                sequence: 2,
                sampled_at_ns: sample,
                until: sample + crate::session::SAMPLE_BUDGET_NS,
                locked: false,
                focus: None,
                locker: None,
                lock_generation: None,
                protocol_unlock: None,
            },
            crate::boottime_ns(),
        )
        .unwrap();
        assert!(
            a.locked,
            "fresh facts reactivated lost source after resume cleared current sample"
        );
    }
    fn locker_snapshot(locker: Option<crate::session::OwnedFocus>) -> crate::session::Snapshot {
        let sampled_at_ns = crate::boottime_ns();
        crate::session::Snapshot {
            source: "22222222222222222222222222222222".into(),
            revision: 1,
            sequence: 1,
            sampled_at_ns,
            until: sampled_at_ns + crate::session::SAMPLE_BUDGET_NS,
            locked: true,
            focus: None,
            locker,
            lock_generation: None,
            protocol_unlock: None,
        }
    }
    fn own_locker() -> crate::session::OwnedFocus {
        crate::session::OwnedFocus::new(
            crate::session::NativeFocus {
                pid: std::process::id(),
                uid: rustix::process::getuid().as_raw(),
                gid: rustix::process::getgid().as_raw(),
            },
            rustix::process::pidfd_open(
                rustix::process::getpid(),
                rustix::process::PidfdFlags::empty(),
            )
            .unwrap(),
        )
    }
    fn session_authority() -> Authority {
        let mut authority = Authority::loading(vec![]).unwrap();
        authority.activate(Durable::default());
        authority.require_session().unwrap();
        authority
    }
    #[test]
    fn locker_changes_without_a_new_revision_are_terminal() {
        // Provider/actor boundary with actual owned kernel handles, not Source IPC.
        for sequence in [1, 2] {
            let mut authority = session_authority();
            let initial = locker_snapshot(None);
            authority
                .observe_session(initial.clone(), crate::boottime_ns())
                .unwrap();
            let mut changed = initial.clone();
            changed.sequence = sequence;
            changed.locker = Some(own_locker());
            authority
                .observe_session(changed, crate::boottime_ns())
                .unwrap();
            assert!(
                authority.session_failed && authority.session.is_none(),
                "changed locker was accepted with unchanged revision at sequence {sequence}"
            );
            authority
                .observe_session(initial, crate::boottime_ns())
                .unwrap();
            assert!(authority.session_failed && authority.session.is_none());
        }
    }
    #[test]
    fn locker_equal_metadata_cannot_hide_a_different_kernel_task() {
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let child_handle = rustix::process::pidfd_open(
            rustix::process::Pid::from_raw(child.id() as i32).unwrap(),
            rustix::process::PidfdFlags::empty(),
        )
        .unwrap();
        let mut authority = session_authority();
        let initial = locker_snapshot(Some(own_locker()));
        let metadata = initial.locker.as_ref().unwrap().metadata.clone();
        authority
            .observe_session(initial.clone(), crate::boottime_ns())
            .unwrap();
        let mut changed = initial;
        changed.sequence += 1;
        changed.locker = Some(crate::session::OwnedFocus::new(metadata, child_handle));
        authority
            .observe_session(changed, crate::boottime_ns())
            .unwrap();
        let refused = authority.session_failed && authority.session.is_none();
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        assert!(
            refused,
            "same metadata hid a different retained task without a new revision"
        );
    }
    #[test]
    fn locker_unlocked_provider_snapshot_is_refused() {
        let mut authority = session_authority();
        let mut snapshot = locker_snapshot(Some(own_locker()));
        snapshot.locked = false;
        authority
            .observe_session(snapshot, crate::boottime_ns())
            .unwrap();
        assert!(
            authority.session_failed && authority.session.is_none() && authority.locked,
            "unlocked provider supplied mapped locker facts"
        );
    }
    #[test]
    fn locker_cached_sample_and_same_task_heartbeat_do_not_fail_source() {
        let mut authority = session_authority();
        let initial = locker_snapshot(Some(own_locker()));
        authority
            .observe_session(initial.clone(), crate::boottime_ns())
            .unwrap();
        authority
            .observe_session(initial.clone(), crate::boottime_ns())
            .unwrap();
        let lifecycle = authority.lifecycle;
        let mut heartbeat = initial;
        heartbeat.sequence += 1;
        heartbeat.locker = Some(own_locker());
        authority
            .observe_session(heartbeat, crate::boottime_ns())
            .unwrap();
        assert!(!authority.session_failed && authority.session.is_some());
        assert_eq!(
            authority.lifecycle, lifecycle,
            "same living task heartbeat fenced policy"
        );
    }
    fn protocol_event(sample: &crate::session::Snapshot) -> crate::session::ProtocolUnlock {
        crate::session::ProtocolUnlock {
            generation: 7,
            sequence: sample.sequence,
            observed_at_ns: sample.sampled_at_ns,
        }
    }
    #[test]
    fn protocol_invalid_shapes_fail_the_actor_terminally() {
        let mut refused = Vec::new();
        for case in 0..8 {
            let mut authority = session_authority();
            let mut snapshot = locker_snapshot(None);
            snapshot.locked = false;
            let mut event = protocol_event(&snapshot);
            match case {
                0 => {
                    snapshot.locked = true;
                    snapshot.lock_generation = Some(0);
                }
                1 => snapshot.lock_generation = Some(7),
                2 => {
                    snapshot.locked = true;
                    snapshot.protocol_unlock = Some(event);
                }
                3 => {
                    event.generation = 0;
                    snapshot.protocol_unlock = Some(event);
                }
                4 => {
                    event.sequence = 0;
                    snapshot.protocol_unlock = Some(event);
                }
                5 => {
                    event.sequence += 1;
                    snapshot.protocol_unlock = Some(event);
                }
                6 => {
                    event.observed_at_ns = 0;
                    snapshot.protocol_unlock = Some(event);
                }
                7 => {
                    event.observed_at_ns += 1;
                    snapshot.protocol_unlock = Some(event);
                }
                _ => unreachable!(),
            }
            authority
                .observe_session(snapshot, crate::boottime_ns())
                .unwrap();
            refused
                .push(authority.session_failed && authority.session.is_none() && authority.locked);
            if *refused.last().unwrap() {
                authority
                    .observe_session(locker_snapshot(None), crate::boottime_ns())
                    .unwrap();
                assert!(authority.session_failed && authority.session.is_none());
            }
        }
        assert!(
            refused.iter().all(|refused| *refused),
            "invalid shape admission results: {refused:?}"
        );
    }
    #[test]
    fn protocol_generation_changes_require_a_new_actor_revision() {
        let mut refused = Vec::new();
        for sequence in [1, 2] {
            let mut authority = session_authority();
            let mut initial = locker_snapshot(Some(own_locker()));
            initial.lock_generation = Some(7);
            authority
                .observe_session(initial.clone(), crate::boottime_ns())
                .unwrap();
            assert!(!authority.session_failed);
            let mut changed = initial.clone();
            changed.sequence = sequence;
            changed.lock_generation = Some(8);
            authority
                .observe_session(changed, crate::boottime_ns())
                .unwrap();
            refused.push(authority.session_failed && authority.session.is_none());
            if *refused.last().unwrap() {
                authority
                    .observe_session(initial, crate::boottime_ns())
                    .unwrap();
                assert!(authority.session_failed && authority.session.is_none());
            }
        }
        assert!(
            refused.iter().all(|refused| *refused),
            "generation mutation admission results: {refused:?}"
        );
    }
    #[test]
    fn protocol_unlock_changes_require_a_new_actor_revision() {
        let mut refused = Vec::new();
        for sequence in [1, 2] {
            for case in 0..4 {
                let mut authority = session_authority();
                let mut initial = locker_snapshot(None);
                initial.locked = false;
                initial.sequence = 2;
                let mut event = protocol_event(&initial);
                event.sequence = 1;
                event.observed_at_ns -= 1;
                if case != 0 {
                    initial.protocol_unlock = Some(event.clone());
                }
                authority
                    .observe_session(initial.clone(), crate::boottime_ns())
                    .unwrap();
                assert!(!authority.session_failed);
                let mut changed = initial.clone();
                changed.sequence += sequence - 1;
                match case {
                    0 => changed.protocol_unlock = Some(event),
                    1 => changed.protocol_unlock = None,
                    2 => changed.protocol_unlock.as_mut().unwrap().generation += 1,
                    3 => changed.protocol_unlock.as_mut().unwrap().observed_at_ns += 1,
                    _ => unreachable!(),
                }
                authority
                    .observe_session(changed, crate::boottime_ns())
                    .unwrap();
                refused.push(authority.session_failed && authority.session.is_none());
                if *refused.last().unwrap() {
                    authority
                        .observe_session(initial, crate::boottime_ns())
                        .unwrap();
                    assert!(authority.session_failed && authority.session.is_none());
                }
            }
        }
        assert!(
            refused.iter().all(|refused| *refused),
            "unlock mutation admission results: {refused:?}"
        );
    }
    #[test]
    fn protocol_cached_samples_and_sticky_heartbeats_preserve_actor_revision() {
        let mut authority = session_authority();
        let mut sample = locker_snapshot(Some(own_locker()));
        sample.lock_generation = Some(7);
        authority
            .observe_session(sample.clone(), crate::boottime_ns())
            .unwrap();
        let lifecycle = authority.lifecycle;
        authority
            .observe_session(sample.clone(), crate::boottime_ns())
            .unwrap();
        sample.sequence += 1;
        authority
            .observe_session(sample.clone(), crate::boottime_ns())
            .unwrap();
        assert!(!authority.session_failed);
        assert_eq!(authority.lifecycle, lifecycle);
        sample.sequence += 1;
        sample.revision += 1;
        sample.locked = false;
        sample.locker = None;
        sample.lock_generation = None;
        sample.protocol_unlock = Some(protocol_event(&sample));
        authority
            .observe_session(sample.clone(), crate::boottime_ns())
            .unwrap();
        assert!(!authority.session_failed);
        let lifecycle = authority.lifecycle;
        authority
            .observe_session(sample.clone(), crate::boottime_ns())
            .unwrap();
        sample.sequence += 1;
        authority
            .observe_session(sample, crate::boottime_ns())
            .unwrap();
        assert!(!authority.session_failed);
        assert_eq!(authority.lifecycle, lifecycle);
    }
    #[test]
    fn auth_attempt_lock_epoch_overflow_retires_and_keeps_slot() {
        let mut authority = session_authority();
        authority.activate(Durable {
            lock_pending: true,
            ..Durable::default()
        });
        let mut sample = locker_snapshot(Some(own_locker()));
        sample.lock_generation = Some(1);
        authority
            .observe_session(sample.clone(), crate::boottime_ns())
            .unwrap();
        authority.lock_admission_epoch = u64::MAX;
        let now = crate::boottime_ns();
        let until = now + 20_000_000_000;
        let attempt = authority.begin_authentication_attempt(now, until).unwrap();
        assert!(authority.current_authentication_attempt(&attempt, crate::boottime_ns()));
        assert!(matches!(
            authority.plan_lock(crate::boottime_ns()),
            Decision::Immediate(Reply::Unavailable)
        ));
        assert!(crate::boottime_ns() < sample.until);
        assert!(crate::boottime_ns() < until);
        assert!(!authority.current_authentication_attempt(&attempt, crate::boottime_ns()));
        assert!(authority.is_degraded());
        assert_eq!(authority.lock_admission_epoch, u64::MAX);
        assert!(authority.authentication.as_ref().unwrap().retired);
        assert!(
            authority
                .begin_authentication_attempt(crate::boottime_ns(), until)
                .is_err()
        );
        assert!(authority.durable.lock_pending);
    }
    #[test]
    fn auth_attempt_id_overflow_fails_closed_before_any_slot() {
        let mut authority = session_authority();
        authority.activate(Durable {
            lock_pending: true,
            ..Durable::default()
        });
        let mut sample = locker_snapshot(Some(own_locker()));
        sample.lock_generation = Some(1);
        authority
            .observe_session(sample.clone(), crate::boottime_ns())
            .unwrap();
        authority.authentication_sequence = u64::MAX;
        let now = crate::boottime_ns();
        assert!(crate::boottime_ns() < sample.until);
        assert!(
            authority
                .begin_authentication_attempt(now, now + 20_000_000_000)
                .is_err()
        );
        assert!(authority.is_degraded());
        assert!(authority.authentication.is_none());
        assert_eq!(authority.authentication_sequence, u64::MAX);
        assert!(authority.durable.lock_pending);
    }
    struct Harness {
        core: Authority,
        disk: Disk,
    }
    impl std::ops::Deref for Harness {
        type Target = Authority;
        fn deref(&self) -> &Authority {
            &self.core
        }
    }
    impl std::ops::DerefMut for Harness {
        fn deref_mut(&mut self) -> &mut Authority {
            &mut self.core
        }
    }
    impl Harness {
        fn grant(&mut self, app: &str, permission: Permission, allowed: bool) -> io::Result<()> {
            let intent = self.core.plan_grant(app, permission, allowed, 0)?;
            let success = self.disk.replace(&intent.next).is_ok();
            if self.core.finish(intent, success, 0) == Reply::Ok {
                Ok(())
            } else {
                Err(io::ErrorKind::Other.into())
            }
        }
    }
    use crate::{AppRequest, boottime_ns};
    use std::os::unix::fs::PermissionsExt;

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            // These real pidfd/durable-file tests require the isolated root runner.
            assert_eq!(rustix::process::getuid().as_raw(), 0);
            let root = std::env::var_os("NATIVE_LIFECYCLE_TEST_ROOT")
                .expect("use the isolated component runner");
            let path = PathBuf::from(root).join(format!("lifecycle-{}", fresh_id().unwrap()));
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
        fn authority(&self) -> Harness {
            let profiles = vec![AppProfile {
                app: "one".into(),
                uid: 10000,
                executable: "/unused".into(),
                args: vec![],
            }];
            let (disk, durable) = Disk::open(&self.0, &["one".into()]).unwrap();
            let mut core = Authority::loading(profiles).unwrap();
            core.activate(durable);
            let mut a = Harness { core, disk };
            let pid = rustix::process::getpid();
            a.register(
                "one",
                pid.as_raw_nonzero().get() as u32,
                rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::NONBLOCK).unwrap(),
            )
            .unwrap();
            a.grant("one", Permission::Capture, true).unwrap();
            a.lifecycle(Some(false), None, None).unwrap();
            a
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn request(a: &mut Harness, r: AppRequest, mut clock: impl FnMut() -> u64) -> Reply {
        // Direct component caller injection; actual credentials are qualified separately.
        match a.core.plan_request(std::process::id(), 10000, r, clock()) {
            Decision::Immediate(r) => r,
            Decision::Persist(intent) => {
                let success = a.disk.replace(&intent.next).is_ok();
                a.core.finish(intent, success, clock())
            }
        }
    }
    fn prepared(a: &mut Harness, now: u64) -> String {
        match request(
            a,
            AppRequest::Prepare {
                permission: Permission::Capture,
                ttl_ms: 1,
            },
            || now,
        ) {
            Reply::Prepared { permit } => permit,
            r => panic!("{r:?}"),
        }
    }
    #[test]
    fn late_durable_success_grants_no_current_admission() {
        let dir = Temp::new();
        let mut a = dir.authority();
        let p = prepared(&mut a, 10);
        let mut samples = [10, 1_000_010].into_iter();
        assert_eq!(
            request(&mut a, AppRequest::Admit { permit: p.clone() }, || samples
                .next()
                .expect("exact samples")),
            Reply::Unavailable
        );
        assert_eq!(a.durable.receipts.len(), 1);
        assert_eq!(
            request(&mut a, AppRequest::Admit { permit: p }, || 10),
            Reply::Unavailable
        );
        assert!(dir.0.join("policy.json").is_file());
    }
    #[test]
    fn suspended_time_jump_expires_without_timer_delivery() {
        let dir = Temp::new();
        let mut a = dir.authority();
        let p = prepared(&mut a, 20);
        assert_eq!(
            request(&mut a, AppRequest::Admit { permit: p }, || 1_000_020),
            Reply::Denied
        );
        assert_eq!(a.durable.receipts.len(), 0);
        assert!(boottime_ns() > 0);
    }
    #[test]
    fn failed_admission_write_never_recovers_or_retries() {
        let dir = Temp::new();
        let mut a = dir.authority();
        let p = prepared(&mut a, 10);
        std::fs::write(dir.0.join("policy.pending"), b"private failure fixture").unwrap();
        assert_eq!(
            request(&mut a, AppRequest::Admit { permit: p }, || 10),
            Reply::Unavailable
        );
        assert!(a.degraded);
        assert!(a.permits.is_empty());
        assert!(a.durable.receipts.is_empty());
        std::fs::remove_file(dir.0.join("policy.pending")).unwrap();
        assert_eq!(
            request(&mut a, AppRequest::Query {}, || 10),
            Reply::Unavailable
        );
    }
    #[test]
    fn exhausted_lifecycle_revision_fences_prepared_work() {
        let dir = Temp::new();
        let mut a = dir.authority();
        let p = prepared(&mut a, 10);
        a.lifecycle = u64::MAX;
        assert!(a.lifecycle(Some(true), None, None).is_err());
        assert!(a.degraded, "revision exhaustion retained usable authority");
        assert_eq!(
            request(&mut a, AppRequest::Admit { permit: p }, || 10),
            Reply::Unavailable
        );
    }
}

#[cfg(test)]
#[path = "media_authority_tests.rs"]
mod media_tests;
