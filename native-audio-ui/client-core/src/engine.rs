#![forbid(unsafe_code)]
use crate::{
    protocol::{self, Preview, Reply, Row, State},
    queue::Queue,
    wire::{Code, Fresh, RECONNECT, ROUTE, Route, STOP},
};
use pokelinux_power::clock::{BootClock, Clock};
pub const CLOSED: u32 = 0;
pub const CONNECTING: u32 = 1;
pub const SENDING: u32 = 2;
pub const RECEIVING: u32 = 3;
pub const IDLE: u32 = 4;
const CONNECT: u32 = 1;
const STATUS: u32 = 2;
const SELECT: u32 = 3;
const RECON: u32 = 4;
const PLAY: u32 = 5;
const STOP_REQUEST: u32 = 6;
const MUTE_REQUEST: u32 = 7;
// Exact public presentation codes from the frozen C header.
const DISCONNECTED: u32 = 0;
const CONNECTING_AUDIO: u32 = 1;
const UNAVAILABLE_INSPECT: u32 = 2;
const CHECKING: u32 = 3;
const APPLYING: u32 = 4;
const APPLIED: u32 = 5;
const INTERRUPTED: u32 = 6;
const CHOOSE_OUTPUT: u32 = 7;
const ROUTE_UNAVAILABLE: u32 = 8;
const OUTPUT_REQUESTED: u32 = 9;
const DENIED: u32 = 10;
const UNAVAILABLE: u32 = 11;
const STATE_UNAVAILABLE: u32 = 12;
const REQUEST_EXPIRED: u32 = 19;
const CLOCK_UNAVAILABLE: u32 = 20;
const REQUEST_REFUSED: u32 = 21;
const QUEUED_REFUSED: u32 = 22;
const SEND_REFUSED: u32 = 23;
const REPLY_OUTSIDE_REQUEST: u32 = 24;
const REPLY_REFUSED: u32 = 27;
const SELECTED_IDLE: u32 = 28;
const PREVIEW_STARTING: u32 = 29;
const PREVIEW_PLAYING: u32 = 30;
const PREVIEW_RETIRING: u32 = 31;
const PREVIEW_COMPLETED: u32 = 32;
const PREVIEW_FAILED: u32 = 33;
const PREVIEW_CANCELLED: u32 = 34;
const PREVIEW_MUTE_PENDING: u32 = 35;

