#![forbid(unsafe_code)]

use crate::{
    clock,
    layout::*,
    protocol::{self, Snapshot},
};

pub struct Engine {
    observation: View,
    snapshot: Option<Snapshot>,
    has_owner: bool,
    native: bool,
    read_watch: bool,
    connect_pending: bool,
    immediate_send: bool,
    readiness: u32,
    stopped: bool,
    effect: u64,
    outstanding: Option<Step>,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            observation: View {
                connection_generation: 1,
                poll_interval_ms: 250,
                ..View::default()
            },
            snapshot: None,
            has_owner: false,
            native: false,
            read_watch: false,
            connect_pending: false,
            immediate_send: false,
            readiness: 0,
            stopped: false,
            effect: 0,
            outstanding: None,
        }
    }
}
fn milliseconds(until: u64, now: u64) -> u32 {
    let remaining = until.saturating_sub(now);
    (remaining / 1_000_000 + u64::from(!remaining.is_multiple_of(1_000_000))).min(i32::MAX as u64)
        as u32
}
impl Engine {
    fn active(&self) -> bool {
        matches!(self.observation.stage, CONNECTING | SENDING | RECEIVING)
    }
    fn terminal(&mut self) {
        // Exhausted identities are never reused. Native edge must retire local
        // resources even when another CLOSE token cannot be admitted.
        // Withdrawing a live cache still needs its final notification when
        // request/effect/connection identity exhaustion caused termination.
        if let Some(next) = self
            .observation
            .change_sequence
            .checked_add(u64::from(self.observation.valid != 0))
        {
            self.observation.change_sequence = next;
        }
        self.stopped = true;
        self.observation.stage = STOPPED;
        self.observation.poll_interval_ms = 0;
        self.observation.valid = 0;
        self.observation.original_until_boottime_ns = 0;
        self.observation.request_timer_ms = 0;
        self.observation.expiry_timer_ms = 0;
        self.readiness = 0;
        self.outstanding = None;
    }
    fn changed(&mut self) -> bool {
        if let Some(next) = self.observation.change_sequence.checked_add(1) {
            self.observation.change_sequence = next;
            true
        } else {
            self.terminal();
            false
        }
    }
    fn invalidate(&mut self) {
        self.observation.expiry_timer_ms = 0;
        if self.observation.valid != 0 {
            self.observation.valid = 0;
            self.changed();
        }
    }
    fn fail(&mut self) {
        self.outstanding = None;
        self.readiness = 0;
        self.immediate_send = false;
        self.observation.original_until_boottime_ns = 0;
        self.observation.request_timer_ms = 0;
        self.invalidate();
        self.observation.stage = if self.native {
            CLOSING
        } else if self.stopped {
            STOPPED
        } else {
            IDLE
        };
    }
    fn retired(&mut self) {
        self.native = false;
        self.read_watch = false;
        self.connect_pending = false;
        self.immediate_send = false;
        self.readiness = 0;
        self.outstanding = None;
        self.observation.original_until_boottime_ns = 0;
        self.observation.request_timer_ms = 0;
        if let Some(next) = self.observation.connection_generation.checked_add(1) {
            self.observation.connection_generation = next;
            self.observation.stage = if self.stopped { STOPPED } else { IDLE };
        } else {
            self.terminal();
        }
    }
    fn timers(&mut self, now: u64) {
        self.observation.request_timer_ms = if self.active() {
            milliseconds(self.observation.original_until_boottime_ns, now)
        } else {
            0
        };
        self.observation.expiry_timer_ms = if self.observation.valid != 0 {
            milliseconds(self.observation.palette_until_boottime_ns, now)
        } else {
            0
        };
    }
    fn observe(&mut self) -> Result<u64, i32> {
        let Some(now) = clock::now() else {
            self.fail();
            return Err(REFUSED);
        };
        if self.observation.valid != 0 && now >= self.observation.palette_until_boottime_ns {
            self.invalidate();
        }
        if self.active() && now >= self.observation.original_until_boottime_ns {
            self.fail();
            return Err(EXPIRED);
        }
        self.timers(now);
        Ok(now)
    }
    pub fn view(&self) -> View {
        let mut view = self.observation;
        view.has_snapshot = u32::from(self.snapshot.is_some());
        view.read_enabled =
            u32::from(self.native && self.read_watch && !matches!(view.stage, CLOSING | STOPPED));
        view.write_enabled = u32::from(
            self.native
                && ((view.stage == CONNECTING && self.connect_pending)
                    || (view.stage == SENDING && !self.immediate_send)),
        );
        if let Some(model) = &self.snapshot {
            view.base_size = model.base_size;
            view.font_scale = f64::from(model.base_size) / 12.0;
            view.palette = model.palette;
            view.font_tokens = model.font_tokens;
            view.spacing_tokens = model.spacing_tokens;
        }
        view
    }
    pub fn available(&self) -> bool {
        let now = clock::now();
        self.observation.valid != 0
            && now.is_some_and(|now| now < self.observation.palette_until_boottime_ns)
    }
    pub fn refresh(&mut self) -> i32 {
        if self.stopped {
            return REFUSED;
        }
        let now = match self.observe() {
            Ok(now) => now,
            Err(code) => return code,
        };
        if self.observation.stage != IDLE {
            return WAIT;
        }
        let Some(until) = now.checked_add(500_000_000) else {
            self.fail();
            return REFUSED;
        };
        let Some(request) = self.observation.request_token.checked_add(1) else {
            self.terminal();
            return REFUSED;
        };
        self.observation.request_token = request;
        self.observation.original_until_boottime_ns = until;
        self.observation.stage = if self.native { SENDING } else { CONNECTING };
        self.immediate_send = self.native;
        self.connect_pending = false;
        self.timers(now);
        OK
    }
    pub fn ready(&mut self, generation: u64, event: u32) -> i32 {
        if generation != self.observation.connection_generation {
            return STALE;
        }
        let view = self.view();
        let enabled = match event {
            READ_READY => view.read_enabled,
            WRITE_READY => view.write_enabled,
            _ => 0,
        };
        if enabled == 0 {
            return REFUSED;
        }
        self.readiness |= event;
        OK
    }
    fn issue(&mut self, action: u32) -> Result<Step, i32> {
        let Some(effect) = self.effect.checked_add(1) else {
            self.terminal();
            return Err(REFUSED);
        };
        self.effect = effect;
        let mut plan = Step {
            connection_generation: self.observation.connection_generation,
            request_token: self.observation.request_token,
            effect_token: effect,
            original_until_boottime_ns: self.observation.original_until_boottime_ns,
            action,
            ..Step::default()
        };
        if action == SEND {
            plan.packet_length = QUERY.len() as u32;
            plan.packet[..QUERY.len()].copy_from_slice(QUERY);
        }
        if action == CONNECT {
            self.native = true;
        }
        self.outstanding = Some(plan);
        Ok(plan)
    }
    pub fn step(&mut self) -> Result<Step, i32> {
        if self.observation.stage == STOPPED {
            return Err(EMPTY);
        }
        // Local retirement never requires a working clock or renewed deadline.
        if self.observation.stage == CLOSING {
            return if self.outstanding.is_some() {
                Err(WAIT)
            } else {
                self.issue(CLOSE)
            };
        }
        self.observe()?;
        if self.outstanding.is_some() {
            return Err(WAIT);
        }
        if self.readiness & READ_READY != 0 && self.view().read_enabled != 0 {
            self.readiness &= !READ_READY;
            return self.issue(RECEIVE);
        }
        match self.observation.stage {
            CONNECTING if !self.connect_pending => self.issue(CONNECT),
            CONNECTING if self.readiness & WRITE_READY != 0 => {
                self.readiness &= !WRITE_READY;
                self.issue(FINISH_CONNECT)
            }
            SENDING if self.immediate_send || self.readiness & WRITE_READY != 0 => {
                self.immediate_send = false;
                self.readiness &= !WRITE_READY;
                self.issue(SEND)
            }
            IDLE => Err(EMPTY),
            _ => Err(WAIT),
        }
    }
    fn matches(&self, plan: Step) -> bool {
        self.outstanding == Some(plan)
    }
    pub fn report(&mut self, plan: Step, outcome: u32, sent: u32) -> i32 {
        if self.stopped && plan.action != CLOSE {
            return REFUSED;
        }
        if !self.matches(plan) {
            return STALE;
        }
        let permitted = match plan.action {
            CONNECT => {
                matches!(
                    outcome,
                    CONNECTED | CONNECT_PENDING | CONNECT_FAILED_CLOSED | IO_FAILED
                ) && sent == 0
            }
            FINISH_CONNECT => matches!(outcome, CONNECTED | IO_FAILED) && sent == 0,
            SEND => {
                (outcome == SENT && sent == plan.packet_length)
                    || (matches!(outcome, WOULD_BLOCK | INTERRUPTED | IO_FAILED) && sent == 0)
            }
            RECEIVE => matches!(outcome, WOULD_BLOCK | INTERRUPTED | IO_FAILED) && sent == 0,
            CLOSE => outcome == CLOSED && sent == 0,
            _ => false,
        };
        if !permitted {
            return INVALID;
        }
        self.outstanding = None;
        if outcome == CLOSED {
            self.retired();
            return OK;
        }
        if outcome == CONNECT_FAILED_CLOSED {
            self.fail();
            self.retired();
            return OK;
        }
        let now = match self.observe() {
            Ok(now) => now,
            Err(code) => return code,
        };
        match outcome {
            CONNECTED => {
                self.observation.stage = SENDING;
                self.connect_pending = false;
                self.immediate_send = true;
            }
            CONNECT_PENDING => self.connect_pending = true,
            SENT => {
                self.observation.stage = RECEIVING;
                self.read_watch = true;
            }
            WOULD_BLOCK | INTERRUPTED => {}
            IO_FAILED => {
                self.fail();
                return OK;
            }
            _ => return INVALID,
        }
        self.timers(now);
        OK
    }
    pub fn received(&mut self, plan: Step, bytes: &[u8]) -> i32 {
        if self.stopped {
            return REFUSED;
        }
        if !self.matches(plan) || plan.action != RECEIVE {
            return STALE;
        }
        self.outstanding = None;
        if self.observation.stage != RECEIVING {
            self.fail();
            return REFUSED;
        }
        if let Err(code) = self.observe() {
            return code;
        }
        let Some(response) = protocol::decode(bytes) else {
            self.fail();
            return REFUSED;
        };
        // A real recvmsg or decoding may cross the original admission cutoff.
        let now = match self.observe() {
            Ok(now) => now,
            Err(code) => return code,
        };
        if now >= response.until {
            self.fail();
            return EXPIRED;
        }
        if self.has_owner
            && self.observation.owner == response.owner
            && (response.revision < self.observation.revision
                || (response.revision == self.observation.revision
                    && response.snapshot.is_some()
                    && self.snapshot.is_some()
                    && response.snapshot != self.snapshot))
        {
            self.fail();
            return REFUSED;
        }
        let published = response.snapshot.is_some();
        let changed = (self.observation.valid != 0) != published
            || (published && self.snapshot != response.snapshot);
        if changed && !self.changed() {
            return REFUSED;
        }
        self.has_owner = true;
        self.observation.owner = response.owner;
        self.observation.revision = response.revision;
        self.observation.palette_until_boottime_ns = response.until;
        self.observation.valid = u32::from(published);
        if published {
            self.snapshot = response.snapshot;
        }
        self.observation.stage = IDLE;
        self.observation.original_until_boottime_ns = 0;
        self.immediate_send = false;
        self.readiness = 0;
        self.timers(now);
        OK
    }
    pub fn tick(&mut self) -> i32 {
        if self.stopped {
            return REFUSED;
        }
        match self.observe() {
            Ok(_) => OK,
            Err(code) => code,
        }
    }
    pub fn stop(&mut self) -> i32 {
        if !self.stopped {
            self.stopped = true;
            self.observation.poll_interval_ms = 0;
            self.fail();
        }
        OK
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_request_or_effect_invalidates_and_notifies_once() {
        for effect_counter in [false, true] {
            let mut state = Engine {
                native: true,
                read_watch: true,
                has_owner: true,
                snapshot: Some(Snapshot {
                    base_size: 12,
                    font_tokens: [12; 8],
                    ..Snapshot::default()
                }),
                ..Engine::default()
            };
            state.observation.valid = 1;
            state.observation.palette_until_boottime_ns = u64::MAX;
            state.observation.change_sequence = 41;
            if effect_counter {
                state.observation.stage = SENDING;
                state.observation.original_until_boottime_ns =
                    clock::now().unwrap().checked_add(500_000_000).unwrap();
                state.immediate_send = true;
                state.effect = u64::MAX;
                assert_eq!(state.step(), Err(REFUSED));
                assert_eq!(state.effect, u64::MAX);
            } else {
                state.observation.request_token = u64::MAX;
                assert_eq!(state.refresh(), REFUSED);
                assert_eq!(state.observation.request_token, u64::MAX);
            }
            let view = state.view();
            assert_eq!(view.valid, 0);
            assert_eq!(
                view.change_sequence, 42,
                "terminal availability withdrawal must notify"
            );
            assert_eq!(
                (
                    view.stage,
                    view.poll_interval_ms,
                    view.read_enabled,
                    view.write_enabled
                ),
                (STOPPED, 0, 0, 0)
            );
            assert_eq!(state.step(), Err(EMPTY));
            assert_eq!(state.view().change_sequence, 42);
        }
    }
}
