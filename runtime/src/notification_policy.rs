//! Desktop-banner dismissal only. No Android commands, ringtone or DND changes.
//! Owned above bus generations so a notifier restart does not reset dismissal.
use crate::{
    Identity,
    watch::{RingingCall, Snapshot, SourceState},
};

#[derive(Clone, Default)]
pub struct Dismissals {
    generation: Option<Identity>,
    active: Vec<RingingCall>,
    dismissed: Vec<RingingCall>,
}
fn same(left: &RingingCall, right: &RingingCall) -> bool {
    left.id() == right.id() && left.episode() == right.episode()
}
impl Dismissals {
    #[cfg(target_os = "linux")]
    pub(crate) fn stored(&self) -> (Option<&Identity>, Vec<(u32, &Identity, bool)>) {
        (
            self.generation.as_ref(),
            self.active
                .iter()
                .map(|call| {
                    (
                        call.id(),
                        call.episode(),
                        self.dismissed.iter().any(|entry| same(call, entry)),
                    )
                })
                .collect(),
        )
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn restore(
        generation: Option<Identity>,
        records: Vec<(u32, Identity, bool)>,
    ) -> Result<Self, &'static str> {
        if records.len() > 64 || (generation.is_none() && !records.is_empty()) {
            return Err("invalid stored dismissal state");
        }
        let mut state = Self {
            generation,
            active: Vec::new(),
            dismissed: Vec::new(),
        };
        for (id, episode, dismissed) in records {
            let call = RingingCall::from_stored(id, episode)?;
            if state
                .active
                .iter()
                .any(|entry| entry.id() == call.id() || entry.episode() == call.episode())
            {
                return Err("duplicate stored dismissal identity");
            }
            if dismissed {
                state.dismissed.push(call.clone());
            }
            state.active.push(call);
        }
        Ok(state)
    }
    /// Only a complete observed registry can retire episodes. Detachment,
    /// unavailability and transport gaps do not prove that a call has ended.
    pub fn observe(&mut self, current: &Snapshot) {
        if current.source_state() != SourceState::Observed {
            return;
        }
        let generation = current.cursor().generation().clone();
        if self.generation.as_ref() != Some(&generation) {
            self.dismissed.clear();
            self.generation = Some(generation);
        }
        self.active = current.ringing().to_vec();
        self.dismissed
            .retain(|dismissed| self.active.iter().any(|call| same(call, dismissed)));
    }
    /// Record only the original banner's still-active episodes. A delayed
    /// dismissal cannot suppress a new call or replace a newer generation.
    pub fn dismiss(&mut self, presented: &Snapshot, current: &Snapshot) {
        self.observe(current);
        if presented.source_state() != SourceState::Observed
            || self.generation.as_ref() != Some(presented.cursor().generation())
        {
            return;
        }
        for call in &self.active {
            if presented.ringing().iter().any(|shown| same(call, shown))
                && !self.dismissed.iter().any(|entry| same(call, entry))
            {
                self.dismissed.push(call.clone());
            }
        }
        // Both vectors are subsets of one validated snapshot (at most 64).
    }
    /// This checks dismissal only. Freshness, lock policy, permission and owner
    /// validity remain independent requirements for presenting a notification.
    pub fn has_undismissed(&self, current: &Snapshot) -> bool {
        current.source_state() == SourceState::Observed
            && current
                .ringing()
                .iter()
                .any(|call| !self.contains(current, call))
    }
    fn contains(&self, current: &Snapshot, call: &RingingCall) -> bool {
        self.generation.as_ref() == Some(current.cursor().generation())
            && self.dismissed.iter().any(|entry| same(call, entry))
    }
}

/// Cosmetic repeat suppression for one daemon lifetime. Keep this above bus
/// connections, not inside a notification generation. Deliberately not durable:
/// a daemon restart may show a fresh undismissed episode again.
#[derive(Default)]
pub struct BannerAttempts {
    generation: Option<Identity>,
    attempted: Vec<RingingCall>,
}
impl BannerAttempts {
    /// A gap is not proof of call completion. Only a complete observed registry
    /// retires attempts, keeping the vector bounded by the snapshot's 64 calls.
    pub fn observe(&mut self, current: &Snapshot) {
        if current.source_state() != SourceState::Observed {
            return;
        }
        if self.generation.as_ref() != Some(current.cursor().generation()) {
            self.attempted.clear();
            self.generation = Some(current.cursor().generation().clone());
        }
        self.attempted
            .retain(|entry| current.ringing().iter().any(|call| same(call, entry)));
    }
    /// Consume one opportunity for each new undismissed episode in this banner.
    /// The owner calls this after freshness, policy, owner and in-flight capacity
    /// checks, immediately before handing Notify to its adapter. An uncertain
    /// send does not refund the attempt. This grants no display/call authority.
    pub fn claim(&mut self, current: &Snapshot, dismissals: &Dismissals) -> bool {
        self.observe(current);
        if current.source_state() != SourceState::Observed {
            return false;
        }
        let mut claimed = false;
        for call in current.ringing() {
            if !dismissals.contains(current, call)
                && !self.attempted.iter().any(|entry| same(call, entry))
            {
                self.attempted.push(call.clone());
                claimed = true;
            }
        }
        claimed
    }
}
