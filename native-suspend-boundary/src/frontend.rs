//! Exact frozen V1 Binder objects backed by the separate portable coordinator.
use crate::{BnWakeLock, ISuspendControl, ISystemSuspend, IWakeLock, WakeLockType};
use pokelinux_suspend::{Authority, Coordinator, Lifecycle, WakeLock};
use pokelinux_suspend::{BootClock, Clock};
use rsbinder::{
    BinderFeatures, BinderResult, DeathRecipient, ExceptionCode, Interface, SIBinder, Status,
    Strong, WIBinder,
};
use std::sync::{
    Arc, Condvar, Mutex, Weak,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

pub struct State {
    coordinator: Coordinator,
    acquisitions: AtomicU64,
    releases: AtomicU64,
    destructions: AtomicU64,
    caller: Mutex<(i64, i64, String)>,
    release_caller: Mutex<(i64, i64)>,
    hold: Mutex<bool>,
    changed: Condvar,
    authority: Mutex<Option<Binding>>,
    generation: AtomicU64,
}
struct Binding {
    _lease: Authority,
    token: SIBinder,
    watch: Arc<AuthorityDeath>,
    generation: u64,
}
impl Drop for Binding {
    fn drop(&mut self) {
        let _ = self.token.unlink_to_death_arc(&self.watch);
    }
}
struct AuthorityDeath {
    state: Weak<State>,
    generation: u64,
}
impl DeathRecipient for AuthorityDeath {
    fn binder_died(&self, _: &WIBinder) {
        if let Some(state) = self.state.upgrade() {
            let mut binding = state.authority.lock().unwrap();
            if binding
                .as_ref()
                .is_some_and(|b| b.generation == self.generation)
            {
                binding.take();
            }
        }
    }
}
fn allowed() -> BinderResult<()> {
    if rsbinder::get_calling_uid() != 2000 {
        Err(ExceptionCode::Security.into())
    } else {
        Ok(())
    }
}
fn unavailable() -> Status {
    Status::new_service_specific_error(31, Some("coordinator not available".into()))
}
pub fn features() -> BinderFeatures {
    let mut features = BinderFeatures::default();
    features.set_requesting_sid = true;
    features
}
impl State {
    pub fn new(coordinator: Coordinator) -> Arc<Self> {
        Arc::new(Self {
            coordinator,
            acquisitions: AtomicU64::new(0),
            releases: AtomicU64::new(0),
            destructions: AtomicU64::new(0),
            caller: Mutex::new((0, 0, String::new())),
            release_caller: Mutex::new((0, 0)),
            hold: Mutex::new(false),
            changed: Condvar::new(),
            authority: Mutex::new(None),
            generation: AtomicU64::new(0),
        })
    }
}
pub struct Suspend(pub Arc<State>);
impl Interface for Suspend {}
impl ISystemSuspend for Suspend {
    fn acquireWakeLock(&self, _: WakeLockType, name: &str) -> BinderResult<Strong<dyn IWakeLock>> {
        *self.0.caller.lock().unwrap() = (
            rsbinder::get_calling_uid() as i64,
            rsbinder::get_calling_pid() as i64,
            rsbinder::get_calling_sid()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
        allowed()?;
        if name.len() > 256 {
            return Err(ExceptionCode::IllegalArgument.into());
        }
        let lease = self.0.coordinator.acquire().map_err(|_| unavailable())?;
        self.0.acquisitions.fetch_add(1, Ordering::AcqRel);
        Ok(BnWakeLock::new_binder_with_features(
            Wake {
                state: self.0.clone(),
                lease,
                released: AtomicBool::new(false),
            },
            features(),
        ))
    }
}
struct Wake {
    state: Arc<State>,
    lease: WakeLock,
    released: AtomicBool,
}
impl Interface for Wake {}
impl IWakeLock for Wake {
    fn release(&self) -> BinderResult<()> {
        let mut held = self.state.hold.lock().unwrap();
        let deadline = BootClock.now() + Duration::from_secs(3);
        while *held {
            let remaining = deadline.saturating_sub(BootClock.now());
            if remaining.is_zero() {
                return Err(Status::new_service_specific_error(32, None));
            }
            held = self.state.changed.wait_timeout(held, remaining).unwrap().0;
        }
        *self.state.release_caller.lock().unwrap() = (
            rsbinder::get_calling_uid() as i64,
            rsbinder::get_calling_pid() as i64,
        );
        if !self.released.swap(true, Ordering::AcqRel) {
            self.lease.release();
            self.state.releases.fetch_add(1, Ordering::AcqRel);
        }
        Ok(())
    }
}
impl Drop for Wake {
    fn drop(&mut self) {
        self.state.destructions.fetch_add(1, Ordering::AcqRel);
        let _ = &self.lease;
    }
}
pub struct Control(pub Arc<State>);
impl Interface for Control {}
impl ISuspendControl for Control {
    fn snapshot(&self) -> BinderResult<Vec<i64>> {
        allowed()?;
        let s = self.0.coordinator.snapshot();
        let caller = self.0.caller.lock().unwrap();
        let release = self.0.release_caller.lock().unwrap();
        Ok(vec![
            s.wake_locks as i64,
            match s.lifecycle {
                Lifecycle::Ready => 0,
                Lifecycle::Stopped => 1,
                Lifecycle::Degraded => 2,
                Lifecycle::AuthorityLost => 3,
            },
            self.0.acquisitions.load(Ordering::Acquire) as i64,
            self.0.releases.load(Ordering::Acquire) as i64,
            caller.0,
            caller.1,
            i64::from(s.armed),
            self.0.generation.load(Ordering::Acquire) as i64,
            release.0,
            release.1,
            self.0.destructions.load(Ordering::Acquire) as i64,
        ])
    }
    fn callerSid(&self) -> BinderResult<String> {
        allowed()?;
        Ok(self.0.caller.lock().unwrap().2.clone())
    }
    fn holdRelease(&self, hold: bool) -> BinderResult<()> {
        allowed()?;
        *self.0.hold.lock().unwrap() = hold;
        self.0.changed.notify_all();
        Ok(())
    }
    fn arm(&self, token: &SIBinder) -> BinderResult<()> {
        allowed()?;
        if !token.is_remote() {
            return Err(ExceptionCode::IllegalArgument.into());
        }
        let mut binding = self.0.authority.lock().unwrap();
        if binding.is_some() {
            return Err(unavailable());
        }
        let generation = self.0.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let watch = Arc::new(AuthorityDeath {
            state: Arc::downgrade(&self.0),
            generation,
        });
        token.link_to_death_arc(&watch)?;
        let lease = match self.0.coordinator.arm() {
            Ok(a) => a,
            Err(_) => {
                let _ = token.unlink_to_death_arc(&watch);
                return Err(unavailable());
            }
        };
        *binding = Some(Binding {
            _lease: lease,
            token: token.clone(),
            watch,
            generation,
        });
        Ok(())
    }
    fn attempt(&self) -> BinderResult<i32> {
        allowed()?;
        Ok(match self.0.coordinator.attempt() {
            pokelinux_suspend::Attempt::Disabled => 0,
            pokelinux_suspend::Attempt::Returned => 1,
            _ => 2,
        })
    }
    fn die(&self) -> BinderResult<()> {
        allowed()?;
        self.0.coordinator.stop();
        std::process::exit(77);
    }
}
