//! Actual app Child and display-context ownership in one actor launch slot.
use crate::{
    AppProfile, Reply, authority::Authority, boottime_ns, display, fresh_id,
    service::trusted_executable,
};
use rustix::process::{Pid, PidfdFlags, Signal};
use std::{
    collections::BTreeMap,
    io,
    path::Path,
    process::{Child, Command, Stdio},
};

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Preparing,
    Running,
    Retiring,
}
const NORMAL_STOP_GRACE_NS: u64 = 500_000_000;
#[derive(Clone, Copy)]
enum Retirement {
    Grace { until: u64 },
    Forced,
}
struct Slot {
    profile: AppProfile,
    instance: String,
    child: Option<Child>,
    context: Option<Box<dyn display::Context>>,
    phase: Phase,
    retirement: Option<Retirement>,
    until: u64,
    connection: Option<u64>,
    reported: bool,
}
impl Slot {
    fn retire(&mut self, authority: &mut Authority) -> io::Result<()> {
        // Data fencing precedes pathname withdrawal and every child signal.
        authority.revoke(&self.profile.app);
        if matches!(self.retirement, Some(Retirement::Forced)) {
            return Ok(());
        }
        self.phase = Phase::Retiring;
        // Latch before fallible withdrawal/signalling. Forced cannot regain grace.
        self.retirement = Some(Retirement::Forced);
        let context = self.context.as_mut().map(|c| c.stop()).transpose();
        let child = self.child.as_mut().map(|c| c.kill()).transpose();
        context?;
        child?;
        Ok(())
    }
    fn normal_stop(&mut self, authority: &mut Authority, cleanup_until: u64) -> io::Result<()> {
        authority.revoke(&self.profile.app);
        if self.retirement.is_some() {
            return self.poll_retirement(authority);
        }
        if self.phase != Phase::Running {
            return self.retire(authority);
        }
        let now = boottime_ns();
        // Leave at least 500ms inside the existing actor budget for KILL/waits.
        let until = now
            .saturating_add(NORMAL_STOP_GRACE_NS)
            .min(cleanup_until.saturating_sub(NORMAL_STOP_GRACE_NS));
        if now >= until {
            return self.retire(authority);
        }
        self.phase = Phase::Retiring;
        self.retirement = Some(Retirement::Grace { until });
        // Withdrawal is attempted before TERM or an error-path KILL.
        if let Some(context) = self.context.as_mut()
            && let Err(error) = context.stop()
        {
            self.retirement = Some(Retirement::Forced);
            if let Some(child) = self.child.as_mut() {
                let _ = child.kill();
            }
            return Err(error);
        }
        let signalled = self
            .child
            .as_ref()
            .map(|child| {
                // This sole owner retains the unreaped Child, preventing PID reuse.
                let pid = Pid::from_raw(child.id() as i32)
                    .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
                rustix::process::kill_process(pid, Signal::TERM).map_err(io::Error::from)
            })
            .transpose();
        if let Err(error) = signalled {
            self.retirement = Some(Retirement::Forced);
            if let Some(child) = self.child.as_mut() {
                let _ = child.kill();
            }
            return Err(error);
        }
        Ok(())
    }
    fn poll_retirement(&mut self, authority: &mut Authority) -> io::Result<()> {
        if matches!(self.retirement, Some(Retirement::Grace { until }) if boottime_ns() >= until) {
            self.retire(authority)?;
        }
        Ok(())
    }
    fn occupied(&self) -> bool {
        self.child.is_some() || self.context.as_ref().is_some_and(|c| c.occupied())
    }
    fn completion(&mut self) -> Option<Completion> {
        if self.reported || self.phase == Phase::Preparing {
            return None;
        }
        self.reported = true;
        self.connection.map(|connection| Completion {
            instance: self.instance.clone(),
            connection,
            app: self.profile.app.clone(),
            reply: if self.phase == Phase::Running {
                Reply::Ok
            } else {
                Reply::Unavailable
            },
        })
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        if self.occupied() {
            // Rust Child drop detaches. Neither errors nor unwinding permit it.
            std::process::abort();
        }
    }
}

