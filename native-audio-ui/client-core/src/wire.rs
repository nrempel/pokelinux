#![forbid(unsafe_code)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Route {
    pub length: u32,
    pub bytes: [u8; 64],
}
impl Route {
    pub const EMPTY: Self = Self {
        length: 0,
        bytes: [0; 64],
    };
    pub fn id(&self) -> Option<&[u8]> {
        let bytes = self.bytes.get(..self.length as usize)?;
        (!bytes.is_empty()
            && bytes
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(b)))
        .then_some(bytes)
    }
}
#[repr(C)]
pub struct Fresh {
    pub transport_online: u32,
    pub status_complete: u32,
    pub available: u32,
    pub can_choose: u32,
    pub can_reconnect: u32,
    pub route_count: u32,
    pub routes: [Route; 32],
}
impl Fresh {
    pub fn valid(&self) -> bool {
        if [
            self.transport_online,
            self.status_complete,
            self.available,
            self.can_choose,
            self.can_reconnect,
        ]
        .iter()
        .any(|v| *v > 1)
        {
            return false;
        }
        let Some(routes) = self.routes.get(..self.route_count as usize) else {
            return false;
        };
        let mut previous: Option<&[u8]> = None;
        for route in routes {
            let Some(id) = route.id() else {
                return false;
            };
            if previous.is_some_and(|old| old >= id) {
                return false;
            }
            previous = Some(id);
        }
        true
    }
}
#[repr(C)]
pub struct Command {
    pub token: u64,
    pub original_until_boottime_ns: u64,
    pub kind: u32,
    pub ttl_ms: u32,
    pub route: Route,
}
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Ok = 0,
    Empty = 1,
    Busy = 2,
    Refused = 3,
    Expired = 4,
    Invalid = 5,
    Stale = 6,
}
pub const ROUTE: u32 = 1;
pub const RECONNECT: u32 = 2;

pub const STOP: u32 = 3;
