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
    until: u64,
    session: Option<SessionRead>,
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
    root_lock_intent: bool,
    reconcile_after: u64,
    session_view: Option<pokelinux_kernel_identity::ProcessView>,
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
            root_lock_intent: false,
            reconcile_after: 0,
            session_view: None,
        })
    }
    pub(crate) fn require_session(&mut self) -> io::Result<()> {
        self.session_view = Some(pokelinux_kernel_identity::ProcessView::open()?);
        self.session_required = true;
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
    fn same_focus(
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
    pub(crate) fn fail_session(&mut self) -> io::Result<()> {
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
                            || !self.same_focus(s.focus.as_ref(), old.focus.as_ref())))
                    || (s.revision == old.revision
                        && (s.locked != old.locked
                            || s.focus.as_ref().map(|f| &f.metadata)
                                != old.focus.as_ref().map(|f| &f.metadata)))
            })
        {
            return self.fail_session();
        }
        // Explicit resume/session reconciliation needs a newly sampled packet.
        // Cached but otherwise fresh pre-resume facts cannot restore authority.
        if s.sampled_at_ns <= self.reconcile_after {
            self.session = None;
            self.locked = true;
            for p in self.principals.values_mut() {
                p.background = true;
            }
            return Ok(());
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
            old.revision != s.revision || !self.same_focus(old.focus.as_ref(), s.focus.as_ref())
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
        self.session = Some(s);
        Ok(())
    }
    pub(crate) fn activate(&mut self, durable: Durable) {
        self.durable = durable;
        self.ready = true;
    }
    pub(crate) fn invalidate(&mut self) {
        self.degraded = true;
        self.permits.clear();
    }
    pub(crate) fn ready(&self) -> bool {
        self.ready && !self.degraded
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
                self.degraded = true;
                self.permits.clear();
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
                self.degraded = true;
                self.permits.clear();
                return Err(io::ErrorKind::Other.into());
            }
        };
        self.permits.clear();
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
        self.lifecycle = match self.lifecycle.checked_add(1) {
            Some(v) => v,
            None => {
                self.degraded = true;
                self.permits.clear();
                return Err(io::ErrorKind::Other.into());
            }
        };
        self.permits.clear();
        if let Some(v) = locked {
            if self.session_required {
                self.root_lock_intent = v;
                self.locked = v
                    || self
                        .fresh_session(crate::boottime_ns())
                        .is_none_or(|s| s.locked);
            } else {
                self.locked = v;
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
        if !(1..=1000).contains(&ttl_ms) || !self.allowed(app, Permission::MessagesRead) {
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
            && self.allowed(&stamp.app, Permission::MessagesRead)
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
            AppRequest::MessagesRead { .. } => immediate(Reply::Unavailable),
            AppRequest::Query {} => immediate(Reply::Ok),
            AppRequest::Prepare { permission, ttl_ms } => {
                if permission == Permission::MessagesRead
                    || !(1..=5000).contains(&ttl_ms)
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
        // A known durable commit is retained as history, even when current publication fails.
        self.durable = intent.next;
        if !self.ready()
            || now >= intent.until
            || self.policy != intent.policy
            || self.lifecycle != intent.lifecycle
        {
            self.invalidate();
            return Reply::Unavailable;
        }
        match intent.gate {
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
            },
            crate::boottime_ns(),
        )
        .unwrap();
        assert!(
            a.locked,
            "fresh facts reactivated lost source after resume cleared current sample"
        );
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
