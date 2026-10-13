//! Trusted per-app display provisioning. No app-supplied paths or launch policy.
use crate::AppProfile;
use std::io;

/// Within the existing two-second client RPC, setup gets one original second.
/// The remaining response interval is not an initialization deadline extension.
pub const INITIALIZATION_BUDGET_NS: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Preparing,
    Ready,
    Terminal,
}

/// One exact, reserved launch instance. Poll must be bounded/nonblocking.
pub trait Context {
    fn poll(&mut self) -> io::Result<State>;
    /// Trusted installation arguments, including this instance's distinct path.
    fn arguments(&self) -> &[String];
    /// Withdraw the protected pathname before signalling the owned SDK child.
    /// Errors retain ownership; they never authorize replacement or detachment.
    fn stop(&mut self) -> io::Result<()>;
    /// True until every owned helper has an actual parent wait result.
    fn occupied(&self) -> bool;
}

/// Root's installed provisioner returns all ownership to the actor's app slot.
/// Errors must precede helper creation. A failed but occupied helper must instead
/// be returned as a terminal Context for continued cancellation and reaping.
pub trait Factory {
    fn start(
        &mut self,
        profile: &AppProfile,
        instance: &str,
        until: u64,
    ) -> io::Result<Box<dyn Context>>;
}
