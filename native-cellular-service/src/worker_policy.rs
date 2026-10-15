//! Classify ordinary observation errors; this never dispatches a provider operation.
use pokelinux_cellular_owner::Error;
/// Continue only ordinary observation polling for a concurrent kernel candidate.
/// DataOwner retains its own deadline and permanent retirement rules. No second
/// attach/setup, provider retry or new owner is created here.
pub fn terminal_poll_error(error: Error) -> bool {
    !matches!(
        error,
        Error::Kernel(pokelinux_network_state::ObserveError::ConcurrentChange)
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use pokelinux_network_state::ObserveError;
    #[test]
    fn concurrent_kernel_candidate_change_keeps_same_attempt_pollable() {
        // The owner's public contract leaves this error transient before a join;
        // after a join it retires association itself. Neither case authorizes setup again.
        assert!(
            !terminal_poll_error(Error::Kernel(ObserveError::ConcurrentChange)),
            "MISSING_TRANSIENT_KERNEL_CANDIDATE_POLICY"
        );
    }
    #[test]
    fn deadline_callback_provider_and_kernel_faults_remain_terminal() {
        for error in [
            Error::Unsupported,
            Error::Deadline,
            Error::CallbackFault,
            Error::Stopped,
            Error::AlreadySubmitted,
            Error::Binder,
            Error::Kernel(ObserveError::Unsupported),
            Error::Kernel(ObserveError::Deadline),
            Error::Kernel(ObserveError::Io(5)),
            Error::Kernel(ObserveError::InvalidData),
            Error::Kernel(ObserveError::BoundExceeded),
            Error::Kernel(ObserveError::EvidenceLost),
        ] {
            assert!(
                terminal_poll_error(error),
                "unexpected nonterminal error: {error:?}"
            );
        }
    }
}
