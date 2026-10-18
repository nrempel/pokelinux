//! Lifecycle adapter for the verified root brightness owner connection.
//! Attribute readback is separate from an explicit request and from luminance.
use crate::{
    boottime_ns,
    brightness::{Event, Intent, Provider, State},
};
use pokelinux_brightness::client::{Client, Event as ClientEvent};
use std::{io, path::Path};

pub struct Owner {
    client: Client,
    incarnation: String,
}
impl Owner {
    /// Validate deployment selection without connecting or writing a device.
    pub fn new(path: &Path) -> io::Result<Self> {
        Ok(Self::from_client(Client::new(path.into())?))
    }
    fn from_client(client: Client) -> Self {
        let incarnation = Self::connection_name(&client);
        Self {
            client,
            incarnation,
        }
    }
    fn connection_name(client: &Client) -> String {
        format!("brightness-{:016x}", client.incarnation())
    }
}
fn percent(raw: u32, max: u32) -> u8 {
    ((u64::from(raw) * 100 + u64::from(max) / 2) / u64::from(max)) as u8
}
impl Provider for Owner {
    fn incarnation(&self) -> &str {
        &self.incarnation
    }
    fn state(&self) -> State {
        let view = self.client.snapshot();
        let mut state = State {
            connected: view.connected,
            pending: view.pending,
            revision: view.revision,
            ..State::default()
        };
        if let Some(attrs) = view.observation
            && view.connected
            && view.revision > 0
            && boottime_ns() < view.until_ns
            && attrs.max > 0
            && attrs.requested <= attrs.max
            && attrs.actual.is_none_or(|raw| raw <= attrs.max)
        {
            state.available = true;
            state.observed_percent = Some(percent(attrs.requested, attrs.max));
            state.actual_percent = attrs.actual.map(|raw| percent(raw, attrs.max));
            state.requested_percent = if view.last_set_uncertain {
                None
            } else {
                view.requested_percent
            };
            state.until_ns = view.until_ns.to_string();
        }
        state
    }
    fn readonly_in_flight(&self) -> bool {
        self.client.readonly_in_flight()
    }
    fn submit(&mut self, intent: &Intent) -> io::Result<bool> {
        self.client.set(
            intent.id,
            intent.percent,
            intent.expected_revision,
            intent.until_ns,
        )
    }
    fn inspect(&mut self, until_ns: u64) -> io::Result<bool> {
        let result = self.client.inspect(until_ns);
        // An attempted readonly inspection can establish a connection even if
        // its send subsequently fails. Fence the old actor stamp in that case.
        self.incarnation = Self::connection_name(&self.client);
        result
    }
    fn poll(&mut self) -> Option<Event> {
        self.client.poll().map(|event| match event {
            ClientEvent::Observed => Event::Observed,
            ClientEvent::Completed { id, .. } => Event::Completed {
                id,
                state: self.state(),
            },
            ClientEvent::Rejected { id } => Event::Rejected { id },
            ClientEvent::Unavailable => Event::Unavailable,
        })
    }
    fn revoke(&mut self) {
        self.client.revoke();
    }
}
#[cfg(test)]
#[path = "brightness_owner_tests.rs"]
mod tests;
