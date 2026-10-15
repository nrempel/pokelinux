//! Single-attempt provider/kernel observations. No activation or APN authority.
mod bounded;
mod callbacks;
use callbacks::{Death, Event, Indication, Response, Shared};
use pokelinux_cellular_boundary::generated::android::hardware::radio::{
    RadioIndicationType::RadioIndicationType,
    data::{DataCallFailCause::DataCallFailCause, PdpProtocolType::PdpProtocolType},
};
use pokelinux_cellular_boundary::{
    BnRadioDataIndication, BnRadioDataResponse, IRadioData, IRadioDataIndication,
    IRadioDataResponse, SetupDataCallResult,
};
use pokelinux_cellular_boundary::{RadioError, RadioResponseType};
use pokelinux_network_state::{KernelObserver, KernelSnapshot, NamespaceIdentity, ObserveError};
use rsbinder::{SIBinder, Strong};
use std::net::IpAddr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    Deadline,
    CallbackFault,
    Stopped,
    AlreadySubmitted,
    Binder,
    Kernel(ObserveError),
}
impl From<ObserveError> for Error {
    fn from(v: ObserveError) -> Self {
        Self::Kernel(v)
    }
}
type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    NotSubmitted,
    OutcomeUnknown,
    ProviderAccepted { cid: i32 },
    ProviderRejected { radio_error: i32, cause: i32 },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Association {
    pub namespace: NamespaceIdentity,
    pub ifindex: u32,
    pub lifetime: u64,
    pub revision: u64,
    pub observed_at_boottime_ns: u64,
}
/// One retained endpoint, fresh callback pair and one continuously retained
/// observer. Component data only: discovery, dispatch and permissions are external.
pub struct DataOwner {
    endpoint: Strong<dyn IRadioData>,
    binder: SIBinder,
    death: Arc<Death>,
    response: Strong<dyn IRadioDataResponse>,
    indication: Strong<dyn IRadioDataIndication>,
    shared: Arc<Shared>,
    received: Receiver<Event>,
    observer: KernelObserver,
    expected_uid: u32,
    serial: i32,
    until: u64,
    disposition: Disposition,
    report: Option<SetupDataCallResult>,
    association: Option<Association>,
    stopped: bool,
    provider_live: bool,
    retired: bool,
    terminal: Option<Error>,
}
impl DataOwner {
    pub fn new(
        endpoint: Strong<dyn IRadioData>,
        expected_uid: u32,
        serial: i32,
        until: u64,
    ) -> Result<Self> {
        let observer = KernelObserver::subscribe_current_namespace(until)?;
        let (queue, received) = mpsc::sync_channel(8);
        let shared = Arc::new(Shared {
            queue,
            fault: AtomicBool::new(false),
            closed: AtomicBool::new(false),
        });
        let binder = endpoint.as_binder();
        if !binder.is_remote() {
            return Err(Error::Binder);
        }
        let death = Arc::new(Death(shared.clone()));
        binder
            .link_to_death_arc(&death)
            .map_err(|_| Error::Binder)?;
        Ok(Self {
            endpoint,
            binder,
            death,
            response: BnRadioDataResponse::new_binder(Response(shared.clone())),
            indication: BnRadioDataIndication::new_binder(Indication(shared.clone())),
            shared,
            received,
            observer,
            expected_uid,
            serial,
            until,
            disposition: Disposition::NotSubmitted,
            report: None,
            association: None,
            stopped: false,
            provider_live: true,
            retired: false,
            terminal: None,
        })
    }
    pub fn callbacks(
        &self,
    ) -> (
        &Strong<dyn IRadioDataResponse>,
        &Strong<dyn IRadioDataIndication>,
    ) {
        (&self.response, &self.indication)
    }
    pub fn endpoint(&self) -> &Strong<dyn IRadioData> {
        &self.endpoint
    }
    pub fn disposition(&self) -> Disposition {
        self.disposition
    }
    pub fn provider_report(&self) -> Option<&SetupDataCallResult> {
        self.report.as_ref()
    }
    /// Cached observation, never a current permission or APN-ready grant.
    pub fn last_association(&self) -> Option<&Association> {
        self.association.as_ref()
    }
    pub fn mark_may_have_submitted(&mut self) -> Result<i32> {
        self.check()?;
        if self.disposition != Disposition::NotSubmitted {
            return Err(Error::AlreadySubmitted);
        }
        self.disposition = Disposition::OutcomeUnknown;
        Ok(self.serial)
    }
    fn retire(&mut self) {
        self.association = None;
        self.retired = true;
    }
    fn fail(&mut self, error: Error) -> Error {
        self.retire();
        self.terminal = Some(error);
        error
    }
    fn check(&mut self) -> Result<()> {
        let error = if self.stopped {
            Some(Error::Stopped)
        } else if let Some(error) = self.terminal {
            Some(error)
        } else if boot_ns() >= self.until {
            Some(Error::Deadline)
        } else if self.shared.fault.load(Ordering::SeqCst) {
            Some(Error::CallbackFault)
        } else {
            None
        };
        match error {
            Some(error) => Err(self.fail(error)),
            None => Ok(()),
        }
    }
    fn reduce(&mut self, event: Event) -> Result<bool> {
        self.check()?;
        match event {
            Event::Setup(packet, uid) => {
                let (info, report) = *packet;
                if uid != self.expected_uid
                    || info.r#type != RadioResponseType::SOLICITED
                    || !bounded::bounded_result(&report)
                {
                    return Err(self.fail(Error::CallbackFault));
                }
                // The fresh callback pair belongs to this retained endpoint.
                // A mismatched/duplicate/retired result never settles history.
                if info.serial != self.serial
                    || self.disposition != Disposition::OutcomeUnknown
                    || !self.provider_live
                    || self.retired
                {
                    return Ok(false);
                }
                self.disposition =
                    if info.error == RadioError::NONE && report.cause == DataCallFailCause::NONE {
                        Disposition::ProviderAccepted { cid: report.cid }
                    } else {
                        Disposition::ProviderRejected {
                            radio_error: info.error.0,
                            cause: report.cause.0,
                        }
                    };
                self.report = Some(report);
                Ok(true)
            }
            Event::List(kind, _calls, uid) => {
                if uid != self.expected_uid || kind != RadioIndicationType::UNSOLICITED {
                    return Err(self.fail(Error::CallbackFault));
                }
                // First slice conservatively retires on every list update.
                // A list can withdraw, but cannot establish a new association.
                self.retire();
                Ok(true)
            }
            Event::Died => {
                self.provider_live = false;
                self.retire();
                Ok(true)
            }
        }
    }
    fn drain(&mut self) -> Result<()> {
        // A producer cannot hold this turn open by continually refilling it.
        for _ in 0..16 {
            match self.received.try_recv() {
                Ok(event) => {
                    self.reduce(event)?;
                }
                Err(TryRecvError::Empty) => return self.check(),
                Err(TryRecvError::Disconnected) => return Err(self.fail(Error::CallbackFault)),
            }
        }
        Err(self.fail(Error::CallbackFault))
    }
    /// Fresh observational join for this owner's retained endpoint and observer.
    /// It does not establish that the provider owns the kernel link, or grant
    /// APN, route, carrier, internet, dispatch or app permission authority.
    pub fn poll(&mut self) -> Result<Option<&Association>> {
        self.check()?;
        self.drain()?;
        if self.retired
            || !self.provider_live
            || !matches!(self.disposition, Disposition::ProviderAccepted { .. })
        {
            return Ok(None);
        }
        let snapshot = match self.observer.refresh(self.until) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                // Before the first join, concurrent notifications may be drained
                // and retried within the same caller deadline. Once bound, any
                // refused candidate retires the current association permanently.
                if self.association.is_some() {
                    self.retire();
                }
                let error = Error::Kernel(error);
                return Err(if error == Error::Kernel(ObserveError::ConcurrentChange) {
                    error
                } else {
                    self.fail(error)
                });
            }
        };
        let candidate = self
            .report
            .as_ref()
            .and_then(|report| join_candidate(report, &snapshot));
        // Process known provider invalidations after the kernel read as well.
        // This is not an atomic cross-source snapshot; later changes can occur.
        self.drain()?;
        self.check()?;
        if self.retired || !self.provider_live {
            return Ok(None);
        }
        if let Some(previous) = &self.association {
            let same = candidate.as_ref().is_some_and(|next| {
                next.namespace == previous.namespace
                    && next.ifindex == previous.ifindex
                    && next.lifetime == previous.lifetime
            });
            if !same {
                self.retire();
                return Ok(None);
            }
        }
        self.association = candidate;
        Ok(self.association.as_ref())
    }
    pub fn stop(&mut self) {
        self.stopped = true;
        self.retire();
        self.shared.closed.store(true, Ordering::SeqCst);
    }
}
impl Drop for DataOwner {
    fn drop(&mut self) {
        self.stop();
        let _ = self.binder.unlink_to_death_arc(&self.death);
    }
}

