//! Fixed IPv4 on an existing, exclusively administered link.
//! Namespace/name/index observations do not establish ownership. The trusted
//! caller must retain the radio/link lifecycle and exclude other administrators.
//! No association, link creation, route operation or destructive Drop is provided.
use crate::kernel::{
    self, Error, Mutator, before, collect, command_deadline, fixed_ipv4_state, identity, io_error,
    mutate_fixed_ipv4,
};
use pokelinux_network_state::{KernelObserver, KernelSnapshot, NamespaceIdentity, boot_time_ns};
use rustix::thread::{self, CapabilitySet};
use std::{
    fs::File,
    io::{self, Write},
    thread::ThreadId,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExistingLinkIdentity {
    pub namespace: NamespaceIdentity,
    pub ifindex: u32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileFact {
    pub target: ExistingLinkIdentity,
    /// A pre-existing link has no witnessed create epoch. None is an exact
    /// baseline, never a wildcard or an ownership token.
    pub observed_lifetime: Option<u64>,
    pub configured: bool,
    pub observed_at_boottime_ns: u64,
}

struct Origin {
    retained: File,
    namespace: NamespaceIdentity,
    thread: ThreadId,
}
impl Origin {
    fn retain(expected: NamespaceIdentity) -> Result<Self, Error> {
        let retained = File::open("/proc/thread-self/ns/net").map_err(io_error)?;
        if identity(&retained)? != expected {
            return Err(Error::Conflict);
        }
        let value = Self {
            retained,
            namespace: expected,
            thread: std::thread::current().id(),
        };
        value.check()?;
        Ok(value)
    }
    fn check(&self) -> Result<(), Error> {
        if std::thread::current().id() != self.thread {
            return Err(Error::Conflict);
        }
        let current = File::open("/proc/thread-self/ns/net").map_err(io_error)?;
        if identity(&self.retained)? != self.namespace || identity(&current)? != self.namespace {
            return Err(Error::Conflict);
        }
        Ok(())
    }
}

/// Retain this same object for a persistent owner's epoch. Every action uses
/// its original at-most-one-second cutoff. A possibly dispatched failure or
/// conflicting/lost observation retires all further methods without retry,
/// replacement or rollback. Dropping only releases descriptors.
pub struct ExistingIpv4Profile {
    origin: Origin,
    observer: KernelObserver,
    mutator: Mutator,
    target: ExistingLinkIdentity,
    observed_lifetime: Option<u64>,
    configured: bool,
    terminal: bool,
}
impl ExistingIpv4Profile {
    /// Attach in the constructing thread's namespace. expected_configured is
    /// required prior state, not acceptance of an earlier uncertain operation.
    /// Pre-existing identity cannot prove continuity before subscription; the
    /// trusted exclusive lifecycle owner must provide that custody.
    pub fn attach_existing(
        target: ExistingLinkIdentity,
        expected_configured: bool,
        original_until_ns: u64,
    ) -> Result<Self, Error> {
        if !valid_target(&target) {
            return Err(Error::InvalidData);
        }
        command_deadline(original_until_ns)?;
        let origin = Origin::retain(target.namespace)?;
        let mut observer = KernelObserver::subscribe_current_namespace(original_until_ns)
            .map_err(Error::Observation)?;
        let snapshot = collect(&mut observer, original_until_ns)?;
        // An interface newly created after initial subscription is a different
        // lifecycle, not an acceptable replacement for this existing target.
        let observed = selected_profile(&snapshot, &target, None, expected_configured)?;
        origin.check()?;
        let mutator = Mutator::new()?;
        origin.check()?;
        before(original_until_ns)?;
        Ok(Self {
            origin,
            observer,
            mutator,
            target,
            observed_lifetime: observed.observed_lifetime,
            configured: observed.configured,
            terminal: false,
        })
    }

    fn active(&mut self) -> Result<(), Error> {
        if self.terminal {
            return Err(Error::Uncertain);
        }
        if let Err(error) = self.origin.check() {
            self.terminal = true;
            return Err(error);
        }
        Ok(())
    }
    fn read_profile(&mut self, expected: bool, until: u64) -> Result<ProfileFact, Error> {
        self.origin.check()?;
        let snapshot = collect(&mut self.observer, until)?;
        let fact = selected_profile(&snapshot, &self.target, self.observed_lifetime, expected)?;
        self.origin.check()?;
        before(until)?;
        Ok(fact)
    }

    /// Exactly one explicit fixed-address choice after fresh prior-state readback.
    /// Kernel ACK0 alone is insufficient; the selected profile must be observed.
    pub fn set_configured(
        &mut self,
        configured: bool,
        original_until_ns: u64,
    ) -> Result<ProfileFact, Error> {
        self.active()?;
        command_deadline(original_until_ns)?;
        let prior = match self.read_profile(self.configured, original_until_ns) {
            Ok(fact) => fact,
            Err(error) => {
                self.terminal = true;
                return Err(error);
            }
        };
        if prior.configured == configured {
            before(original_until_ns)?;
            return Ok(prior);
        }
        match mutate_fixed_ipv4(
            &mut self.mutator,
            self.target.ifindex,
            configured,
            original_until_ns,
        ) {
            Ok(()) => {}
            // This is the last pre-send check. Later expiry is uncertain.
            Err(Error::Deadline) => return Err(Error::Deadline),
            Err(_) => {
                self.terminal = true;
                return Err(Error::Uncertain);
            }
        }
        match self.read_profile(configured, original_until_ns) {
            Ok(fact) if before(original_until_ns).is_ok() => {
                self.configured = fact.configured;
                Ok(fact)
            }
            _ => {
                self.terminal = true;
                Err(Error::Uncertain)
            }
        }
    }

    /// Fresh read-only observation; conflicting state cannot be silently adopted.
    pub fn refresh(&mut self, original_until_ns: u64) -> Result<ProfileFact, Error> {
        self.active()?;
        command_deadline(original_until_ns)?;
        match self.read_profile(self.configured, original_until_ns) {
            Ok(fact) => Ok(fact),
            Err(error) => {
                self.terminal = true;
                Err(error)
            }
        }
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 15
        && name != "lo"
        && name
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"_.-".contains(&v))
}

fn valid_target(target: &ExistingLinkIdentity) -> bool {
    target.ifindex > 1 && target.namespace.ino > 0 && valid_name(&target.name)
}

fn selected_profile(
    snapshot: &KernelSnapshot,
    target: &ExistingLinkIdentity,
    lifetime: Option<u64>,
    expected: bool,
) -> Result<ProfileFact, Error> {
    let mut matches = snapshot
        .links
        .iter()
        .filter(|link| link.ifindex == target.ifindex);
    let link = matches.next().ok_or(Error::Conflict)?;
    if matches.next().is_some()
        || snapshot.namespace != target.namespace
        || link.name != target.name
        || link.observed_lifetime != lifetime
        || link.flags & 1 == 0
    {
        return Err(Error::Conflict);
    }
    let configured = fixed_ipv4_state(link)?;
    if configured != expected {
        return Err(Error::Conflict);
    }
    Ok(ProfileFact {
        target: target.clone(),
        observed_lifetime: link.observed_lifetime,
        configured,
        observed_at_boottime_ns: snapshot.observed_at_boottime_ns,
    })
}

fn parse_cli(arguments: &[String]) -> io::Result<(bool, ExistingLinkIdentity, u64)> {
    let invalid = || io::Error::from(io::ErrorKind::InvalidInput);
    if arguments.len() != 6 {
        return Err(invalid());
    }
    let configured = match arguments[0].as_str() {
        "profile-up" => true,
        "profile-down" => false,
        _ => return Err(invalid()),
    };
    let number = |index: usize| crate::protocol::decimal(&arguments[index]).ok_or_else(invalid);
    let namespace = NamespaceIdentity {
        dev: number(1)?,
        ino: number(2)?,
    };
    let ifindex = u32::try_from(number(3)?).map_err(|_| invalid())?;
    let target = ExistingLinkIdentity {
        namespace,
        ifindex,
        name: arguments[4].clone(),
    };
    if !valid_target(&target) {
        return Err(invalid());
    }
    Ok((configured, target, number(5)?))
}

fn profile_rights() -> io::Result<()> {
    if thread::gettid() != rustix::process::getpid()
        || std::fs::read_dir("/proc/self/task")?.count() != 1
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let keep = CapabilitySet::NET_ADMIN;
    let actual = thread::capabilities(None)?;
    let last: u32 = std::fs::read_to_string("/proc/sys/kernel/cap_last_cap")?
        .trim()
        .parse()
        .map_err(io::Error::other)?;
    if last >= 64 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    // The owned launcher may already have irreversibly reduced this process.
    // Do not attempt privileged CAPBSET_DROP on bits which are already absent.
    let mut minimal = actual.effective == keep
        && actual.permitted == keep
        && actual.inheritable.is_empty()
        && thread::no_new_privs()?;
    for index in 0..=last {
        let cap = CapabilitySet::from_bits_retain(1_u64 << index);
        minimal &= thread::capability_is_in_bounding_set(cap)? == keep.contains(cap)
            && !thread::capability_is_in_ambient_set(cap)?;
    }
    if minimal {
        Ok(())
    } else {
        crate::service::reduce(keep)
    }
}

/// Normal one-shot trusted-worker modes. This does not enter or repair the
/// persistent Service's ReceiveCommand loop, authenticate apps, or own a radio.
pub fn run_cli(arguments: &[String]) -> io::Result<()> {
    let (configured, target, until) = parse_cli(arguments)?;
    profile_rights()?;
    let started = boot_time_ns();
    let error = |error: Error| io::Error::other(format!("fixed profile: {error:?}"));
    let mut profile =
        ExistingIpv4Profile::attach_existing(target, !configured, until).map_err(error)?;
    let fact = profile.set_configured(configured, until).map_err(error)?;
    let completed = boot_time_ns();
    if completed >= until {
        return Err(error(Error::Uncertain));
    }
    let value = serde_json::json!({"schema":1,"status":"completed", "operation":arguments[0],
        "namespace":{"dev":fact.target.namespace.dev,"ino":fact.target.namespace.ino},
        "ifindex":fact.target.ifindex,"ifname":fact.target.name,
        "profile":{"local":kernel::LOCAL_ADDRESS.to_string(),"prefix_len":kernel::LOCAL_PREFIX},
        "configured":fact.configured,"observed_lifetime":fact.observed_lifetime,
        "observed_at_boottime_ns":fact.observed_at_boottime_ns,"original_until_ns":until,
        "started_boottime_ns":started,"completed_boottime_ns":completed});
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    if boot_time_ns() >= until {
        return Err(error(Error::Uncertain));
    }
    io::stdout().lock().write_all(&bytes)
}

#[cfg(test)]
#[path = "fixed_address_tests.rs"]
mod tests;
