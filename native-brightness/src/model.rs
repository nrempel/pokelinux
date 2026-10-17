//! Local observation/intent policy. Retirement is supplied only after actual
//! worker wait and pipe EOF; this module itself proves neither process custody
//! nor physical luminance. Controlled clocks test policy, not kernel suspend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Observation {
    pub max: u32,
    pub requested: u32,
    pub actual: Option<u32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Observe,
    Set { percent: u8 },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ticket {
    pub id: u64,
    pub action: Action,
    pub until_ns: u64,
    pub publication_until_ns: u64,
    pub baseline: Option<Observation>,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct View {
    pub revision: u64,
    pub observation: Option<Observation>,
    pub until_ns: u64,
    /// Actual occupied slot, retained after local cancellation/deadline.
    pub pending: bool,
    /// Current admitted intent only, never proof that the property changed.
    pub requested_percent: Option<u8>,
    /// Fresh readonly reconciliation never acknowledges an earlier uncertain Set.
    pub last_set_uncertain: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejected {
    Busy,
    Unavailable,
    StaleRevision,
    Invalid,
    Expired,
}
const SECOND: u64 = 1_000_000_000;
impl Observation {
    pub fn valid(self) -> bool {
        self.max > 0 && self.requested <= self.max && self.actual.is_none_or(|v| v <= self.max)
    }
}
pub(crate) fn scaled(max: u32, percent: u8) -> u32 {
    ((u64::from(max) * u64::from(percent) + 50) / 100) as u32
}
#[derive(Clone, Copy)]
struct Flight {
    ticket: Ticket,
    admitted: bool,
}
#[derive(Default)]
pub struct Owner {
    view: View,
    flight: Option<Flight>,
    next_id: u64,
    exhausted: bool,
}
impl Owner {
    pub fn new() -> Self {
        Self::default()
    }
    fn advance(&mut self) {
        match self.view.revision.checked_add(1) {
            Some(next) => self.view.revision = next,
            None => {
                self.exhausted = true;
                self.view.observation = None;
                self.view.until_ns = 0;
            }
        }
    }
    fn withdraw(&mut self) {
        self.view.observation = None;
        self.view.until_ns = 0;
        self.view.requested_percent = None;
    }
    fn refresh(&mut self, now: u64) {
        if let Some(flight) = self.flight.as_mut()
            && flight.admitted
            && now >= flight.ticket.until_ns
        {
            flight.admitted = false;
            if matches!(flight.ticket.action, Action::Set { .. }) {
                self.view.last_set_uncertain = true;
            }
            self.withdraw();
            self.advance();
        }
        if self.view.observation.is_some() && now >= self.view.until_ns {
            self.withdraw();
            self.advance();
        }
    }
    fn ticket(
        &mut self,
        action: Action,
        until_ns: u64,
        publication_until_ns: u64,
        baseline: Option<Observation>,
    ) -> Option<Ticket> {
        if self.exhausted {
            return None;
        }
        let id = self.next_id.checked_add(1)?;
        self.next_id = id;
        Some(Ticket {
            id,
            action,
            until_ns,
            publication_until_ns,
            baseline,
        })
    }
    pub fn view(&mut self, now: u64) -> View {
        self.refresh(now);
        self.view
    }
    pub fn observe(&mut self, now: u64) -> Option<Ticket> {
        self.refresh(now);
        if self.flight.is_some() {
            return None;
        }
        let until = now.checked_add(SECOND)?;
        let publication_until = now.checked_add(2 * SECOND)?;
        let ticket = self.ticket(Action::Observe, until, publication_until, None)?;
        self.flight = Some(Flight {
            ticket,
            admitted: true,
        });
        self.view.pending = true;
        self.advance();
        Some(ticket)
    }
    pub fn set(
        &mut self,
        percent: u8,
        expected_revision: u64,
        until_ns: u64,
        now: u64,
    ) -> Result<Ticket, Rejected> {
        self.refresh(now);
        if self.flight.is_some() {
            return Err(Rejected::Busy);
        }
        if percent > 100 {
            return Err(Rejected::Invalid);
        }
        if until_ns <= now {
            return Err(Rejected::Expired);
        }
        if until_ns - now > SECOND {
            return Err(Rejected::Invalid);
        }
        if self.exhausted {
            return Err(Rejected::Unavailable);
        }
        let baseline = self.view.observation.ok_or(Rejected::Unavailable)?;
        if expected_revision == 0 || expected_revision != self.view.revision {
            return Err(Rejected::StaleRevision);
        }
        let publication_until = now.checked_add(2 * SECOND).ok_or(Rejected::Invalid)?;
        let ticket = self
            .ticket(
                Action::Set { percent },
                until_ns,
                publication_until,
                Some(baseline),
            )
            .ok_or(Rejected::Unavailable)?;
        self.withdraw();
        self.view.pending = true;
        self.view.requested_percent = Some(percent);
        self.flight = Some(Flight {
            ticket,
            admitted: true,
        });
        self.advance();
        Ok(ticket)
    }
    pub fn cancel(&mut self, now: u64) -> Option<Ticket> {
        self.refresh(now);
        let ticket = self.flight.as_ref().map(|v| v.ticket);
        let admitted = self.flight.as_ref().is_some_and(|v| v.admitted);
        if let Some(flight) = self.flight.as_mut() {
            flight.admitted = false;
        }
        if let Some(ticket) = ticket
            && matches!(ticket.action, Action::Set { .. })
        {
            self.view.last_set_uncertain = true;
        }
        if admitted || self.view.observation.is_some() {
            self.withdraw();
            self.advance();
        }
        ticket
    }
    /// Caller retains the real worker until wait+EOF. A stale ticket can neither
    /// free another occupied slot nor publish a late successful result.
    pub fn retired(&mut self, ticket: Ticket, value: Option<Observation>, now: u64) -> bool {
        self.refresh(now);
        let Some(flight) = self.flight.filter(|v| v.ticket == ticket) else {
            return false;
        };
        self.flight = None;
        self.view.pending = false;
        self.view.requested_percent = None;
        let value = value.filter(|v| v.valid());
        let confirmed = flight.admitted
            && now < ticket.until_ns
            && now < ticket.publication_until_ns
            && value.is_some_and(|v| match ticket.action {
                Action::Observe => true,
                Action::Set { percent } => ticket
                    .baseline
                    .is_some_and(|b| b.max == v.max && v.requested == scaled(b.max, percent)),
            });
        self.withdraw();
        if confirmed {
            self.view.observation = value;
            self.view.until_ns = ticket.publication_until_ns;
            if matches!(ticket.action, Action::Set { .. }) {
                self.view.last_set_uncertain = false;
            }
        } else if matches!(ticket.action, Action::Set { .. }) {
            self.view.last_set_uncertain = true;
        }
        self.advance();
        confirmed && !self.exhausted
    }
}
