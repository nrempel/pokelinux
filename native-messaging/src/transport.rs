//! Device-neutral qualification of asynchronous submission results. The adapter
//! owns wire serials/methods/slots; this owner retains durable dispatch permits.
use crate::outbox::{DispatchPermit, Error, Outbox, Receipt, Result, TransportResult};
use std::collections::HashMap;

pub const MAX_ENDPOINTS: usize = 8;
pub const MAX_PENDING: usize = 32;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EndpointId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Source {
    owner: uuid::Uuid,
    endpoint: EndpointId,
    generation: uuid::Uuid,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AttemptId(uuid::Uuid);
pub struct Request {
    pub source: Source,
    pub attempt: AttemptId,
    tpdu: Vec<u8>,
}
impl Request {
    pub fn tpdu(&self) -> &[u8] {
        &self.tpdu
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Local IPC submission only, not a qualified remote send result.
    IpcReturned,
    /// Intermediate receipt acknowledgement only.
    RequestAcknowledged,
    /// Adapter-qualified final result for this exact operation/source.
    SendAccepted,
    /// A terminal outcome without proof that submission did not occur.
    Uncertain,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ignored,
    Pending,
    Accepted,
    Uncertain,
}
struct Pending {
    source: Source,
    message: String,
    permit: DispatchPermit,
}
pub struct SubmissionOwner {
    outbox: Outbox,
    owner: uuid::Uuid,
    sources: HashMap<EndpointId, Source>,
    pending: HashMap<AttemptId, Pending>,
}
impl SubmissionOwner {
    pub fn new(outbox: Outbox) -> Self {
        Self {
            outbox,
            owner: uuid::Uuid::new_v4(),
            sources: HashMap::new(),
            pending: HashMap::new(),
        }
    }
    pub fn prepare(
        &mut self,
        id: &str,
        recipient: &str,
        text: &str,
        report: bool,
    ) -> Result<Receipt> {
        self.outbox.prepare(id, recipient, text, report)
    }
    pub fn receipt(&self, id: &str) -> Result<Receipt> {
        self.outbox.lookup(id)
    }
    /// Local source replacement revokes old permits. It supplies no remote reset proof.
    pub fn attach(&mut self, endpoint: EndpointId) -> Result<Source> {
        if !self.sources.contains_key(&endpoint) && self.sources.len() >= MAX_ENDPOINTS {
            return Err(Error::Refused("endpoint capacity exhausted"));
        }
        if let Some(old) = self.sources.remove(&endpoint) {
            self.pending.retain(|_, p| p.source != old);
        }
        let source = Source {
            owner: self.owner,
            endpoint,
            generation: uuid::Uuid::new_v4(),
        };
        self.sources.insert(endpoint, source);
        Ok(source)
    }
    fn active(&self, source: Source) -> bool {
        source.owner == self.owner && self.sources.get(&source.endpoint) == Some(&source)
    }
    pub fn revoke(&mut self, source: Source) {
        if self.active(source) {
            self.sources.remove(&source.endpoint);
        }
        self.pending.retain(|_, p| p.source != source);
    }
    pub fn begin(&mut self, source: Source, message: &str, part: usize) -> Result<Request> {
        if !self.active(source) {
            return Err(Error::Refused("inactive submission source"));
        }
        if self.pending.len() >= MAX_PENDING {
            return Err(Error::Refused("pending capacity exhausted"));
        }
        let permit = self.outbox.claim(message, part)?;
        let attempt = AttemptId(uuid::Uuid::new_v4());
        let request = Request {
            source,
            attempt,
            tpdu: permit.tpdu().to_vec(),
        };
        self.pending.insert(
            attempt,
            Pending {
                source,
                message: message.into(),
                permit,
            },
        );
        Ok(request)
    }
    /// Caller serializes this owner with cancellation/revocation. A final result
    /// must already have passed the backend's method/serial/metadata checks.
    pub fn observe(
        &mut self,
        source: Source,
        attempt: AttemptId,
        signal: Signal,
    ) -> Result<Outcome> {
        if !self.active(source)
            || self
                .pending
                .get(&attempt)
                .is_none_or(|p| p.source != source)
        {
            return Ok(Outcome::Ignored);
        }
        match signal {
            Signal::IpcReturned | Signal::RequestAcknowledged => Ok(Outcome::Pending),
            Signal::Uncertain => {
                self.pending.remove(&attempt);
                Ok(Outcome::Uncertain)
            }
            Signal::SendAccepted => {
                let pending = self
                    .pending
                    .remove(&attempt)
                    .ok_or(Error::Refused("missing pending permit"))?;
                self.outbox
                    .finish(pending.permit, TransportResult::Accepted)?;
                Ok(Outcome::Accepted)
            }
        }
    }
    pub fn cancel(&mut self, message: &str) -> Result<()> {
        self.outbox.cancel(message)?;
        self.pending.retain(|_, p| p.message != message);
        Ok(())
    }
}
