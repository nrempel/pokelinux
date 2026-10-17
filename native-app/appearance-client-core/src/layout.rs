#![forbid(unsafe_code)]

pub const OK: i32 = 0;
pub const EMPTY: i32 = 1;
pub const WAIT: i32 = 2;
pub const REFUSED: i32 = 3;
pub const EXPIRED: i32 = 4;
pub const INVALID: i32 = 5;
pub const STALE: i32 = 6;
pub const IDLE: u32 = 0;
pub const CONNECTING: u32 = 1;
pub const SENDING: u32 = 2;
pub const RECEIVING: u32 = 3;
pub const CLOSING: u32 = 4;
pub const STOPPED: u32 = 5;
pub const CONNECT: u32 = 1;
pub const FINISH_CONNECT: u32 = 2;
pub const SEND: u32 = 3;
pub const RECEIVE: u32 = 4;
pub const CLOSE: u32 = 5;
pub const READ_READY: u32 = 1;
pub const WRITE_READY: u32 = 2;
pub const CONNECTED: u32 = 1;
pub const CONNECT_PENDING: u32 = 2;
pub const CONNECT_FAILED_CLOSED: u32 = 3;
pub const SENT: u32 = 4;
pub const WOULD_BLOCK: u32 = 5;
pub const INTERRUPTED: u32 = 6;
pub const IO_FAILED: u32 = 7;
pub const CLOSED: u32 = 8;
pub const QUERY: &[u8; 17] = b"{\"op\":\"snapshot\"}";

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Rgba16 {
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub a: u16,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct View {
    pub connection_generation: u64,
    pub request_token: u64,
    pub original_until_boottime_ns: u64,
    pub palette_until_boottime_ns: u64,
    pub change_sequence: u64,
    pub stage: u32,
    pub valid: u32,
    pub has_snapshot: u32,
    pub poll_interval_ms: u32,
    pub request_timer_ms: u32,
    pub expiry_timer_ms: u32,
    pub read_enabled: u32,
    pub write_enabled: u32,
    pub owner: [u8; 16],
    pub revision: u32,
    pub base_size: u32,
    pub font_scale: f64,
    pub palette: [Rgba16; 9],
    pub font_tokens: [u32; 8],
    pub spacing_tokens: [u32; 6],
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Step {
    pub connection_generation: u64,
    pub request_token: u64,
    pub effect_token: u64,
    pub original_until_boottime_ns: u64,
    pub action: u32,
    pub packet_length: u32,
    pub packet: [u8; 32],
}
const _: () = assert!(std::mem::size_of::<View>() == 232);
const _: () = assert!(std::mem::size_of::<Step>() == 72);
