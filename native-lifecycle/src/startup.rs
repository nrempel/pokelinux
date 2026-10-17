//! Startup observation consumes the actor's accepted Source cutoff only.
//! It grants no permission, renews no sample and does not read Source packets.
use std::io;

pub trait Observer {
    /// Called from the existing actor. None means no current startup eligibility;
    /// locked sessions may be eligible. Stop must fence all later publication.
    fn poll(&mut self, source_until: Option<u64>, stopping: bool) -> io::Result<()>;
}
