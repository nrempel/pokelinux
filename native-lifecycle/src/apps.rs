//! Actual app Child and display-context ownership in one actor launch slot.
use crate::{
    AppProfile, Reply, authority::Authority, boottime_ns, display, fresh_id,
    service::trusted_executable,
};
use rustix::process::{Pid, PidfdFlags};
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
struct Slot {
    profile: AppProfile,
    instance: String,
    child: Option<Child>,
    context: Option<Box<dyn display::Context>>,
    phase: Phase,
    until: u64,
    connection: Option<u64>,
    reported: bool,
}
impl Slot {
    fn retire(&mut self, authority: &mut Authority) -> io::Result<()> {
        // Data fencing precedes pathname withdrawal and every child signal.
        authority.revoke(&self.profile.app);
        self.phase = Phase::Retiring;
        let context = self.context.as_mut().map(|c| c.stop()).transpose();
        let child = self.child.as_mut().map(|c| c.kill()).transpose();
        context?;
        child?;
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
                && child.try_wait()?.is_some()
            {
                slot.child = None;
                slot.retire(authority)?;
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
            if !matches!(state, Ok(display::State::Ready | display::State::Preparing))
                || (slot.phase == Phase::Running && !matches!(state, Ok(display::State::Ready)))
            {
                slot.retire(authority)?;
            }
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
