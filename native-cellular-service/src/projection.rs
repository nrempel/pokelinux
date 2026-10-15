//! Parent-local projection after separate process admission. Never operation authority.
use crate::wire::{Association, Disposition, Frame, Report};
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub incarnation: String,
    pub attempt: u64,
    pub source_revision: u64,
    pub source_sampled_at_ns: u64,
    pub provider_disposition: Disposition,
    #[serde(deserialize_with = "crate::wire::required_nullable")]
    pub provider_report: Option<Report>,
    #[serde(deserialize_with = "crate::wire::required_nullable")]
    pub association: Option<Association>,
    pub observer_initial_sync_completed: bool,
}
impl Projection {
    pub fn new(incarnation: String) -> Self {
        Self {
            incarnation,
            attempt: 1,
            source_revision: 0,
            source_sampled_at_ns: 0,
            provider_disposition: Disposition::NotSubmitted,
            provider_report: None,
            association: None,
            observer_initial_sync_completed: false,
        }
    }
    /// Caller must first admit this frame from the retained Supervisor/Ready child.
    pub fn accept(
        &mut self,
        frame: &Frame,
        serial: i32,
        until: u64,
        now: u64,
        namespace: (u64, u64),
    ) -> io::Result<()> {
        if frame.incarnation != self.incarnation
            || frame.attempt != 1
            || frame.revision <= self.source_revision
            || frame.serial != serial
            || frame.until_ns != until
            || now >= until
            || frame.sampled_at_ns > now
            || frame.sampled_at_ns < self.source_sampled_at_ns
            || !frame.observer_initial_sync_completed
            || frame
                .association
                .as_ref()
                .is_some_and(|a| (a.namespace.dev, a.namespace.ino) != namespace)
            || self.provider_report.as_ref().is_some_and(|report| {
                frame.provider_report.as_ref() != Some(report)
                    || frame.disposition != self.provider_disposition
            })
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.source_revision = frame.revision;
        self.source_sampled_at_ns = frame.sampled_at_ns;
        self.provider_disposition = frame.disposition.clone();
        self.provider_report = frame.provider_report.clone();
        self.association = frame.association.clone();
        self.observer_initial_sync_completed = true;
        Ok(())
    }
    pub fn withdraw(&mut self) {
        self.association = None;
    }
}
#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