// This first join accepts a deliberately narrow structural subset. Provider
// fields remain historical reports; DNS/P-CSCF/gateway values are not kernel
// facts. Finite Android elapsedRealtime lifetimes are not converted into native
// BOOTTIME nanoseconds here; only the explicit infinite sentinel is eligible.
fn provider_addresses(report: &SetupDataCallResult) -> Option<Vec<(IpAddr, u8)>> {
    if report.ifname.is_empty()
        || report.ifname.len() > 15
        || !report
            .ifname
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
        || report.cid < 0
        || report.active != 2
        || report.addresses.is_empty()
        || !bounded::bounded_result(report)
    {
        return None;
    }
    let mut addresses = Vec::new();
    for address in &report.addresses {
        if address.addressProperties != 0
            || address.deprecationTime != i64::MAX
            || address.expirationTime != i64::MAX
        {
            return None;
        }
        let (ip, prefix) = address.address.split_once('/')?;
        if prefix.is_empty() || prefix.len() > 3 || !prefix.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let ip: IpAddr = ip.parse().ok()?;
        let prefix: u8 = prefix.parse().ok()?;
        if prefix == 0
            || prefix > if ip.is_ipv4() { 32 } else { 128 }
            || ip.is_multicast()
            || ip.is_unspecified()
        {
            return None;
        }
        if addresses.contains(&(ip, prefix)) {
            return None;
        }
        addresses.push((ip, prefix));
    }
    let v4 = addresses.iter().any(|(ip, _)| ip.is_ipv4());
    let v6 = addresses.iter().any(|(ip, _)| ip.is_ipv6());
    let protocol_matches = match report.r#type {
        PdpProtocolType::IP => v4 && !v6,
        PdpProtocolType::IPV6 => v6 && !v4,
        PdpProtocolType::IPV4V6 => v4 && v6,
        _ => false,
    };
    protocol_matches.then_some(addresses)
}
fn join_candidate(report: &SetupDataCallResult, snapshot: &KernelSnapshot) -> Option<Association> {
    let addresses = provider_addresses(report)?;
    let mut matching = snapshot
        .links
        .iter()
        .filter(|link| link.name == report.ifname);
    let link = matching.next()?;
    if matching.next().is_some() || link.flags & 1 == 0 || link.mtu == 0 {
        return None;
    }
    let lifetime = link.observed_lifetime.filter(|v| *v > 0)?;
    for (ip, prefix) in addresses {
        let mtu = if ip.is_ipv4() {
            report.mtuV4
        } else {
            report.mtuV6
        };
        if u32::try_from(mtu).ok()? != link.mtu {
            return None;
        }
        // Compare numeric LOCAL addresses. A point-to-point peer is separate;
        // provider gateways do not prove the peer. Extra kernel addresses do not
        // become provider reports and do not supply missing reported addresses.
        if link
            .addresses
            .iter()
            .filter(|a| {
                a.local == ip
                    && a.prefix_len == prefix
                    && a.scope == 0
                    && a.flags & (0x08 | 0x20 | 0x40) == 0
            })
            .count()
            != 1
        {
            return None;
        }
    }
    Some(Association {
        namespace: snapshot.namespace,
        ifindex: link.ifindex,
        lifetime,
        revision: snapshot.revision,
        observed_at_boottime_ns: snapshot.observed_at_boottime_ns,
    })
}

fn boot_ns() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    u64::try_from(t.tv_sec)
        .unwrap()
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(u64::try_from(t.tv_nsec).unwrap())
        .unwrap()
}
#[cfg(test)]
mod fixture;
#[cfg(test)]
mod kernel_tests;
