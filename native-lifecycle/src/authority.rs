use crate::{AppProfile, AppRequest, MAX_PERMITS, MAX_RECEIPTS, Permission, Reply, fresh_id};
use rustix::{
    event::{PollFd, PollFlags, poll},
    fd::OwnedFd,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    app: String,
    permission: Permission,
    allowed: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Durable {
    grants: Vec<Grant>,
    receipts: Vec<String>,
}
struct Principal {
    uid: u32,
    pid: u32,
    generation: u64,
    background: bool,
    alive: OwnedFd,
}
struct Permit {
    app: String,
    generation: u64,
    policy: u64,
    lifecycle: u64,
    permission: Permission,
    until: u64,
}

pub struct Authority {
    profiles: Vec<AppProfile>,
    principals: BTreeMap<String, Principal>,
    permits: BTreeMap<String, Permit>,
    durable: Durable,
    directory: PathBuf,
    _lock: File,
    incarnation: String,
    sequence: u64,
    generation: u64,
    policy: u64,
    lifecycle: u64,
    locked: bool,
    suspended: bool,
    degraded: bool,
}
fn private_file(path: &Path) -> io::Result<File> {
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.uid() != 0 || meta.mode() & 0o077 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
}
impl Authority {
    pub fn open(profiles: Vec<AppProfile>, directory: &Path) -> io::Result<Self> {
        let m = std::fs::symlink_metadata(directory)?;
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o077 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(directory.join("owner.lock"))?;
        let m = lock.metadata()?;
        if !m.is_file() || m.uid() != 0 || m.mode() & 0o077 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)?;
        match std::fs::symlink_metadata(directory.join("policy.pending")) {
            Ok(_) => return Err(io::ErrorKind::InvalidData.into()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let path = directory.join("policy.json");
        let durable = if match std::fs::symlink_metadata(&path) {
            Ok(_) => true,
            Err(e) if e.kind() == io::ErrorKind::NotFound => false,
            Err(e) => return Err(e),
        } {
            let mut bytes = Vec::new();
            private_file(&path)?.take(32769).read_to_end(&mut bytes)?;
            if bytes.len() > 32768 {
                return Err(io::ErrorKind::InvalidData.into());
            }
            let value: Durable = serde_json::from_slice(&bytes)?;
            if value.grants.len() > crate::MAX_APPS * 2 || value.receipts.len() > MAX_RECEIPTS {
                return Err(io::ErrorKind::InvalidData.into());
            }
            for (i, grant) in value.grants.iter().enumerate() {
                if !profiles.iter().any(|p| p.app == grant.app)
                    || value.grants[..i]
                        .iter()
                        .any(|g| g.app == grant.app && g.permission == grant.permission)
                {
                    return Err(io::ErrorKind::InvalidData.into());
                }
            }
            if value.receipts.iter().any(|s| {
                s.len() != 49
                    || s.as_bytes()[32] != b'-'
                    || !s
                        .bytes()
                        .enumerate()
                        .all(|(i, b)| i == 32 || b.is_ascii_hexdigit())
            }) {
                return Err(io::ErrorKind::InvalidData.into());
            }
            value
        } else {
            Durable::default()
        };
        Ok(Self {
            profiles,
            principals: BTreeMap::new(),
            permits: BTreeMap::new(),
            durable,
            directory: directory.into(),
            _lock: lock,
            incarnation: fresh_id()?,
            sequence: 0,
            generation: 0,
            policy: 0,
            lifecycle: 0,
            locked: true,
            suspended: false,
            degraded: false,
        })
    }
    fn persist(&mut self, next: Durable) -> io::Result<()> {
        let result = (|| {
            let path = self.directory.join("policy.pending");
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)?;
            let bytes = serde_json::to_vec(&next)?;
            if bytes.len() > 32768 {
                return Err(io::ErrorKind::InvalidData.into());
            }
            file.write_all(&bytes)?;
            file.sync_all()?;
            std::fs::rename(path, self.directory.join("policy.json"))?;
            File::open(&self.directory)?.sync_all()?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.durable = next;
                Ok(())
            }
            Err(e) => {
                self.degraded = true;
                self.permits.clear();
                Err(e)
            }
        }
    }
    pub fn register(&mut self, app: &str, pid: u32, alive: OwnedFd) -> io::Result<()> {
        if self.degraded || !living(&alive)? {
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
                background: false,
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
    pub fn grant(&mut self, app: &str, permission: Permission, allowed: bool) -> io::Result<()> {
        if self.degraded || !self.profiles.iter().any(|p| p.app == app) {
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
        self.persist(next)
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
            self.locked = v;
        }
        if let Some(v) = suspended {
            self.suspended = v;
        }
        if let Some((app, v)) = background {
            self.principals
                .get_mut(app)
                .ok_or(io::ErrorKind::NotFound)?
                .background = v;
        }
        Ok(())
    }
    fn allowed(&self, app: &str, permission: Permission) -> bool {
        !self.locked
            && !self.suspended
            && self.principals.get(app).is_some_and(|p| !p.background)
            && self
                .durable
                .grants
                .iter()
                .any(|g| g.app == app && g.permission == permission && g.allowed)
    }
    fn id(&mut self) -> Option<String> {
        self.sequence = self.sequence.checked_add(1)?;
        Some(format!("{}-{:016x}", self.incarnation, self.sequence))
    }
    pub fn request(
        &mut self,
        pid: u32,
        uid: u32,
        request: AppRequest,
        mut clock: impl FnMut() -> u64,
    ) -> Reply {
        let now = clock();
        let _ = self.reap_dead();
        self.permits.retain(|_, p| now < p.until);
        if self.degraded {
            return Reply::Unavailable;
        }
        let Some(app) = self
            .principals
            .iter()
            .find(|(_, p)| p.uid == uid && p.pid == pid)
            .map(|(a, _)| a.clone())
        else {
            return Reply::Denied;
        };
        match request {
            AppRequest::Query {} => Reply::Ok,
            AppRequest::Prepare { permission, ttl_ms } => {
                if !(1..=5000).contains(&ttl_ms)
                    || !self.allowed(&app, permission)
                    || self.permits.len() >= MAX_PERMITS
                    || self.permits.values().filter(|p| p.app == app).count() >= 4
                {
                    return Reply::Denied;
                }
                let Some(until) = now.checked_add(ttl_ms * 1_000_000) else {
                    return Reply::Unavailable;
                };
                let Some(id) = self.id() else {
                    self.degraded = true;
                    return Reply::Unavailable;
                };
                let generation = self.principals[&app].generation;
                self.permits.insert(
                    id.clone(),
                    Permit {
                        app,
                        generation,
                        policy: self.policy,
                        lifecycle: self.lifecycle,
                        permission,
                        until,
                    },
                );
                Reply::Prepared { permit: id }
            }
            AppRequest::Admit { permit } => {
                // A foreign principal cannot consume another principal's reservation.
                if self.permits.get(&permit).is_none_or(|p| p.app != app) {
                    return Reply::Denied;
                }
                let Some(p) = self.permits.remove(&permit) else {
                    return Reply::Denied;
                };
                if p.app != app
                    || now >= p.until
                    || p.generation != self.principals[&app].generation
                    || p.policy != self.policy
                    || p.lifecycle != self.lifecycle
                    || !self.allowed(&app, p.permission)
                {
                    return Reply::Denied;
                }
                if self.durable.receipts.len() >= MAX_RECEIPTS {
                    return Reply::Unavailable;
                }
                let Some(id) = self.id() else {
                    self.degraded = true;
                    return Reply::Unavailable;
                };
                let mut next = self.durable.clone();
                next.receipts.push(id.clone());
                if self.persist(next).is_err() {
                    return Reply::Unavailable;
                }
                // Durability can consume the permit lifetime. Late success records
                // uncertainty but grants no current admission and cannot be retried.
                if clock() >= p.until || !living(&self.principals[&app].alive).unwrap_or(false) {
                    return Reply::Unavailable;
                }
                Reply::AdmittedUncertain { receipt: id }
            }
        }
    }
    pub fn is_degraded(&self) -> bool {
        self.degraded
    }
    pub fn snapshot(&mut self) -> Reply {
        let _ = self.reap_dead();
        Reply::Snapshot {
            principals: self.principals.len(),
            permits: self.permits.len(),
            receipts: self.durable.receipts.len(),
            locked: self.locked,
            suspended: self.suspended,
            degraded: self.degraded,
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
        fn authority(&self) -> Authority {
            let mut a = Authority::open(
                vec![AppProfile {
                    app: "one".into(),
                    uid: 10000,
                    executable: "/unused".into(),
                    args: vec![],
                }],
                &self.0,
            )
            .unwrap();
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
    fn request(a: &mut Authority, r: AppRequest, clock: impl FnMut() -> u64) -> Reply {
        // Direct component caller injection; actual credentials are qualified separately.
        a.request(std::process::id(), 10000, r, clock)
    }
    fn prepared(a: &mut Authority, now: u64) -> String {
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
            Reply::Denied
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
