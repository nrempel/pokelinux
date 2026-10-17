#![forbid(unsafe_code)]
use crate::wire::{Code, Command, Fresh, RECONNECT, ROUTE, Route};
#[derive(Clone, Copy)]
struct Intent {
    token: u64,
    until: u64,
    kind: u32,
    route: Route,
}
pub struct Queue {
    intent: Option<Intent>,
    next_token: u64,
}
impl Queue {
    pub fn new() -> Self {
        Self {
            intent: None,
            next_token: 1,
        }
    }
    pub fn pending(&self) -> bool {
        self.intent.is_some()
    }
    pub fn offer(&mut self, kind: u32, route: Option<&Route>, now: u64) -> Code {
        if self.intent.is_some() {
            return Code::Busy;
        }
        let route = match (kind, route) {
            (ROUTE, Some(route)) if route.id().is_some() => *route,
            (RECONNECT, None) => Route::EMPTY,
            _ => return Code::Invalid,
        };
        let Some(until) = now
            .checked_add(1_000_000_000)
            .filter(|v| *v <= i64::MAX as u64)
        else {
            return Code::Invalid;
        };
        let Some(next) = self.next_token.checked_add(1) else {
            return Code::Invalid;
        };
        self.intent = Some(Intent {
            token: self.next_token,
            until,
            kind,
            route,
        });
        self.next_token = next;
        Code::Ok
    }
    pub fn prepare(&mut self, fresh: &Fresh, now: u64) -> Result<Command, Code> {
        let intent = self.intent.ok_or(Code::Empty)?;
        // Expiry is authoritative even while an incomplete status is held.
        if now >= intent.until {
            self.cancel();
            return Err(Code::Expired);
        }
        if !fresh.valid() {
            self.cancel();
            return Err(Code::Invalid);
        }
        if fresh.transport_online == 0 {
            self.cancel();
            return Err(Code::Refused);
        }
        if fresh.status_complete == 0 {
            return Err(Code::Busy);
        }
        let mut ttl_ms = 0;
        if intent.kind == ROUTE {
            let member = fresh.routes[..fresh.route_count as usize]
                .iter()
                .any(|r| r.id() == intent.route.id());
            if fresh.available == 0 || fresh.can_choose == 0 || !member {
                self.cancel();
                return Err(Code::Refused);
            }
            ttl_ms = ((intent.until - now) / 1_000_000) as u32;
            if ttl_ms == 0 {
                self.cancel();
                return Err(Code::Expired);
            }
        } else if fresh.can_reconnect == 0 {
            self.cancel();
            return Err(Code::Refused);
        }
        Ok(Command {
            token: intent.token,
            original_until_boottime_ns: intent.until,
            kind: intent.kind,
            ttl_ms,
            route: intent.route,
        })
    }
    pub fn sent(&mut self, token: u64, now: u64) -> Code {
        let Some(intent) = self.intent else {
            return Code::Stale;
        };
        if intent.token != token {
            return Code::Stale;
        }
        self.cancel();
        if now >= intent.until {
            Code::Expired
        } else {
            Code::Ok
        }
    }
    pub fn cancel(&mut self) {
        self.intent = None;
    }
}
