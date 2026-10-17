#![forbid(unsafe_code)]

use crate::{clock, layout::*, model::Model, protocol};
use std::collections::VecDeque;

const EXCHANGE_NS: u64 = 1_000_000_000;
const CYCLE_NS: u64 = 250_000_000;
const RETRY_NS: u64 = 100_000_000;
const STOP_NS: u64 = 500_000_000;

#[derive(Clone)]
struct Request {
    send: Box<Step>,
    receive: Box<Step>,
}
#[derive(Clone)]
struct Notification {
    plan: Box<Step>,
    issued: bool,
}
#[derive(Clone)]
struct Completion {
    notification: Option<Notification>,
    wake: Option<u64>,
    close: bool,
    terminal: bool,
}
#[derive(Clone)]
enum Phase {
    New,
    NeedConnect,
    Connecting(Box<Step>),
    Sending(Request),
    Receiving(Request),
    Complete(Completion),
    Cooldown(Box<Step>),
    Exit,
}
struct Event {
    token: u64,
    generation: u64,
    bytes: Vec<u8>,
}

pub struct Engine {
    model: Model,
    phase: Phase,
    entered: bool,
    exited: bool,
    joined: bool,
    stopping: bool,
    stop_until: Option<u64>,
    connected: bool,
    close_obligation: bool,
    close_plan: Option<Box<Step>>,
    epoch: u64,
    request_token: u64,
    event_token: u64,
    conversation_cycle: bool,
    events: VecDeque<Event>,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            model: Model::default(),
            phase: Phase::New,
            entered: false,
            exited: false,
            joined: false,
            stopping: false,
            stop_until: None,
            connected: false,
            close_obligation: false,
            close_plan: None,
            epoch: 0,
            request_token: 0,
            event_token: 0,
            conversation_cycle: true,
            events: VecDeque::new(),
        }
    }
}
impl Engine {
    fn plan(&self, action: u32) -> Step {
        Step {
            generation: self.model.generation,
            connection_epoch: self.epoch,
            action,
            ..Step::default()
        }
    }
    pub fn can_free(&self) -> bool {
        !self.entered || (self.exited && self.joined)
    }
    pub fn enter(&mut self) -> i32 {
        if self.entered {
            return REFUSED;
        }
        self.entered = true;
        // Stop can win before the created native QThread enters its closure.
        self.phase = if self.stopping {
            Phase::Exit
        } else {
            Phase::NeedConnect
        };
        OK
    }
    pub fn view(&self) -> View {
        let mut view = self.model.view();
        view.connection_epoch = self.epoch;
        view.queued_events = self.events.len() as u64;
        view.stopping = u32::from(self.stopping);
        view.worker_active = u32::from(self.entered && !self.exited);
        view.stage = if self.stopping {
            STOPPED
        } else {
            match &self.phase {
                Phase::New => NEW,
                Phase::NeedConnect | Phase::Connecting(_) => CONNECTING,
                Phase::Sending(_) => SENDING,
                Phase::Receiving(_) => RECEIVING,
                Phase::Complete(_) | Phase::Cooldown(_) => COOLDOWN,
                Phase::Exit => STOPPED,
            }
        };
        if !self.stopping {
            if let Phase::Sending(request) | Phase::Receiving(request) = &self.phase {
                view.request_token = request.send.request_token;
                view.original_until_boottime_ns = request.send.original_until_boottime_ns;
            }
        }
        view
    }
    pub fn action(&mut self, action: u32, id: Option<Id>) -> i32 {
        if self.stopping {
            return REFUSED;
        }
        let result = self.model.action(action, id);
        if result == ABORT {
            self.stop();
            return REFUSED;
        }
        result
    }
    fn fail_counter(&mut self) -> i32 {
        self.stop();
        REFUSED
    }
    fn begin_request(&mut self, sample: Option<u64>) -> i32 {
        let generation = self.model.generation;
        let Some(until) = sample.and_then(|v| v.checked_add(EXCHANGE_NS)) else {
            return self.complete(generation, Vec::new(), None, true, true);
        };
        let Some(token) = self.request_token.checked_add(1) else {
            return self.fail_counter();
        };
        let packet = protocol::request(
            self.conversation_cycle,
            self.model.selected,
            self.model.cursor,
            self.model.after,
        );
        if packet.len() > CAPACITY {
            return self.fail_counter();
        }
        self.request_token = token;
        let mut send = self.plan(SEND);
        send.generation = generation;
        send.request_token = token;
        send.original_until_boottime_ns = until;
        send.packet_length = packet.len() as u32;
        send.wait_slice_ms = 20;
        send.packet[..packet.len()].copy_from_slice(&packet);
        let receive = Step {
            action: RECEIVE,
            packet_length: 0,
            packet: [0; CAPACITY],
            ..send
        };
        self.phase = Phase::Sending(Request {
            send: Box::new(send),
            receive: Box::new(receive),
        });
        OK
    }
    /// Complete once; an absent clock or overflowing wake is terminal, never a
    /// fabricated future wake. This internal transition does not call Stop.
    fn complete(
        &mut self,
        generation: u64,
        bytes: Vec<u8>,
        sample: Option<u64>,
        close: bool,
        terminal: bool,
    ) -> i32 {
        let wake = sample.and_then(|v| v.checked_add(CYCLE_NS));
        let terminal = terminal || wake.is_none();
        let Some(token) = self.event_token.checked_add(1) else {
            return self.fail_counter();
        };
        self.event_token = token;
        self.conversation_cycle = !self.conversation_cycle;
        self.events.push_back(Event {
            token,
            generation,
            bytes: if terminal { Vec::new() } else { bytes },
        });
        let mut plan = self.plan(NOTIFY);
        plan.generation = generation;
        plan.event_token = token;
        plan.wake_boottime_ns = wake.unwrap_or(0);
        self.phase = Phase::Complete(Completion {
            notification: Some(Notification {
                plan: Box::new(plan),
                issued: false,
            }),
            wake,
            close: close || terminal,
            terminal,
        });
        OK
    }
    fn close_step(&mut self) -> Step {
        if self.close_plan.is_none() {
            self.close_plan = Some(Box::new(self.plan(CLOSE)));
        }
        // A close plan is stable even if a UI action or Stop occurs meanwhile.
        self.close_plan.as_deref().copied().unwrap_or_default()
    }
    fn sleep(&self, wake: u64) -> Box<Step> {
        Box::new(Step {
            wake_boottime_ns: wake,
            wait_slice_ms: 10,
            ..self.plan(SLEEP)
        })
    }
    fn check_request(&mut self, plan: &Step) -> i32 {
        let sample = clock::now();
        match sample {
            Some(now) if now < plan.original_until_boottime_ns => OK,
            Some(now) => {
                self.complete(plan.generation, Vec::new(), Some(now), true, false);
                EXPIRED
            }
            None => {
                self.complete(plan.generation, Vec::new(), None, true, true);
                REFUSED
            }
        }
    }
    pub fn step(&mut self) -> Result<Step, i32> {
        if !self.entered || self.exited {
            return Err(REFUSED);
        }
        loop {
            if self.stopping {
                return Ok(if self.close_obligation {
                    self.close_step()
                } else {
                    self.plan(EXIT)
                });
            }
            match self.phase.clone() {
                Phase::New => return Err(REFUSED),
                Phase::NeedConnect => {
                    let Some(epoch) = self.epoch.checked_add(1) else {
                        return Err(self.fail_counter());
                    };
                    self.epoch = epoch;
                    let plan = self.plan(CONNECT);
                    self.close_obligation = true;
                    self.phase = Phase::Connecting(Box::new(plan));
                    return Ok(plan);
                }
                Phase::Connecting(plan) => return Ok(*plan),
                Phase::Sending(request) => {
                    if self.check_request(&request.send) == OK {
                        return Ok(*request.send);
                    }
                }
                Phase::Receiving(request) => {
                    if self.check_request(&request.receive) == OK {
                        return Ok(*request.receive);
                    }
                }
                Phase::Complete(mut completion) => {
                    if let Some(notification) = completion.notification.as_mut() {
                        if notification.issued {
                            return Err(WAIT);
                        }
                        notification.issued = true;
                        let plan = *notification.plan;
                        self.phase = Phase::Complete(completion);
                        return Ok(plan);
                    }
                    if completion.close && self.close_obligation {
                        return Ok(self.close_step());
                    }
                    self.phase = if completion.terminal {
                        Phase::Exit
                    } else if let Some(wake) = completion.wake {
                        Phase::Cooldown(self.sleep(wake))
                    } else {
                        Phase::Exit
                    };
                }
                Phase::Cooldown(plan) => match clock::now() {
                    Some(now) if now < plan.wake_boottime_ns => return Ok(*plan),
                    Some(now) => {
                        if self.connected {
                            self.begin_request(Some(now));
                        } else {
                            self.phase = Phase::NeedConnect;
                        }
                    }
                    None => {
                        self.complete(
                            self.model.generation,
                            Vec::new(),
                            None,
                            self.close_obligation,
                            true,
                        );
                    }
                },
                Phase::Exit => {
                    return Ok(if self.close_obligation {
                        self.close_step()
                    } else {
                        self.plan(EXIT)
                    })
                }
            }
        }
    }
    pub fn report(&mut self, plan: Step, outcome: u32, sent: u32) -> i32 {
        if !self.entered || self.exited {
            return REFUSED;
        }
        if plan.reserved != 0 || plan.packet_length as usize > CAPACITY {
            return INVALID;
        }
        // CLOSED is the only accepted report during Stop. Native fd custody is
        // never discharged by a refused late CONNECTED or SENT report.
        if outcome == CLOSED {
            if sent != 0 || plan.action != CLOSE {
                return INVALID;
            }
            if !self.close_obligation || self.close_plan.as_deref() != Some(&plan) {
                return STALE;
            }
            self.close_obligation = false;
            self.connected = false;
            self.close_plan = None;
            if let Phase::Complete(completion) = &mut self.phase {
                completion.close = false;
            }
            return OK;
        }
        if self.stopping {
            return REFUSED;
        }
        if outcome != SENT && sent != 0 {
            return INVALID;
        }
        match self.phase.clone() {
            Phase::Connecting(expected) if *expected == plan => match outcome {
                CONNECTED => {
                    self.connected = true;
                    let result = self.begin_request(clock::now());
                    // The credentialed native connect happened; the clock may
                    // still refuse admitting any SEND, leaving a terminal plan.
                    if result == OK && matches!(self.phase, Phase::Sending(_)) {
                        OK
                    } else {
                        REFUSED
                    }
                }
                CONNECT_FAILED => {
                    self.close_obligation = false;
                    self.connected = false;
                    match clock::now().and_then(|v| v.checked_add(RETRY_NS)) {
                        Some(wake) => self.phase = Phase::Cooldown(self.sleep(wake)),
                        None => {
                            self.complete(self.model.generation, Vec::new(), None, false, true);
                        }
                    }
                    OK
                }
                // Native path rejection is before socket creation; the adapter
                // reports it only with no actual fd, and does not retry.
                PATH_REFUSED => {
                    self.close_obligation = false;
                    self.connected = false;
                    self.phase = Phase::Exit;
                    OK
                }
                _ => INVALID,
            },
            Phase::Sending(request) if *request.send == plan => {
                if !matches!(outcome, SENT | WOULD_BLOCK | INTERRUPTED | IO_FAILED) {
                    return INVALID;
                }
                let checked = self.check_request(&plan);
                if checked != OK {
                    return checked;
                }
                match outcome {
                    SENT if sent == plan.packet_length => {
                        self.phase = Phase::Receiving(request);
                        OK
                    }
                    WOULD_BLOCK | INTERRUPTED => OK,
                    _ => {
                        self.complete(plan.generation, Vec::new(), clock::now(), true, false);
                        OK
                    }
                }
            }
            Phase::Receiving(request) if *request.receive == plan => {
                if !matches!(outcome, WOULD_BLOCK | INTERRUPTED | IO_FAILED) {
                    return INVALID;
                }
                let checked = self.check_request(&plan);
                if checked != OK {
                    return checked;
                }
                if outcome == IO_FAILED {
                    self.complete(plan.generation, Vec::new(), clock::now(), true, false);
                }
                OK
            }
            Phase::Complete(mut completion) if outcome == NOTIFY_QUEUED => {
                let matches = completion
                    .notification
                    .as_ref()
                    .is_some_and(|n| n.issued && *n.plan == plan);
                if !matches {
                    return STALE;
                }
                // Independent of event FIFO existence/current UI generation.
                completion.notification = None;
                self.phase = Phase::Complete(completion);
                OK
            }
            _ => STALE,
        }
    }
    pub fn received(&mut self, plan: Step, bytes: Vec<u8>) -> i32 {
        if self.stopping || !self.entered || self.exited {
            return REFUSED;
        }
        if plan.reserved != 0 || bytes.len() > CAPACITY {
            return INVALID;
        }
        let Phase::Receiving(request) = &self.phase else {
            return STALE;
        };
        if *request.receive != plan {
            return STALE;
        }
        let sample = clock::now();
        match sample {
            Some(now) if now < plan.original_until_boottime_ns => {
                let empty = bytes.is_empty();
                self.complete(plan.generation, bytes, Some(now), empty, false)
            }
            Some(now) => {
                self.complete(plan.generation, Vec::new(), Some(now), true, false);
                EXPIRED
            }
            None => {
                self.complete(plan.generation, Vec::new(), None, true, true);
                REFUSED
            }
        }
    }
    pub fn deliver(&mut self, token: u64) -> i32 {
        if self.stopping {
            return REFUSED;
        }
        if !self
            .events
            .front()
            .is_some_and(|event| event.token == token)
        {
            return STALE;
        }
        let Some(event) = self.events.pop_front() else {
            return STALE;
        };
        if event.generation != self.model.generation {
            return STALE;
        }
        let result = self.model.deliver(&event.bytes);
        if result == ABORT {
            self.stop();
            return REFUSED;
        }
        result
    }
    pub fn stop(&mut self) -> i32 {
        if self.stopping {
            return OK;
        }
        self.stopping = true;
        self.stop_until = clock::now().and_then(|v| v.checked_add(STOP_NS));
        self.events.clear();
        self.phase = Phase::Exit;
        // Keep any copied CLOSE identity and unknown native CONNECT obligation.
        OK
    }
    pub fn worker_exited(&mut self) -> i32 {
        if !self.entered
            || self.exited
            || self.close_obligation
            || (!self.stopping && !matches!(self.phase, Phase::Exit))
        {
            return REFUSED;
        }
        self.exited = true;
        OK
    }
    pub fn join_result(&mut self, actual_completed: u32) -> i32 {
        if actual_completed > 1 {
            return INVALID;
        }
        // Stop may precede the created native worker entering its closure.
        if !self.stopping {
            return REFUSED;
        }
        if actual_completed == 1 && self.exited {
            self.joined = true;
            return OK;
        }
        if actual_completed == 1 {
            return ABORT;
        }
        match (self.stop_until, clock::now()) {
            (Some(until), Some(now)) if now < until => WAIT,
            _ => ABORT,
        }
    }
}
