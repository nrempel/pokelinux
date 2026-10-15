//! Private fixture support for the actual retained Messages provider.
//! No authentication producer or lifecycle authority crosses this facade.
use pokelinux_lifecycle::{
    boottime_ns,
    messages::{Event, Provider},
};
use pokelinux_messages::worker::Projector;
use std::{io, path::Path};

const DROP_DRAIN_NS: u64 = 2_000_000_000;

// No Debug/Clone: complete packets may contain private retained history.
pub enum ReadEvent {
    Complete { token: u64, packet: Vec<u8> },
    Failed { token: u64 },
}

/// Owns the real writer/readonly children through normal bounded drain.
/// Its inherent API keeps lifecycle crate identities out of test adapters.
pub struct History {
    projector: Projector,
}

impl History {
    pub fn spawn(runtime: &Path, raw: &Path, inbox: &Path, exe: &Path) -> io::Result<Self> {
        Ok(Self {
            projector: Projector::spawn(runtime, raw, inbox, exe)?,
        })
    }

    pub fn incarnation(&self) -> &str {
        Provider::incarnation(&self.projector)
    }

    pub fn occupied(&self) -> bool {
        Provider::occupied(&self.projector)
    }

    pub fn read_occupied(&self) -> bool {
        Provider::read_occupied(&self.projector)
    }

    pub fn busy(&self) -> bool {
        Provider::busy(&self.projector)
    }

    pub fn available(&self) -> bool {
        Provider::available(&self.projector)
    }

    pub fn incoming_progress(&self) -> (u64, u64, u64) {
        Provider::incoming_progress(&self.projector)
    }

    pub fn receiving(&self) -> bool {
        Provider::receiving(&self.projector)
    }

    pub fn incoming_failure(&self) -> Option<&str> {
        Provider::incoming_failure(&self.projector)
    }

    /// Parse one bounded query and preserve the caller's original deadline.
    /// The actual provider validates the native query and admits the read.
    pub fn start(&mut self, query: &[u8], until: u64) -> io::Result<u64> {
        if query.is_empty() || query.len() > pokelinux_messages::protocol::MAX_PACKET {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let query: serde_json::Value = serde_json::from_slice(query)?;
        Provider::start(&mut self.projector, query, until)
    }

    pub fn poll(&mut self) -> io::Result<Option<ReadEvent>> {
        Ok(
            Provider::poll(&mut self.projector)?.map(|event| match event {
                Event::Complete { token, packet } => ReadEvent::Complete { token, packet },
                Event::Failed { token } => ReadEvent::Failed { token },
            }),
        )
    }

    pub fn cancel_read(&mut self, token: u64) {
        Provider::cancel_read(&mut self.projector, token);
    }

    pub fn stop(&mut self) {
        Provider::stop(&mut self.projector);
    }

    pub fn restart(&mut self) -> io::Result<()> {
        Provider::restart(&mut self.projector)
    }

    pub fn drain_until(&mut self, until: u64) -> bool {
        Provider::drain_until(&mut self.projector, until)
    }
}

impl Drop for History {
    fn drop(&mut self) {
        if self.occupied() {
            // Early service failure still owns both children. First use the
            // normal stop/poll/wait drain; occupancy alone is not an error.
            let until = boottime_ns().saturating_add(DROP_DRAIN_NS);
            if !self.drain_until(until) || self.occupied() {
                // An unresolved owner cannot silently drop live Child handles.
                std::process::abort();
            }
        }
    }
}
