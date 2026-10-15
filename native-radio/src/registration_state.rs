//! Conditional read projection, kept inside the radio publication gate.
//! Production has no APN owner yet: no production path constructs Context.
//! This is neither Android's asynchronous loss policy nor a command dispatcher.
use crate::{owner::SessionKey, registration::RegistrationWire};
use pokelinux_phone_state::{RegistrationAvailability, RegistrationObservation, RegistrationView};

pub(crate) struct Context {
    pub(crate) session: SessionKey,
    pub(crate) slot: i32,
    pub(crate) live_entity: bool,
    pub(crate) request_membership: bool,
}
#[derive(Default)]
pub(crate) struct Projection {
    pub(crate) generation: u64,
    pub(crate) context: Option<Context>,
    view: RegistrationView,
}
impl Projection {
    pub(crate) fn view(&self) -> RegistrationView {
        self.view
    }
    pub(crate) fn withdraw(&mut self) {
        if self.view.availability != RegistrationAvailability::Unavailable {
            // Positive publication reserves this final withdrawal revision.
            self.view.revision += 1;
            self.view.availability = RegistrationAvailability::Unavailable;
        }
    }
    pub(crate) fn retire(&mut self) {
        self.withdraw();
        self.context = None;
    }
    pub(crate) fn commit(
        &mut self,
        session: &SessionKey,
        slot: i32,
        source_revision: u64,
        wire: &RegistrationWire,
    ) {
        let admitted = self.context.as_ref().is_some_and(|context| {
            context.session == *session
                && context.slot == slot
                && context.live_entity
                && context.request_membership
        });
        let qualified = wire.registration_state_wire == 2
            && wire.feature_wire == 0
            && wire.feature_mask_wire == 0
            && wire.status_code_wire == 0
            && wire.uri_bytes().is_empty()
            && wire.reason_bytes().is_empty()
            && wire.feature_mask_ext_wire.is_none()
            && wire.sip_delegate_mask_wire.is_none()
            && wire.feature_ext_wire.is_none();
        if admitted && qualified && self.view.revision < u64::MAX - 1 {
            let observed = RegistrationObservation {
                source_generation: self.generation,
                source_revision,
            };
            // Reserve one revision for terminal withdrawal; exhaustion can
            // never leave an affirmative view or reopen it after retirement.
            self.view.revision += 1;
            self.view.availability = RegistrationAvailability::RegistrationObserved(observed);
            self.view.last_observed = Some(observed);
        } else {
            self.withdraw();
        }
    }
}

#[cfg(test)]
mod tests;