struct Pending {
    kind: u32,
    until: u64,
    token: u64,
    output: String,
    queue_token: Option<u64>,
    prepared_length: u32,
    preview_route: Option<Route>,
    preview_level: Option<(u32, bool)>,
    preview_mute: Option<(u64, bool)>,
}
pub struct Client {
    started: bool,
    epoch: u64,
    next_token: u64,
    stage: u32,
    pending: Option<Pending>,
    queue: Queue,
    online: bool,
    available: bool,
    choose: bool,
    reconnect: bool,
    status: u32,
    requested: Option<Route>,
    applied: Option<Route>,
    policy_settled: bool,
    preview: Preview,
    preview_generation: Option<u64>,
    playback_muted: Option<bool>,
    selected_level_percent: u32,
    selected_muted: bool,
    rows: Vec<Row>,
    pages: Vec<Row>,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Label {
    pub length: u32,
    pub bytes: [u8; 96],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ViewRow {
    pub id: Route,
    pub label: Label,
}
#[repr(C)]
pub struct View {
    pub connection_epoch: u64,
    pub request_until_boottime_ns: u64,
    pub queued_until_boottime_ns: u64,
    pub stage: u32,
    pub request_kind: u32,
    pub status: u32,
    pub online: u32,
    pub available: u32,
    pub can_choose: u32,
    pub can_reconnect: u32,
    pub route_count: u32,
    pub requested_present: u32,
    pub applied_present: u32,
    pub queued_kind: u32,
    pub reserved: u32,
    pub requested: Route,
    pub applied: Route,
    pub routes: [ViewRow; 32],
    pub policy_settled: u32,
    pub preview: u32,
    pub can_play: u32,
    pub can_stop: u32,
    pub selected_level_percent: u32,
    pub selected_muted: u32,
    pub can_adjust_preview: u32,
    pub preview_reserved: u32,
    pub preview_generation: u64,
    pub playback_muted: u32,
    pub playback_mute_known: u32,
    pub can_toggle_mute: u32,
    pub mute_checked: u32,
    pub mute_is_live: u32,
    pub mute_reserved: u32,
}
#[repr(C)]
pub struct Packet {
    pub connection_epoch: u64,
    pub token: u64,
    pub original_until_boottime_ns: u64,
    pub length: u32,
    pub reserved: u32,
    pub bytes: [u8; 4096],
}
impl Client {
    pub fn new() -> Self {
        Self {
            started: false,
            epoch: 0,
            next_token: 1,
            stage: CLOSED,
            pending: None,
            queue: Queue::new(),
            online: false,
            available: false,
            choose: false,
            reconnect: false,
            status: DISCONNECTED,
            requested: None,
            applied: None,
            policy_settled: false,
            preview: Preview::Idle,
            preview_generation: None,
            playback_muted: None,
            selected_level_percent: 100,
            selected_muted: false,
            rows: Vec::new(),
            pages: Vec::new(),
        }
    }
    fn sample(&mut self) -> Result<u64, Code> {
        let n: Option<u64> = BootClock.now().elapsed().as_nanos().try_into().ok();
        match n.filter(|n| *n <= i64::MAX as u64) {
            Some(n) => Ok(n),
            None => {
                self.fail(CLOCK_UNAVAILABLE);
                Err(Code::Refused)
            }
        }
    }
    fn timely(&mut self) -> Result<u64, Code> {
        let n = self.sample()?;
        if self.pending.as_ref().is_some_and(|p| n >= p.until) {
            self.fail(REQUEST_EXPIRED);
            Err(Code::Expired)
        } else {
            Ok(n)
        }
    }
    fn deadline(&mut self, n: u64) -> Result<u64, Code> {
        match n
            .checked_add(1_000_000_000)
            .filter(|v| *v <= i64::MAX as u64)
        {
            Some(v) => Ok(v),
            None => {
                self.fail(CLOCK_UNAVAILABLE);
                Err(Code::Refused)
            }
        }
    }
    fn fail(&mut self, status: u32) {
        self.pending = None;
        self.stage = CLOSED;
        self.queue.cancel();
        self.online = false;
        self.available = false;
        self.choose = false;
        self.reconnect = false;
        self.requested = None;
        self.applied = None;
        self.policy_settled = false;
        self.preview = Preview::Failed;
        self.preview_generation = None;
        self.playback_muted = None;
        self.rows.clear();
        self.pages.clear();
        self.status = status;
    }
    fn token(&mut self) -> Result<u64, Code> {
        let t = self.next_token;
        match t.checked_add(1) {
            Some(next) => {
                self.next_token = next;
                Ok(t)
            }
            None => {
                self.fail(STATE_UNAVAILABLE);
                Err(Code::Refused)
            }
        }
    }
    fn request(
        &mut self,
        kind: u32,
        until: u64,
        output: String,
        queue_token: Option<u64>,
    ) -> Result<(), Code> {
        let token = self.token()?;
        self.pending = Some(Pending {
            kind,
            until,
            token,
            output,
            queue_token,
            prepared_length: 0,
            preview_route: None,
            preview_level: None,
            preview_mute: None,
        });
        self.stage = SENDING;
        Ok(())
    }
    fn connect(&mut self) -> Result<(), Code> {
        let n = self.sample()?;
        let until = self.deadline(n)?;
        let Some(epoch) = self.epoch.checked_add(1) else {
            self.fail(STATE_UNAVAILABLE);
            return Err(Code::Refused);
        };
        self.started = true;
        self.epoch = epoch;
        self.pending = Some(Pending {
            kind: CONNECT,
            until,
            token: 0,
            output: String::new(),
            queue_token: None,
            prepared_length: 0,
            preview_route: None,
            preview_level: None,
            preview_mute: None,
        });
        self.stage = CONNECTING;
        self.status = CONNECTING_AUDIO;
        Ok(())
    }
    pub fn start(&mut self) -> Code {
        if self.started {
            return if self.stage == CLOSED {
                Code::Refused
            } else {
                Code::Busy
            };
        }
        self.connect().map_or_else(|e| e, |()| Code::Ok)
    }
    pub fn connected(&mut self, epoch: u64) -> Code {
        if epoch != self.epoch {
            return Code::Stale;
        }
        if self.stage != CONNECTING {
            return Code::Refused;
        }
        if let Err(e) = self.timely() {
            return e;
        }
        self.online = true;
        self.pages.clear();
        let until = self.pending.as_ref().expect("connecting request").until;
        self.request(STATUS, until, protocol::status(0), None)
            .map_or_else(|e| e, |()| Code::Ok)
    }
    fn status_or_idle(&self) -> bool {
        self.stage == IDLE || self.pending.as_ref().is_some_and(|p| p.kind == STATUS)
    }
    fn can_choose(&self) -> bool {
        self.online
            && self.available
            && self.choose
            && !self.queue.pending()
            && self.status_or_idle()
    }
    fn can_reconnect(&self) -> bool {
        !self.queue.pending()
            && (self.stage == CLOSED || (self.online && self.reconnect && self.status_or_idle()))
    }
    fn can_adjust_preview(&self) -> bool {
        !self.queue.pending()
            && !self.preview.active()
            && !self.pending.as_ref().is_some_and(|p| {
                matches!(p.kind, SELECT | RECON | PLAY | STOP_REQUEST | MUTE_REQUEST)
            })
    }
    pub fn preview_level(&mut self, percent: u32, muted: u32) -> Code {
        if percent > 100 || muted > 1 {
            return Code::Invalid;
        }
        if !self.can_adjust_preview() {
            return Code::Busy;
        }
        // A local next-preview choice never creates, consumes or renews a request.
        self.selected_level_percent = percent;
        self.selected_muted = muted != 0;
        Code::Ok
    }
    fn can_mute_playing(&self) -> bool {
        self.online
            && self.available
            && self.choose
            && self.policy_settled
            && self.preview == Preview::Playing
            && self.preview_generation.is_some()
            && self.playback_muted.is_some()
            && self
                .applied
                .zip(self.requested)
                .is_some_and(|(a, b)| a.id() == b.id())
            && self.pending.is_none()
            && !self.queue.pending()
    }
    /// Idle changes only the next-preview choice. Playing changes this exact
    /// generation's future feed; the confirmed model stays unchanged until reply.
    pub fn toggle_preview_mute(&mut self) -> Code {
        if self.preview != Preview::Playing {
            return self
                .preview_level(self.selected_level_percent, u32::from(!self.selected_muted));
        }
        let n = match self.timely() {
            Ok(n) => n,
            Err(e) => return e,
        };
        if self.pending.is_some() || self.queue.pending() {
            return Code::Busy;
        }
        if !self.can_mute_playing() {
            return Code::Refused;
        }
        let generation = self.preview_generation.expect("current Play generation");
        let muted = !self.playback_muted.expect("current feed policy");
        let until = match self.deadline(n) {
            Ok(n) => n,
            Err(e) => return e,
        };
        if let Err(e) = self.request(MUTE_REQUEST, until, String::new(), None) {
            return e;
        }
        self.pending.as_mut().expect("mute request").preview_mute = Some((generation, muted));
        self.status = PREVIEW_MUTE_PENDING;
        Code::Ok
    }
    fn can_play(&self) -> bool {
        self.online
            && self.available
            && self.choose
            && self.policy_settled
            && self.stage == IDLE
            && self.pending.is_none()
            && !self.queue.pending()
            && !self.preview.active()
            && self
                .requested
                .is_some_and(|route| self.rows.iter().any(|r| r.id.id() == route.id()))
    }
    fn can_stop(&self) -> bool {
        self.online
            && !self.queue.metadata().is_some_and(|(kind, _)| kind == STOP)
            && !self
                .pending
                .as_ref()
                .is_some_and(|p| p.kind == STOP_REQUEST)
            && (self.preview.active() || self.pending.as_ref().is_some_and(|p| p.kind == PLAY))
    }
    pub fn preview_play(&mut self) -> Code {
        let n = match self.timely() {
            Ok(n) => n,
            Err(e) => return e,
        };
        if self.pending.is_some() || self.queue.pending() {
            return Code::Busy;
        }
        if !self.can_play() {
            return Code::Refused;
        }
        let until = match self.deadline(n) {
            Ok(n) => n,
            Err(e) => return e,
        };
        let route = self.requested.expect("selected logical route");
        let level = (self.selected_level_percent, self.selected_muted);
        if let Err(e) = self.request(PLAY, until, String::new(), None) {
            return e;
        }
        let pending = self.pending.as_mut().expect("play request");
        pending.preview_route = Some(route);
        pending.preview_level = Some(level);
        self.preview = Preview::Starting;
        self.status = PREVIEW_STARTING;
        Code::Ok
    }
    pub fn preview_stop(&mut self) -> Code {
        let n = match self.timely() {
            Ok(n) => n,
            Err(e) => return e,
        };
        if self.queue.pending() {
            if self.queue.metadata().is_some_and(|(kind, _)| kind != STOP) && self.can_stop() {
                self.fail(PREVIEW_CANCELLED);
                return Code::Ok;
            }
            return Code::Busy;
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.kind == STOP_REQUEST)
        {
            return Code::Busy;
        }
        if !self.can_stop() {
            return Code::Refused;
        }
        // A single transport cannot multiplex cleanup over an effect. Closing
        // the channel withdraws ownership, without claiming remote quiet/reap.
        if self.pending.as_ref().is_some_and(|p| p.kind != STATUS) {
            self.fail(PREVIEW_CANCELLED);
            return Code::Ok;
        }
        if self.pending.is_some() {
            return self.queue.offer(STOP, None, n);
        }
        let until = match self.deadline(n) {
            Ok(n) => n,
            Err(e) => return e,
        };
        self.request(STOP_REQUEST, until, protocol::preview_stop(), None)
            .map_or_else(
                |e| e,
                |()| {
                    self.preview = Preview::Retiring;
                    self.status = PREVIEW_RETIRING;
                    Code::Ok
                },
            )
    }
    pub fn choose(&mut self, route: &Route) -> Code {
        if route.id().is_none() {
            return Code::Invalid;
        }
        let n = match self.timely() {
            Ok(n) => n,
            Err(e) => return e,
        };
        if self.queue.pending() || !self.status_or_idle() {
            return Code::Busy;
        }
        if !self.can_choose() || !self.rows.iter().any(|r| r.id.id() == route.id()) {
            return Code::Refused;
        }
        if self.pending.is_some() {
            return self.queue.offer(ROUTE, Some(route), n);
        }
        let until = match self.deadline(n) {
            Ok(v) => v,
            Err(e) => return e,
        };
        self.request(SELECT, until, protocol::selection(route, 1000), None)
            .map_or_else(|e| e, |()| Code::Ok)
    }
    pub fn reconnect(&mut self) -> Code {
        if self.stage == CLOSED {
            return self.connect().map_or_else(|e| e, |()| Code::Ok);
        }
        let n = match self.timely() {
            Ok(n) => n,
            Err(e) => return e,
        };
        if self.queue.pending() || !self.status_or_idle() {
            return Code::Busy;
        }
        if !self.can_reconnect() {
            return Code::Refused;
        }
        if self.pending.is_some() {
            return self.queue.offer(RECONNECT, None, n);
        }
        let until = match self.deadline(n) {
            Ok(v) => v,
            Err(e) => return e,
        };
        self.request(RECON, until, protocol::reconnect(), None)
            .map_or_else(|e| e, |()| Code::Ok)
    }
    pub fn refresh(&mut self) -> Code {
        let n = match self.timely() {
            Ok(n) => n,
            Err(e) => return e,
        };
        if self.pending.is_some() {
            return Code::Busy;
        }
        if !self.online {
            return Code::Refused;
        }
        self.pages.clear();
        let until = match self.deadline(n) {
            Ok(v) => v,
            Err(e) => return e,
        };
        self.request(STATUS, until, protocol::status(0), None)
            .map_or_else(|e| e, |()| Code::Ok)
    }
    fn fresh(&self) -> Fresh {
        let mut routes = [Route::EMPTY; 32];
        for (out, row) in routes.iter_mut().zip(&self.rows) {
            *out = row.id;
        }
        Fresh {
            transport_online: u32::from(self.online),
            status_complete: u32::from(!self.pending.as_ref().is_some_and(|p| p.kind == STATUS)),
            available: u32::from(self.available),
            can_choose: u32::from(self.choose),
            can_reconnect: u32::from(self.reconnect),
            route_count: self.rows.len() as u32,
            routes,
        }
    }
    fn idle(&mut self) {
        self.pending = None;
        self.stage = IDLE;
        self.pages.clear();
    }
    fn dispatch(&mut self, n: u64) -> Result<(), Code> {
        let fresh = self.fresh();
        let cleanup = self.queue.metadata().is_some_and(|(kind, _)| kind == STOP);
        match self.queue.prepare(&fresh, n) {
            Ok(c) => self.request(
                if c.kind == ROUTE {
                    SELECT
                } else if c.kind == STOP {
                    STOP_REQUEST
                } else {
                    RECON
                },
                c.original_until_boottime_ns,
                if c.kind == ROUTE {
                    protocol::selection(&c.route, c.ttl_ms)
                } else if c.kind == STOP {
                    protocol::preview_stop()
                } else {
                    protocol::reconnect()
                },
                Some(c.token),
            ),
            Err(e @ (Code::Expired | Code::Refused)) if cleanup => {
                self.fail(REQUEST_EXPIRED);
                Err(e)
            }
            Err(Code::Empty | Code::Busy | Code::Expired | Code::Refused) => Ok(()),
            Err(e) => {
                self.fail(QUEUED_REFUSED);
                Err(e)
            }
        }
    }
    pub fn prepare(&mut self) -> Result<Packet, Code> {
        let n = self.timely()?;
        if self.stage != SENDING {
            return Err(Code::Empty);
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.queue_token.is_some())
        {
            let fresh = self.fresh();
            match self.queue.prepare(&fresh, n) {
                Ok(c) => {
                    let p = self.pending.as_mut().expect("sending request");
                    if p.queue_token != Some(c.token) {
                        self.fail(QUEUED_REFUSED);
                        return Err(Code::Refused);
                    }
                    if c.kind == ROUTE {
                        p.output = protocol::selection(&c.route, c.ttl_ms);
                    }
                }
                Err(e @ (Code::Expired | Code::Refused)) => {
                    if self
                        .pending
                        .as_ref()
                        .is_some_and(|p| p.kind == STOP_REQUEST)
                    {
                        self.fail(REQUEST_EXPIRED);
                    } else {
                        self.idle();
                    }
                    return Err(e);
                }
                Err(e) => {
                    self.fail(QUEUED_REFUSED);
                    return Err(e);
                }
            }
        }
        let p = self.pending.as_mut().expect("sending request");
        if p.kind == PLAY {
            let ttl = ((p.until - n) / 1_000_000) as u32;
            if ttl == 0 {
                self.fail(REQUEST_EXPIRED);
                return Err(Code::Expired);
            }
            let (percent, muted) = p.preview_level.expect("frozen play level");
            p.output = protocol::preview_play(
                &p.preview_route.expect("frozen play route"),
                ttl,
                p.until,
                percent,
                muted,
            );
        }
        if p.kind == MUTE_REQUEST {
            let (generation, muted) = p.preview_mute.expect("frozen mute intent");
            let ttl = ((p.until - n) / 1_000_000) as u32;
            if ttl == 0 {
                self.fail(REQUEST_EXPIRED);
                return Err(Code::Expired);
            }
            if self.preview_generation != Some(generation) || self.preview != Preview::Playing {
                self.fail(REQUEST_REFUSED);
                return Err(Code::Refused);
            }
            p.output = protocol::preview_mute(generation, muted, ttl, p.until);
        }
        let length = p.output.len();
        if length == 0 || length > 4096 {
            self.fail(REQUEST_REFUSED);
            return Err(Code::Refused);
        }
        let mut bytes = [0; 4096];
        bytes[..length].copy_from_slice(p.output.as_bytes());
        p.prepared_length = length as u32;
        Ok(Packet {
            connection_epoch: self.epoch,
            token: p.token,
            original_until_boottime_ns: p.until,
            length: length as u32,
            reserved: 0,
            bytes,
        })
    }
    pub fn sent(&mut self, epoch: u64, token: u64, outcome: u32, count: u32) -> Code {
        if epoch != self.epoch
            || self.stage != SENDING
            || !self
                .pending
                .as_ref()
                .is_some_and(|p| p.token == token && p.prepared_length > 0)
        {
            return Code::Stale;
        }
        if !(1..=4).contains(&outcome) || (outcome != 1 && count != 0) {
            return Code::Invalid;
        }
        let n = match self.timely() {
            Ok(n) => n,
            Err(e) => return e,
        };
        if outcome == 2 || outcome == 3 {
            return Code::Ok;
        }
        let p = self.pending.as_ref().expect("sending request");
        if outcome == 4 || count != p.prepared_length {
            self.fail(SEND_REFUSED);
            return Code::Refused;
        }
        if let Some(token) = p.queue_token
            && self.queue.sent(token, n) != Code::Ok
        {
            self.fail(REQUEST_EXPIRED);
            return Code::Expired;
        }
        let p = self.pending.as_mut().expect("sending request");
        p.output.clear();
        p.prepared_length = 0;
        self.stage = RECEIVING;
        if p.kind == SELECT {
            self.applied = None;
            self.status = OUTPUT_REQUESTED;
        }
        Code::Ok
    }
    fn apply(&mut self, s: State) {
        self.available = s.connected && s.available;
        self.requested = s.requested;
        self.applied = s.applied;
        self.policy_settled = s.policy_settled;
        self.preview = s.preview;
        self.preview_generation = s.preview_generation;
        self.playback_muted = s.preview_muted;
        self.status = if !self.available {
            UNAVAILABLE_INSPECT
        } else if s.preview == Preview::Starting {
            PREVIEW_STARTING
        } else if s.preview == Preview::Playing {
            PREVIEW_PLAYING
        } else if s.preview == Preview::Retiring {
            PREVIEW_RETIRING
        } else if s.preview == Preview::Completed {
            PREVIEW_COMPLETED
        } else if s.preview == Preview::Failed {
            PREVIEW_FAILED
        } else if s.pending {
            if self.requested.is_some() {
                APPLYING
            } else {
                CHECKING
            }
        } else if self.applied.is_some() {
            APPLIED
        } else if self.requested.is_some() && self.policy_settled {
            SELECTED_IDLE
        } else if self.requested.is_some() {
            INTERRUPTED
        } else if self.choose {
            CHOOSE_OUTPUT
        } else {
            ROUTE_UNAVAILABLE
        };
    }
    fn accept(&mut self, r: Reply) -> Result<(), Code> {
        let n = self.timely()?;
        let kind = self.pending.as_ref().expect("receiving request").kind;
        match r {
            Reply::Denied | Reply::Unavailable => {
                if kind == STATUS && self.queue.metadata().is_some_and(|(kind, _)| kind == STOP) {
                    // The peer declined inspection, not ownership cleanup. Native
                    // close withdraws the accepted Stop instead of discarding it.
                    self.fail(PREVIEW_CANCELLED);
                    return Ok(());
                }
                if kind == PLAY || kind == STOP_REQUEST || kind == MUTE_REQUEST {
                    self.fail(if matches!(r, Reply::Denied) {
                        DENIED
                    } else {
                        UNAVAILABLE
                    });
                    return Ok(());
                }
                self.queue.cancel();
                self.choose = false;
                self.reconnect = false;
                self.applied = None;
                self.status = if matches!(r, Reply::Denied) {
                    DENIED
                } else {
                    UNAVAILABLE
                };
                self.idle();
            }
            Reply::Ok if kind == RECON => {
                self.idle();
                let until = self.deadline(n)?;
                self.request(STATUS, until, protocol::status(0), None)?;
                return Ok(());
            }
            Reply::Selected(s) if kind == SELECT => {
                self.apply(s);
                self.idle();
            }
            Reply::Preview(s) if kind == MUTE_REQUEST => {
                let (generation, muted) = self
                    .pending
                    .as_ref()
                    .expect("mute request")
                    .preview_mute
                    .expect("frozen mute intent");
                if s.preview_generation != Some(generation)
                    || s.preview_muted != Some(muted)
                    || s.preview != Preview::Playing
                {
                    return Err(Code::Refused);
                }
                self.apply(s);
                self.idle();
            }
            Reply::Preview(s) if kind == PLAY || kind == STOP_REQUEST => {
                self.apply(s);
                self.idle();
            }
            Reply::Status(p) if kind == STATUS => {
                let offset = self.pages.len();
                if offset + p.rows.len() > 32 {
                    return Err(Code::Refused);
                }
                let mut previous = self.pages.last().map(|r| r.id);
                for row in &p.rows {
                    if previous.is_some_and(|old| old.id() >= row.id.id()) {
                        return Err(Code::Refused);
                    }
                    previous = Some(row.id);
                }
                // Finish bounded allocation/collection before the final authority
                // check; an old parsed page cannot publish or dispatch after expiry.
                let mut collected = self.pages.clone();
                collected.extend(p.rows);
                if let Some(next) = p.next {
                    if next != offset + 8 || collected.len() - offset != 8 || next >= 32 {
                        return Err(Code::Refused);
                    }
                    self.timely()?;
                    self.pages = collected;
                    let until = self.pending.as_ref().expect("status request").until;
                    self.request(STATUS, until, protocol::status(next), None)?;
                    return Ok(());
                }
                self.timely()?;
                self.rows = collected;
                self.choose = p.choose;
                self.reconnect = p.reconnect;
                self.apply(p.state);
                self.idle();
            }
            _ => return Err(Code::Refused),
        }
        let now = self.sample()?;
        self.dispatch(now)
    }
    pub fn reply(&mut self, epoch: u64, bytes: &[u8]) -> Code {
        if epoch != self.epoch {
            return Code::Stale;
        }
        if self.stage != RECEIVING {
            self.fail(REPLY_OUTSIDE_REQUEST);
            return Code::Refused;
        }
        if let Err(e) = self.timely() {
            return e;
        }
        let parsed = protocol::parse(bytes);
        let Some(r) = parsed else {
            self.fail(REPLY_REFUSED);
            return Code::Refused;
        };
        match self.accept(r) {
            Ok(()) => Code::Ok,
            Err(Code::Expired) => Code::Expired,
            Err(e) => {
                if self.stage != CLOSED {
                    self.fail(REPLY_REFUSED);
                }
                e
            }
        }
    }
    pub fn disconnected(&mut self, epoch: u64, reason: u32) -> Code {
        if epoch != self.epoch {
            return Code::Stale;
        }
        if !(13..=27).contains(&reason) {
            return Code::Invalid;
        }
        if self.stage == CLOSED {
            return Code::Empty;
        }
        self.fail(reason);
        Code::Ok
    }
    pub fn tick(&mut self) -> Code {
        self.timely().map_or_else(|e| e, |_| Code::Ok)
    }
    pub fn view(&self) -> View {
        const EMPTY: ViewRow = ViewRow {
            id: Route::EMPTY,
            label: Label {
                length: 0,
                bytes: [0; 96],
            },
        };
        let mut routes = [EMPTY; 32];
        for (out, row) in routes.iter_mut().zip(&self.rows) {
            out.id = row.id;
            out.label.length = row.label.len() as u32;
            out.label.bytes[..row.label.len()].copy_from_slice(row.label.as_bytes());
        }
        let (queued_kind, queued_until) = self.queue.metadata().unwrap_or((0, 0));
        View {
            connection_epoch: self.epoch,
            request_until_boottime_ns: self.pending.as_ref().map_or(0, |p| p.until),
            queued_until_boottime_ns: queued_until,
            stage: self.stage,
            request_kind: self.pending.as_ref().map_or(0, |p| p.kind),
            status: self.status,
            online: u32::from(self.online),
            available: u32::from(self.available),
            can_choose: u32::from(self.can_choose()),
            can_reconnect: u32::from(self.can_reconnect()),
            route_count: self.rows.len() as u32,
            requested_present: u32::from(self.requested.is_some()),
            applied_present: u32::from(self.applied.is_some()),
            queued_kind,
            reserved: 0,
            requested: self.requested.unwrap_or(Route::EMPTY),
            applied: self.applied.unwrap_or(Route::EMPTY),
            routes,
            policy_settled: u32::from(self.policy_settled),
            preview: self.preview as u32,
            can_play: u32::from(self.can_play()),
            can_stop: u32::from(self.can_stop()),
            selected_level_percent: self.selected_level_percent,
            selected_muted: u32::from(self.selected_muted),
            can_adjust_preview: u32::from(self.can_adjust_preview()),
            preview_reserved: 0,
            preview_generation: self.preview_generation.unwrap_or(0),
            playback_muted: u32::from(self.playback_muted.unwrap_or(false)),
            playback_mute_known: u32::from(self.playback_muted.is_some()),
            can_toggle_mute: u32::from(self.can_adjust_preview() || self.can_mute_playing()),
            mute_checked: u32::from(if self.preview == Preview::Playing {
                self.playback_muted.unwrap_or(false)
            } else {
                self.selected_muted
            }),
            mute_is_live: u32::from(self.preview == Preview::Playing),
            mute_reserved: 0,
        }
    }
}