pub(crate) struct Completion {
    pub instance: String,
    pub connection: u64,
    pub app: String,
    pub reply: Reply,
}
pub(crate) enum Start {
    Rejected(Reply),
    Reserved { instance: String, until: u64 },
}
#[derive(Default)]
pub(crate) struct Apps {
    slots: BTreeMap<String, Slot>,
    #[cfg(test)]
    pub(crate) wait_observer: Option<std::sync::mpsc::SyncSender<(u32, std::process::ExitStatus)>>,
}
impl Apps {
    pub fn start(
        &mut self,
        profile: &AppProfile,
        connection: u64,
        authority: &Authority,
        factory: &mut Option<&mut dyn display::Factory>,
    ) -> Start {
        if !authority.ready() || authority.session_terminal() {
            return Start::Rejected(Reply::Unavailable);
        }
        if self.slots.contains_key(&profile.app) {
            return Start::Rejected(Reply::Denied);
        }
        let Some(until) = boottime_ns().checked_add(display::INITIALIZATION_BUDGET_NS) else {
            return Start::Rejected(Reply::Unavailable);
        };
        let instance = match fresh_id() {
            Ok(instance) => instance,
            Err(_) => return Start::Rejected(Reply::Unavailable),
        };
        let context = match factory.as_deref_mut() {
            Some(factory) => match factory.start(profile, &instance, until) {
                Ok(context) => Some(context),
                Err(_) => return Start::Rejected(Reply::Unavailable),
            },
            None => None,
        };
        self.slots.insert(
            profile.app.clone(),
            Slot {
                profile: profile.clone(),
                instance: instance.clone(),
                child: None,
                context,
                phase: Phase::Preparing,
                retirement: None,
                until,
                connection: Some(connection),
                reported: false,
            },
        );
        Start::Reserved { instance, until }
    }
    pub fn poll(&mut self, authority: &mut Authority) -> io::Result<Vec<Completion>> {
        let mut completions = Vec::new();
        for slot in self.slots.values_mut() {
            // Observe actual app wait before consuming a display state. A dead
            // app's pathname is withdrawn even if its SDK is held or still Ready.
            if let Some(child) = slot.child.as_mut()
                && let Some(status) = child.try_wait()?
            {
                #[cfg(test)]
                if let Some(observer) = &self.wait_observer {
                    observer
                        .try_send((child.id(), status))
                        .map_err(|_| io::Error::other("bounded app wait witness unavailable"))?;
                }
                #[cfg(not(test))]
                let _ = status;
                slot.child = None;
                if slot.phase != Phase::Retiring {
                    slot.retire(authority)?;
                }
            }
            if slot.phase != Phase::Retiring
                && (authority.session_terminal()
                    || (slot.phase == Phase::Preparing
                        && (!authority.ready() || boottime_ns() >= slot.until)))
            {
                slot.retire(authority)?;
            }
            let state = slot
                .context
                .as_mut()
                .map_or(Ok(display::State::Ready), |c| c.poll());
            if state.is_err()
                || (slot.phase != Phase::Retiring
                    && (!matches!(state, Ok(display::State::Ready | display::State::Preparing))
                        || (slot.phase == Phase::Running
                            && !matches!(state, Ok(display::State::Ready)))))
            {
                slot.retire(authority)?;
            }
            // Intentional Context::stop may report Terminal while still owning
            // helpers. Keep polling/draining it; only an error forces grace.
            slot.poll_retirement(authority)?;
            if let Some(completion) = slot.completion() {
                completions.push(completion);
            }
        }
        // A reported failure can remain occupied for multiple turns. Retirement
        // never releases the slot on pidfd death alone or permits a successor.
        self.slots
            .retain(|_, s| s.phase != Phase::Retiring || s.occupied() || !s.reported);
        Ok(completions)
    }
    /// Called only from the exact owning connection, after its bounded receive
    /// found neither queued EOF nor another request. General polling never spawns.
    pub fn launch(
        &mut self,
        instance: &str,
        connection: u64,
        entry: &Path,
        authority: &mut Authority,
    ) -> io::Result<Option<Completion>> {
        let Some(slot) = self
            .slots
            .values_mut()
            .find(|s| s.instance == instance && s.connection == Some(connection))
        else {
            return Ok(None);
        };
        if slot.phase != Phase::Preparing {
            return Ok(slot.completion());
        }
        let state = slot
            .context
            .as_mut()
            .map_or(Ok(display::State::Ready), |c| c.poll());
        if !authority.ready() || authority.session_terminal() || boottime_ns() >= slot.until {
            slot.retire(authority)?;
        } else if matches!(state, Ok(display::State::Preparing)) {
            return Ok(None);
        } else if !matches!(state, Ok(display::State::Ready)) {
            slot.retire(authority)?;
        } else {
            match spawn(entry, slot) {
                Ok(child) => {
                    // Retain the child before any fallible post-spawn operation.
                    slot.child = Some(child);
                    let state = slot
                        .context
                        .as_mut()
                        .map_or(Ok(display::State::Ready), |c| c.poll());
                    let child = slot.child.as_ref().unwrap();
                    let registered = Pid::from_raw(child.id() as i32)
                        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))
                        .and_then(|pid| {
                            rustix::process::pidfd_open(pid, PidfdFlags::NONBLOCK)
                                .map_err(io::Error::from)
                        })
                        .and_then(|fd| authority.register(&slot.profile.app, child.id(), fd));
                    if registered.is_ok()
                        && matches!(state, Ok(display::State::Ready))
                        && boottime_ns() < slot.until
                    {
                        slot.phase = Phase::Running;
                    } else {
                        slot.retire(authority)?;
                    }
                }
                Err(_) => slot.retire(authority)?,
            }
        }
        Ok(slot.completion())
    }
    pub fn acknowledge(&mut self, instance: &str) {
        for slot in self.slots.values_mut() {
            if slot.instance == instance {
                slot.connection = None;
            }
        }
    }
    pub fn current(&self, instance: &str) -> bool {
        self.slots
            .values()
            .any(|s| s.instance == instance && s.phase == Phase::Running)
    }
    pub fn cancel(&mut self, instance: &str, authority: &mut Authority) -> io::Result<()> {
        for slot in self.slots.values_mut() {
            if slot.instance == instance {
                slot.connection = None;
                slot.retire(authority)?;
            }
        }
        Ok(())
    }
    pub fn cancel_disconnected(
        &mut self,
        active: &[(u64, &str)],
        authority: &mut Authority,
    ) -> io::Result<()> {
        for slot in self.slots.values_mut() {
            if slot.connection.is_some_and(|id| {
                !active
                    .iter()
                    .any(|(connection, instance)| *connection == id && *instance == slot.instance)
            }) {
                slot.connection = None;
                slot.retire(authority)?;
            }
        }
        Ok(())
    }
    pub fn terminate(&mut self, app: &str, authority: &mut Authority) -> Reply {
        authority.revoke(app);
        match self.slots.get_mut(app) {
            Some(slot) => {
                if slot.retire(authority).is_ok() {
                    Reply::Ok
                } else {
                    Reply::Unavailable
                }
            }
            None => Reply::Denied,
        }
    }
    pub fn normal_stop(&mut self, authority: &mut Authority, cleanup_until: u64) -> io::Result<()> {
        let mut failed = None;
        for slot in self.slots.values_mut() {
            if let Err(error) = slot.normal_stop(authority, cleanup_until) {
                failed.get_or_insert(error);
            }
        }
        failed.map_or(Ok(()), Err)
    }
    pub fn stop(&mut self, authority: &mut Authority) -> io::Result<()> {
        let mut failed = None;
        for slot in self.slots.values_mut() {
            if let Err(e) = slot.retire(authority) {
                failed.get_or_insert(e);
            }
        }
        failed.map_or(Ok(()), Err)
    }
    pub fn occupied(&self) -> bool {
        !self.slots.is_empty()
    }
}
fn spawn(entry: &Path, slot: &Slot) -> io::Result<Child> {
    trusted_executable(entry)?;
    trusted_executable(Path::new(&slot.profile.executable))?;
    let arguments = slot
        .context
        .as_ref()
        .map_or(slot.profile.args.as_slice(), |c| c.arguments());
    if arguments.len() > 16 || arguments.iter().any(|a| a.len() > 256) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Command::new(entry)
        .arg("app-entry")
        .arg(slot.profile.uid.to_string())
        .arg(std::process::id().to_string())
        .arg(&slot.profile.executable)
        .args(arguments)
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}
