//! Retained open-radio and fixed-IPv4 ownership for one exclusive worker epoch.
//! Dropping this object releases descriptors; only an explicit successful
//! Disconnect removes the profile and association. No recovery or retry is run.
use crate::{
    fixed_address::{ExistingIpv4Profile, ExistingLinkIdentity, ProfileFact},
    kernel::{self, Error},
};
use pokelinux_network_state::{NamespaceIdentity, ObserveError};
use pokelinux_wireless_scan::{self as wireless, RadioEpoch, RadioFact, RadioTarget};
use std::thread::ThreadId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioOperation {
    Attach,
    Refresh,
    Associate,
    Disconnect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileOperation {
    Attach,
    Refresh,
    Add,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentFailure {
    Radio {
        operation: RadioOperation,
        error: wireless::Error,
    },
    Profile {
        operation: ProfileOperation,
        error: Error,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    /// The composed action's classification, including any earlier mutation.
    pub classification: Error,
    /// First concrete component error, before classification or composition.
    pub component: Option<ComponentFailure>,
}
impl Failure {
    fn radio(operation: RadioOperation, error: wireless::Error) -> Self {
        Self {
            classification: radio_error(error),
            component: Some(ComponentFailure::Radio { operation, error }),
        }
    }
    fn profile(operation: ProfileOperation, error: Error) -> Self {
        Self {
            classification: error,
            component: Some(ComponentFailure::Profile { operation, error }),
        }
    }
}
impl From<Error> for Failure {
    fn from(classification: Error) -> Self {
        Self {
            classification,
            component: None,
        }
    }
}

#[derive(Default)]
struct Diagnostic(Option<ComponentFailure>);
impl Diagnostic {
    fn radio(&mut self, operation: RadioOperation, error: wireless::Error) -> wireless::Error {
        self.0
            .get_or_insert(ComponentFailure::Radio { operation, error });
        error
    }
    fn profile(&mut self, operation: ProfileOperation, error: Error) -> Error {
        self.0
            .get_or_insert(ComponentFailure::Profile { operation, error });
        error
    }
    fn finish(self, classification: Error) -> Failure {
        Failure {
            classification,
            component: self.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WirelessFact {
    pub namespace: NamespaceIdentity,
    pub ifindex: u32,
    pub wiphy: u32,
    pub wdev: u64,
    pub observed_lifetime: Option<u64>,
    pub connected: bool,
    pub observed_at_ns: u64,
}

struct Expected {
    target: ExistingLinkIdentity,
    observed_lifetime: Option<u64>,
    wiphy: u32,
    wdev: u64,
}

/// The trusted caller must retain exclusive radio and namespace custody. Link
/// identity and subscriptions detect loss; neither establishes that authority.
pub struct WirelessBackend {
    radio: RadioEpoch,
    profile: ExistingIpv4Profile,
    expected: Expected,
    thread: ThreadId,
    connected: bool,
    terminal: bool,
}
impl WirelessBackend {
    /// Subscribe to the retained radio before attaching the address observer.
    /// Both baselines must be absent under the same original one-second bound.
    pub fn attach(
        target: ExistingLinkIdentity,
        frequency_mhz: u32,
        bssid: [u8; 6],
        ssid: Vec<u8>,
        until: u64,
    ) -> Result<Self, Failure> {
        kernel::command_deadline(until)?;
        let thread = std::thread::current().id();
        let mut radio = RadioEpoch::attach_disconnected(
            RadioTarget {
                namespace_dev: target.namespace.dev,
                namespace_ino: target.namespace.ino,
                ifindex: target.ifindex,
                frequency_mhz,
                bssid,
                ssid,
            },
            until,
        )
        .map_err(|error| Failure::radio(RadioOperation::Attach, error))?;
        let first = radio
            .refresh(until)
            .map_err(|error| Failure::radio(RadioOperation::Refresh, error))?;
        if first.associated {
            return Err(Error::Conflict.into());
        }
        let mut profile = ExistingIpv4Profile::attach_existing(target.clone(), false, until)
            .map_err(|error| Failure::profile(ProfileOperation::Attach, error))?;
        let address = profile
            .refresh(until)
            .map_err(|error| Failure::profile(ProfileOperation::Refresh, error))?;
        let last = radio
            .refresh(until)
            .map_err(|error| Failure::radio(RadioOperation::Refresh, error))?;
        let expected = Expected {
            target,
            observed_lifetime: address.observed_lifetime,
            wiphy: first.wiphy,
            wdev: first.wdev,
        };
        combine(&expected, false, first, &address, last)?;
        kernel::before(until)?;
        Ok(Self {
            radio,
            profile,
            expected,
            thread,
            connected: false,
            terminal: false,
        })
    }

    fn active(&mut self) -> Result<(), Error> {
        if self.terminal {
            return Err(Error::Uncertain);
        }
        if self.thread != std::thread::current().id() {
            self.terminal = true;
            return Err(Error::Conflict);
        }
        Ok(())
    }

    /// Read radio/profile/radio on the original retained observers. No cached
    /// association is allowed to renew an address-only observation's readiness.
    pub fn refresh(&mut self, until: u64) -> Result<WirelessFact, Failure> {
        self.active()?;
        let mut diagnostic = Diagnostic::default();
        let result = refresh_parts(
            &mut Retained {
                radio: &mut self.radio,
                profile: &mut self.profile,
                diagnostic: &mut diagnostic,
            },
            &self.expected,
            self.connected,
            &mut self.terminal,
            until,
        );
        result.map_err(|classification| diagnostic.finish(classification))
    }

    /// Connect confirms the exact radio association before adding the profile.
    /// Disconnect confirms profile removal before submitting radio Disconnect.
    /// Every stage and the final bracketed read use this unchanged cutoff.
    pub fn set_connected(&mut self, connected: bool, until: u64) -> Result<WirelessFact, Failure> {
        self.active()?;
        let mut diagnostic = Diagnostic::default();
        let result = set_parts(
            &mut Retained {
                radio: &mut self.radio,
                profile: &mut self.profile,
                diagnostic: &mut diagnostic,
            },
            &self.expected,
            &mut self.connected,
            &mut self.terminal,
            connected,
            until,
        );
        result.map_err(|classification| diagnostic.finish(classification))
    }
}

fn radio_error(error: wireless::Error) -> Error {
    match error {
        wireless::Error::Uncertain(_) => Error::Uncertain,
        wireless::Error::Unavailable => Error::Observation(ObserveError::Unsupported),
        wireless::Error::InvalidRequest => Error::InvalidData,
        wireless::Error::Failed(failure) => match failure {
            wireless::Failure::Expired => Error::Deadline,
            wireless::Failure::Identity | wireless::Failure::Contended => Error::Conflict,
            wireless::Failure::Loss => Error::Observation(ObserveError::EvidenceLost),
            wireless::Failure::Limit => Error::Observation(ObserveError::BoundExceeded),
            wireless::Failure::Unsupported => Error::Observation(ObserveError::Unsupported),
            wireless::Failure::Kernel(code) => Error::Io(code),
            _ => Error::InvalidData,
        },
    }
}

fn retained_radio_error(error: wireless::Error) -> Error {
    // On an attached epoch these methods take only a cutoff: selectors were
    // already validated. InvalidRequest therefore means an unsent bad cutoff.
    if error == wireless::Error::InvalidRequest {
        Error::Deadline
    } else {
        radio_error(error)
    }
}

// This private seam exercises only sequencing and composed uncertainty. The
// production implementation always delegates to the two retained primitives.
trait Components {
    fn radio_refresh(&mut self, until: u64) -> Result<RadioFact, Error>;
    fn profile_refresh(&mut self, until: u64) -> Result<ProfileFact, Error>;
    fn associate(&mut self, value: bool, until: u64) -> Result<RadioFact, Error>;
    fn configure(&mut self, value: bool, until: u64) -> Result<ProfileFact, Error>;
    fn before(&mut self, until: u64) -> Result<(), Error>;
}
struct Retained<'a> {
    radio: &'a mut RadioEpoch,
    profile: &'a mut ExistingIpv4Profile,
    diagnostic: &'a mut Diagnostic,
}
impl Components for Retained<'_> {
    fn radio_refresh(&mut self, until: u64) -> Result<RadioFact, Error> {
        self.radio.refresh(until).map_err(|error| {
            retained_radio_error(self.diagnostic.radio(RadioOperation::Refresh, error))
        })
    }
    fn profile_refresh(&mut self, until: u64) -> Result<ProfileFact, Error> {
        self.profile
            .refresh(until)
            .map_err(|error| self.diagnostic.profile(ProfileOperation::Refresh, error))
    }
    fn associate(&mut self, value: bool, until: u64) -> Result<RadioFact, Error> {
        self.radio.set_associated(value, until).map_err(|error| {
            let operation = if value {
                RadioOperation::Associate
            } else {
                RadioOperation::Disconnect
            };
            retained_radio_error(self.diagnostic.radio(operation, error))
        })
    }
    fn configure(&mut self, value: bool, until: u64) -> Result<ProfileFact, Error> {
        self.profile.set_configured(value, until).map_err(|error| {
            let operation = if value {
                ProfileOperation::Add
            } else {
                ProfileOperation::Remove
            };
            self.diagnostic.profile(operation, error)
        })
    }
    fn before(&mut self, until: u64) -> Result<(), Error> {
        kernel::before(until)
    }
}

fn check_radio(expected: &Expected, connected: bool, fact: RadioFact) -> Result<(), Error> {
    if fact.associated != connected
        || fact.wiphy != expected.wiphy
        || fact.wdev != expected.wdev
        || fact.observed_at_boottime_ns == 0
    {
        Err(Error::Conflict)
    } else {
        Ok(())
    }
}
fn check_profile(expected: &Expected, connected: bool, fact: &ProfileFact) -> Result<(), Error> {
    if fact.target != expected.target
        || fact.observed_lifetime != expected.observed_lifetime
        || fact.configured != connected
        || fact.observed_at_boottime_ns == 0
    {
        Err(Error::Conflict)
    } else {
        Ok(())
    }
}
fn combine(
    expected: &Expected,
    connected: bool,
    first: RadioFact,
    profile: &ProfileFact,
    last: RadioFact,
) -> Result<WirelessFact, Error> {
    check_radio(expected, connected, first)?;
    check_profile(expected, connected, profile)?;
    check_radio(expected, connected, last)?;
    Ok(WirelessFact {
        namespace: expected.target.namespace,
        ifindex: expected.target.ifindex,
        wiphy: expected.wiphy,
        wdev: expected.wdev,
        observed_lifetime: expected.observed_lifetime,
        connected,
        observed_at_ns: first
            .observed_at_boottime_ns
            .min(profile.observed_at_boottime_ns)
            .min(last.observed_at_boottime_ns),
    })
}
fn observe(
    parts: &mut impl Components,
    expected: &Expected,
    connected: bool,
    until: u64,
) -> Result<WirelessFact, Error> {
    let first = parts.radio_refresh(until)?;
    let profile = parts.profile_refresh(until)?;
    let last = parts.radio_refresh(until)?;
    let fact = combine(expected, connected, first, &profile, last)?;
    parts.before(until)?;
    Ok(fact)
}
fn refresh_parts(
    parts: &mut impl Components,
    expected: &Expected,
    connected: bool,
    terminal: &mut bool,
    until: u64,
) -> Result<WirelessFact, Error> {
    if *terminal {
        return Err(Error::Uncertain);
    }
    kernel::command_deadline(until)?;
    observe(parts, expected, connected, until).inspect_err(|_| *terminal = true)
}

#[derive(Default)]
struct ActionProgress {
    changed: bool,
}
impl ActionProgress {
    fn failure(&self, error: Error) -> Error {
        if self.changed {
            Error::Uncertain
        } else {
            error
        }
    }
    fn changing<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        let value = result.map_err(|error| self.failure(error))?;
        self.changed = true;
        Ok(value)
    }
    fn checked<T>(&self, result: Result<T, Error>) -> Result<T, Error> {
        result.map_err(|error| self.failure(error))
    }
}
fn apply(
    parts: &mut impl Components,
    expected: &Expected,
    current: bool,
    desired: bool,
    until: u64,
) -> Result<WirelessFact, Error> {
    let prior = observe(parts, expected, current, until)?;
    if current == desired {
        return Ok(prior);
    }
    let mut progress = ActionProgress::default();
    let (radio, profile) = if desired {
        let radio = progress.changing(parts.associate(true, until))?;
        progress.checked(check_radio(expected, true, radio))?;
        let profile = progress.changing(parts.configure(true, until))?;
        progress.checked(check_profile(expected, true, &profile))?;
        (radio, profile)
    } else {
        let profile = progress.changing(parts.configure(false, until))?;
        progress.checked(check_profile(expected, false, &profile))?;
        let radio = progress.changing(parts.associate(false, until))?;
        progress.checked(check_radio(expected, false, radio))?;
        (radio, profile)
    };
    let mut fact = progress.checked(observe(parts, expected, desired, until))?;
    fact.observed_at_ns = fact
        .observed_at_ns
        .min(radio.observed_at_boottime_ns)
        .min(profile.observed_at_boottime_ns);
    Ok(fact)
}
fn set_parts(
    parts: &mut impl Components,
    expected: &Expected,
    connected: &mut bool,
    terminal: &mut bool,
    desired: bool,
    until: u64,
) -> Result<WirelessFact, Error> {
    if *terminal {
        return Err(Error::Uncertain);
    }
    kernel::command_deadline(until)?;
    match apply(parts, expected, *connected, desired, until) {
        Ok(fact) => {
            *connected = desired;
            Ok(fact)
        }
        Err(error) => {
            // A component Deadline can also be a retired observer's runtime
            // failure. Only our outer pure deadline guard permits another call.
            *terminal = true;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn diagnostic_retains_raw_radio_detail_and_composed_classification() {
        let raw = wireless::Error::Failed(wireless::Failure::MissingAssociationAttribute {
            stage: 3,
            command: 33,
            attribute: 47,
        });
        let mut diagnostic = Diagnostic::default();
        let mapped = retained_radio_error(diagnostic.radio(RadioOperation::Refresh, raw));
        diagnostic.profile(ProfileOperation::Remove, Error::Deadline);
        let failure = diagnostic.finish(mapped);
        assert_eq!(failure.classification, Error::InvalidData);
        assert_eq!(
            failure.component,
            Some(ComponentFailure::Radio {
                operation: RadioOperation::Refresh,
                error: raw,
            })
        );

        let mut progress = ActionProgress::default();
        progress.changing(Ok(())).unwrap();
        let mut diagnostic = Diagnostic::default();
        let mapped = retained_radio_error(diagnostic.radio(RadioOperation::Disconnect, raw));
        let failure = diagnostic.finish(progress.failure(mapped));
        assert_eq!(failure.classification, Error::Uncertain);
        assert_eq!(
            failure.component,
            Some(ComponentFailure::Radio {
                operation: RadioOperation::Disconnect,
                error: raw,
            })
        );
    }

    #[test]
    fn diagnostic_preserves_profile_origin_and_attach_refusal_without_an_effect() {
        let mut diagnostic = Diagnostic::default();
        let raw = Error::Observation(ObserveError::EvidenceLost);
        let mapped = diagnostic.profile(ProfileOperation::Add, raw);
        let failure = diagnostic.finish(ActionProgress { changed: true }.failure(mapped));
        assert_eq!(failure.classification, Error::Uncertain);
        assert_eq!(
            failure.component,
            Some(ComponentFailure::Profile {
                operation: ProfileOperation::Add,
                error: raw,
            })
        );

        let failure = WirelessBackend::attach(expected().target, 0, [2; 6], vec![1], until())
            .err()
            .unwrap();
        assert_eq!(
            failure,
            Failure::radio(RadioOperation::Attach, wireless::Error::InvalidRequest)
        );
        let failure = WirelessBackend::attach(expected().target, 2412, [2; 6], vec![1], 0)
            .err()
            .unwrap();
        assert_eq!(
            failure,
            Failure {
                classification: Error::Deadline,
                component: None
            }
        );
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Step {
        Radio,
        Profile,
        Check,
        Associate(bool),
        Configure(bool),
    }

    // Authored component outcomes test only the composition contract. They do
    // not establish kernel dispatch, event continuity or radio/IP behavior.
    struct Fake {
        expected: Expected,
        associated: bool,
        configured: bool,
        calls: Vec<(Step, u64)>,
        fail_at: Option<(usize, Error)>,
        contradict_at: Option<usize>,
        radio_times: VecDeque<u64>,
        profile_times: VecDeque<u64>,
    }
    impl Fake {
        fn new(connected: bool) -> Self {
            Self {
                expected: expected(),
                associated: connected,
                configured: connected,
                calls: vec![],
                fail_at: None,
                contradict_at: None,
                radio_times: [100, 300, 400, 600].into(),
                profile_times: [200, 500].into(),
            }
        }
        fn call(&mut self, step: Step, until: u64) -> Result<(), Error> {
            self.calls.push((step, until));
            if let Some((index, error)) = self.fail_at {
                if self.calls.len() == index {
                    return Err(error);
                }
            }
            Ok(())
        }
        fn radio_fact(&mut self) -> RadioFact {
            RadioFact {
                associated: self.associated ^ (self.contradict_at == Some(self.calls.len())),
                observed_at_boottime_ns: self.radio_times.pop_front().unwrap_or(700),
                wiphy: self.expected.wiphy,
                wdev: self.expected.wdev,
            }
        }
        fn profile_fact(&mut self) -> ProfileFact {
            ProfileFact {
                target: self.expected.target.clone(),
                observed_lifetime: self.expected.observed_lifetime,
                configured: self.configured ^ (self.contradict_at == Some(self.calls.len())),
                observed_at_boottime_ns: self.profile_times.pop_front().unwrap_or(700),
            }
        }
    }
    impl Components for Fake {
        fn radio_refresh(&mut self, until: u64) -> Result<RadioFact, Error> {
            self.call(Step::Radio, until)?;
            Ok(self.radio_fact())
        }
        fn profile_refresh(&mut self, until: u64) -> Result<ProfileFact, Error> {
            self.call(Step::Profile, until)?;
            Ok(self.profile_fact())
        }
        fn associate(&mut self, value: bool, until: u64) -> Result<RadioFact, Error> {
            self.call(Step::Associate(value), until)?;
            self.associated = value;
            Ok(self.radio_fact())
        }
        fn configure(&mut self, value: bool, until: u64) -> Result<ProfileFact, Error> {
            self.call(Step::Configure(value), until)?;
            self.configured = value;
            Ok(self.profile_fact())
        }
        fn before(&mut self, until: u64) -> Result<(), Error> {
            self.call(Step::Check, until)
        }
    }
    fn expected() -> Expected {
        Expected {
            target: ExistingLinkIdentity {
                namespace: NamespaceIdentity { dev: 5, ino: 42 },
                ifindex: 3,
                name: "wlan1".into(),
            },
            observed_lifetime: None,
            wiphy: 11,
            wdev: 17,
        }
    }
    fn until() -> u64 {
        pokelinux_network_state::boot_time_ns() + 1_000_000_000
    }
    fn run(
        fake: &mut Fake,
        current: &mut bool,
        terminal: &mut bool,
        desired: bool,
        cutoff: u64,
    ) -> Result<WirelessFact, Error> {
        set_parts(fake, &expected(), current, terminal, desired, cutoff)
    }
    fn reads() -> Vec<Step> {
        vec![Step::Radio, Step::Profile, Step::Radio, Step::Check]
    }

    #[test]
    fn connect_then_disconnect_have_exact_order_and_one_cutoff() {
        for (current, desired, changes) in [
            (false, true, [Step::Associate(true), Step::Configure(true)]),
            (
                true,
                false,
                [Step::Configure(false), Step::Associate(false)],
            ),
        ] {
            let mut fake = Fake::new(current);
            let cutoff = until();
            let mut connected = current;
            let mut terminal = false;
            let fact = run(&mut fake, &mut connected, &mut terminal, desired, cutoff).unwrap();
            let mut steps = reads();
            steps.extend(changes);
            steps.extend(reads());
            assert_eq!(
                fake.calls,
                steps.into_iter().map(|s| (s, cutoff)).collect::<Vec<_>>()
            );
            assert_eq!(fact.connected, desired);
            assert_eq!(connected, desired);
            assert!(!terminal);
            assert_eq!(fact.namespace, expected().target.namespace);
            assert_eq!(fact.ifindex, 3);
            assert_eq!(fact.observed_lifetime, None);
        }
    }

    #[test]
    fn same_state_only_observes_and_uses_oldest_real_sample() {
        for desired in [false, true] {
            for (radio_times, profile_time) in
                [([100, 300], 200), ([300, 100], 200), ([200, 300], 100)]
            {
                let mut fake = Fake::new(desired);
                fake.radio_times = radio_times.into();
                fake.profile_times = [profile_time].into();
                let cutoff = until();
                let mut connected = desired;
                let mut terminal = false;
                let fact = run(&mut fake, &mut connected, &mut terminal, desired, cutoff).unwrap();
                assert_eq!(fact.observed_at_ns, 100);
                assert_eq!(
                    fake.calls,
                    reads().into_iter().map(|s| (s, cutoff)).collect::<Vec<_>>()
                );
                assert!(!terminal);
            }
        }
    }

    #[test]
    fn intermediate_confirmation_must_succeed_before_the_next_component() {
        for current in [false, true] {
            for contradict_at in [5, 6] {
                let mut fake = Fake::new(current);
                fake.contradict_at = Some(contradict_at);
                let mut connected = current;
                let mut terminal = false;
                assert_eq!(
                    run(&mut fake, &mut connected, &mut terminal, !current, until()),
                    Err(Error::Uncertain)
                );
                assert!(terminal);
                assert_eq!(fake.calls.len(), contradict_at);
                assert_eq!(connected, current);
                if contradict_at == 5 {
                    // Exactly the first component changed. No next mutation,
                    // compensation or implicit rollback follows bad readback.
                    assert!(fake.associated);
                    assert!(!fake.configured);
                }
            }
        }
    }

    #[test]
    fn radio_observation_loss_retires_even_with_ipv4_still_present() {
        let mut fake = Fake::new(true);
        fake.fail_at = Some((1, Error::Observation(ObserveError::EvidenceLost)));
        let mut terminal = false;
        assert_eq!(
            refresh_parts(&mut fake, &expected(), true, &mut terminal, until()),
            Err(Error::Observation(ObserveError::EvidenceLost))
        );
        assert!(terminal);
        assert!(fake.configured);
        assert!(fake.associated);
        let calls = fake.calls.clone();
        let mut connected = true;
        assert_eq!(
            run(&mut fake, &mut connected, &mut terminal, false, until()),
            Err(Error::Uncertain)
        );
        assert_eq!(fake.calls, calls);
    }

    #[test]
    fn first_changing_components_unsent_expiry_keeps_its_classification() {
        for current in [false, true] {
            for error in [
                Error::Deadline,
                Error::Conflict,
                Error::Io(5),
                Error::Uncertain,
            ] {
                let mut fake = Fake::new(current);
                fake.fail_at = Some((5, error));
                let mut connected = current;
                let mut terminal = false;
                assert_eq!(
                    run(&mut fake, &mut connected, &mut terminal, !current, until()),
                    Err(error)
                );
                assert!(terminal);
                assert_eq!(connected, current);
                assert_eq!(fake.calls.len(), 5);
            }
        }
    }

    #[test]
    fn every_later_failure_is_uncertain_and_fences_further_calls() {
        for current in [false, true] {
            // The second mutation, either final radio read, the profile read,
            // and the final deadline all occur after a component changed.
            for failed_call in 6..=10 {
                for error in [
                    Error::Deadline,
                    Error::Conflict,
                    Error::Io(5),
                    Error::Uncertain,
                ] {
                    let mut fake = Fake::new(current);
                    fake.fail_at = Some((failed_call, error));
                    let mut connected = current;
                    let mut terminal = false;
                    assert_eq!(
                        run(&mut fake, &mut connected, &mut terminal, !current, until()),
                        Err(Error::Uncertain)
                    );
                    assert!(terminal);
                    assert_eq!(connected, current);
                    let calls = fake.calls.clone();
                    assert_eq!(
                        run(&mut fake, &mut connected, &mut terminal, current, 0),
                        Err(Error::Uncertain)
                    );
                    assert_eq!(
                        refresh_parts(&mut fake, &expected(), connected, &mut terminal, 0),
                        Err(Error::Uncertain)
                    );
                    assert_eq!(fake.calls, calls);
                }
            }
        }
    }

    #[test]
    fn prior_read_failure_retires_without_any_mutation() {
        for failed_call in 1..=4 {
            let mut fake = Fake::new(false);
            fake.fail_at = Some((failed_call, Error::Deadline));
            let mut connected = false;
            let mut terminal = false;
            assert_eq!(
                run(&mut fake, &mut connected, &mut terminal, true, until()),
                Err(Error::Deadline)
            );
            assert!(terminal);
            assert!(!fake.associated);
            assert!(!fake.configured);
            assert_eq!(fake.calls.len(), failed_call);
        }
    }

    #[test]
    fn invalid_original_cutoff_never_enters_a_component() {
        for cutoff in [0, pokelinux_network_state::boot_time_ns() + 2_000_000_000] {
            let mut fake = Fake::new(false);
            let mut connected = false;
            let mut terminal = false;
            assert_eq!(
                run(&mut fake, &mut connected, &mut terminal, true, cutoff),
                Err(Error::Deadline)
            );
            assert!(fake.calls.is_empty());
            assert!(!terminal);
        }
    }

    #[test]
    fn bracketed_observation_rejects_disagreement_and_identity_changes() {
        let mut fake = Fake::new(false);
        let first = fake.radio_fact();
        let profile = fake.profile_fact();
        let last = fake.radio_fact();
        let selected = expected();
        assert_eq!(
            combine(&selected, false, first, &profile, last)
                .unwrap()
                .observed_at_ns,
            100
        );
        for changed in [
            RadioFact {
                associated: true,
                ..last
            },
            RadioFact {
                wiphy: last.wiphy + 1,
                ..last
            },
            RadioFact {
                wdev: last.wdev + 1,
                ..last
            },
        ] {
            assert_eq!(
                combine(&selected, false, first, &profile, changed),
                Err(Error::Conflict)
            );
            assert_eq!(
                combine(&selected, false, changed, &profile, last),
                Err(Error::Conflict)
            );
        }
        let mut changes = vec![];
        let mut changed = profile.clone();
        changed.configured = true;
        changes.push(changed);
        let mut changed = profile.clone();
        changed.target.namespace.ino += 1;
        changes.push(changed);
        let mut changed = profile.clone();
        changed.target.ifindex += 1;
        changes.push(changed);
        let mut changed = profile.clone();
        changed.target.name = "other0".into();
        changes.push(changed);
        let mut changed = profile.clone();
        changed.observed_lifetime = Some(19);
        changes.push(changed);
        for changed in changes {
            assert_eq!(
                combine(&selected, false, first, &changed, last),
                Err(Error::Conflict)
            );
        }
    }
}
