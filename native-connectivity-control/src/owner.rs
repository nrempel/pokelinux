//! Pure fixed-profile operation ownership; no socket, kernel I/O or app authentication.
//! All `now`/`original_until_ns` values are CLOCK_BOOTTIME nanoseconds supplied by
//! the trusted service. Authority is never deserialized from an application.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Connect,
    Disconnect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Principal {
    pub instance: u64,
    pub generation: u64,
}

/// Current in-process lifecycle decision, including its grant generation.
/// These fields are not an application request or a self-asserted capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Authority {
    pub principal: Principal,
    pub permission_epoch: u64,
    pub allowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Id(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Command {
    pub id: Id,
    pub action: Action,
    pub original_until_ns: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Staged,
    Dispatched,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pending {
    pub command: Command,
    pub phase: Phase,
}

/// A readback of this fixed profile's local configuration, not Internet access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Observation {
    pub connected: bool,
    pub observed_at_ns: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Finish {
    /// Concrete backend proof that no mutating operation was sent.
    DefinitelyUnsent,
    /// Actual fixed-profile readback; not an optimistic requested value.
    Observed(bool),
    /// A possibly sent operation cannot be attributed a definitive result.
    Uncertain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reply {
    pub id: Id,
    pub outcome: Finish,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reject {
    Denied,
    Deadline,
    Busy,
    Uncertain,
    Stale,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct View {
    pub observed: Option<Observation>,
    pub pending: Option<Pending>,
    pub terminal_uncertain: bool,
}

#[derive(Clone, Copy)]
struct Operation {
    pending: Pending,
    authority: Authority,
    requester_attached: bool,
}

#[derive(Clone, Copy)]
struct PendingReply {
    value: Reply,
    authority: Authority,
    original_until_ns: u64,
}

/// One fixed-profile mutation slot, independent of the requester's lifetime.
/// Only the concrete background backend can retire a dispatched operation.
#[derive(Default)]
pub struct Owner {
    sequence: u64,
    active: Option<Operation>,
    reply: Option<PendingReply>,
    observed: Option<Observation>,
    terminal_uncertain: bool,
}

fn authorized(authority: Authority) -> bool {
    authority.allowed
        && authority.principal.instance > 0
        && authority.principal.generation > 0
        && authority.permission_epoch > 0
}

impl Owner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stage(
        &mut self,
        authority: Authority,
        action: Action,
        original_until_ns: u64,
        now: u64,
    ) -> Result<Id, Reject> {
        self.tick(now);
        if !authorized(authority) {
            return Err(Reject::Denied);
        }
        // Subtraction follows ordering so an overflowing absolute request is
        // refused without minting or renewing a deadline.
        if now >= original_until_ns || original_until_ns - now > 1_000_000_000 {
            return Err(Reject::Deadline);
        }
        if self.terminal_uncertain {
            return Err(Reject::Uncertain);
        }
        if self.active.is_some() {
            return Err(Reject::Busy);
        }
        let next = self.sequence.checked_add(1).ok_or(Reject::Unavailable)?;
        let id = Id(next);
        self.sequence = next;
        self.reply = None;
        self.active = Some(Operation {
            pending: Pending {
                command: Command {
                    id,
                    action,
                    original_until_ns,
                },
                phase: Phase::Staged,
            },
            authority,
            requester_attached: true,
        });
        Ok(id)
    }

    /// Background ownership/admission boundary, contiguous with the caller's
    /// begin-send. Returning Command is not proof of an actual kernel send.
    pub fn dispatch(&mut self, id: Id, current: Authority, now: u64) -> Result<Command, Reject> {
        self.tick(now);
        let Some(operation) = self.active.as_mut().filter(|operation| {
            operation.pending.command.id == id && operation.pending.phase == Phase::Staged
        }) else {
            return Err(Reject::Stale);
        };
        if !authorized(current) || current != operation.authority {
            // No command was returned, so this reservation is definitely unsent.
            self.active = None;
            return Err(Reject::Denied);
        }
        operation.pending.phase = Phase::Dispatched;
        Ok(operation.pending.command)
    }

    /// App departure or permission revocation; no implicit Disconnect.
    pub fn withdraw(&mut self, principal: Principal, now: u64) {
        self.tick(now);
        if self
            .reply
            .is_some_and(|reply| reply.authority.principal == principal)
        {
            self.reply = None;
        }
        if let Some(operation) = self.active.as_mut()
            && operation.authority.principal == principal
        {
            if operation.pending.phase == Phase::Staged {
                self.active = None;
            } else {
                operation.requester_attached = false;
            }
        }
    }

    /// Call only when this command's concrete backend custody is retired.
    pub fn finish(&mut self, id: Id, now: u64, outcome: Finish) -> bool {
        self.tick(now);
        let Some(operation) = self.active.filter(|operation| {
            operation.pending.command.id == id && operation.pending.phase == Phase::Dispatched
        }) else {
            return false;
        };
        self.active = None;
        match outcome {
            // Staging/dispatch never optimistically change the observed fact.
            // Do not overwrite a newer independent observation with old history.
            Finish::DefinitelyUnsent => {}
            Finish::Observed(connected) => {
                self.observed = Some(Observation {
                    connected,
                    observed_at_ns: now,
                });
            }
            Finish::Uncertain => {
                self.observed = None;
                self.terminal_uncertain = true;
            }
        }
        if operation.requester_attached {
            self.reply = Some(PendingReply {
                value: Reply { id, outcome },
                authority: operation.authority,
                original_until_ns: operation.pending.command.original_until_ns,
            });
        }
        true
    }

    /// Independent read-only fact, never an acknowledgement/replay of old intent.
    pub fn observe(&mut self, connected: bool, now: u64) {
        self.tick(now);
        self.observed = Some(Observation {
            connected,
            observed_at_ns: now,
        });
    }

    pub fn tick(&mut self, now: u64) {
        if self
            .reply
            .is_some_and(|reply| now >= reply.original_until_ns)
        {
            self.reply = None;
        }
        if let Some(operation) = self.active.as_mut()
            && now >= operation.pending.command.original_until_ns
        {
            if operation.pending.phase == Phase::Staged {
                self.active = None;
            } else {
                // Timeout fences the requester, not the service's actual custody.
                // Only finish can declare definitely-unsent/observed/uncertain.
                operation.requester_attached = false;
            }
        }
    }

    pub fn view(&mut self, now: u64) -> View {
        self.tick(now);
        View {
            observed: self.observed,
            pending: self.active.map(|operation| operation.pending),
            terminal_uncertain: self.terminal_uncertain,
        }
    }

    pub fn take_reply(&mut self, id: Id, current: Authority, now: u64) -> Option<Reply> {
        self.tick(now);
        let reply = self.reply.filter(|reply| reply.value.id == id)?;
        if !authorized(current) || current != reply.authority {
            self.reply = None;
            return None;
        }
        self.reply = None;
        Some(reply.value)
    }
}
