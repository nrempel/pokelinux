//! Authored public fixture controls, conditional on separate original routing qualification.
//! No production radio types, decoder calls, vendor source or modem semantics.
pub const CANDIDATE_MESSAGE_ID: i32 = 3012;
pub const VALID: u32 = 114;
pub const WRONG_SLOT: u32 = 115;
pub const WRONG_ID: u32 = 116;
pub const FRAGMENT: u32 = 117;
pub const TRUNCATED: u32 = 118;
pub const TRAILING: u32 = 119;
pub const INSTALLED_SLOT1: u32 = 120;
pub const STATE_CONNECTED: u32 = 121;
pub const STATE_DISCONNECTED: u32 = 122;
pub const STATE_UNKNOWN: u32 = 123;
pub const STATE_UNQUALIFIED: u32 = 124;
pub const STATE_MALFORMED: u32 = 125;
pub const STATE_WRONG_SLOT: u32 = 126;

pub fn valid() -> Vec<u8> {
    let mut bytes = vec![255, 128];
    bytes.extend(i32::MIN.to_le_bytes());
    bytes.extend(u16::MAX.to_le_bytes());
    bytes.extend(9u16.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(b"sip:routeok");
    bytes.extend(0x11223344i32.to_le_bytes());
    bytes.extend(0x0102030405060708i64.to_le_bytes());
    bytes.extend(0x3344u16.to_le_bytes());
    bytes
}
pub fn callback(control: u32) -> Option<(i32, i32, bool, Vec<u8>)> {
    let mut bytes = valid();
    let mut message = CANDIDATE_MESSAGE_ID;
    let mut slot = 0;
    let fragment = control == FRAGMENT;
    match control {
        VALID | FRAGMENT => {}
        WRONG_SLOT | INSTALLED_SLOT1 => slot = 1,
        WRONG_ID => message += 1,
        TRUNCATED => bytes.truncate(11),
        TRAILING => bytes.push(0x7e),
        STATE_CONNECTED..=STATE_WRONG_SLOT => {
            bytes = vec![0; 12];
            bytes[0] = match control {
                STATE_DISCONNECTED => 1,
                STATE_UNKNOWN => 255,
                _ => 2,
            };
            slot = if control == STATE_WRONG_SLOT { 0 } else { 1 };
            if control == STATE_UNQUALIFIED {
                bytes[1] = 1;
            }
            if control == STATE_MALFORMED {
                bytes.truncate(11);
            }
        }
        _ => return None,
    }
    Some((message, slot, fragment, bytes))
}
