//! Independent fixture-only code105 capture; no production radio/codec types.
use rsbinder::{Parcel, Result, Status, StatusCode};
pub const QUERY: u32 = 127;
pub const RESET: u32 = 128;
pub const VECTOR_ID: &str = "empty-pcscf-wire2-fixed-time";
#[derive(Default)]
pub struct Capture {
    epoch: i32,
    count: i32,
    total: i32,
    packet: Option<Packet>,
    query_logged: bool,
}
struct Packet {
    args: [i32; 4],
    owner_pid: i32,
    owner_uid: i32,
    calling_pid: i32,
    calling_uid: i32,
    payload: Vec<u8>,
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 15) as usize] as char);
    }
    result
}
fn expected() -> Result<Vec<u8>> {
    let row = include_str!("../../../../native-ims/tests/fixtures/shannon-add-pdn.tsv")
        .lines()
        .find(|line| line.split('\t').next() == Some(VECTOR_ID))
        .ok_or(StatusCode::BadValue)?;
    let fields: Vec<_> = row.split('\t').collect();
    if fields.len() != 6 || fields[1..5] != ["2", "0", "0", "0"] || fields[5].len() != 1326 {
        return Err(StatusCode::BadValue);
    }
    fields[5]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| StatusCode::BadValue)?;
            u8::from_str_radix(text, 16).map_err(|_| StatusCode::BadValue)
        })
        .collect()
}
impl Capture {
    pub fn record(
        &mut self,
        args: [i32; 4],
        payload: Vec<u8>,
        owner: [i32; 2],
        calling: [i32; 2],
    ) -> Result<()> {
        // A third attempt leaves a sticky bounded sentinel, never a successful
        // two-packet snapshot that hides duplicate oneway traffic.
        self.count = self.count.saturating_add(1).min(3);
        self.total = self.total.saturating_add(1).min(3);
        if payload.len() > 2048 || self.total > 2 {
            println!(
                "ADD-PDN-CAPTURE-REFUSED epoch={} total={} count={} owner_pid={} owner_uid={} calling_pid={} calling_uid={} bytes={}",
                self.epoch,
                self.total,
                self.count,
                owner[0],
                owner[1],
                calling[0],
                calling[1],
                payload.len()
            );
            std::io::Write::flush(&mut std::io::stdout())?;
            return Err(StatusCode::BadValue);
        }
        let exact = args == [1, 42, 105, 1]
            && payload == expected()?
            && owner[0] > 1
            && owner[1] == 1000
            && calling == [0, 1000];
        println!(
            "ADD-PDN-CAPTURE epoch={} total={} owner_pid={} owner_uid={} calling_pid={} calling_uid={} serial={} client_id={} request={} slot={} bytes={} exact={} payload_hex={}",
            self.epoch,
            self.total,
            owner[0],
            owner[1],
            calling[0],
            calling[1],
            args[0],
            args[1],
            args[2],
            args[3],
            payload.len(),
            exact,
            hex(&payload)
        );
        std::io::Write::flush(&mut std::io::stdout())?;
        self.packet = Some(Packet {
            args,
            owner_pid: owner[0],
            owner_uid: owner[1],
            calling_pid: calling[0],
            calling_uid: calling[1],
            payload,
        });
        Ok(())
    }
    /// Clear only the authored capture table after its one explicit preflight.
    /// This touches no radio owner, registration, real operation or retry authority.
    pub fn reset_preflight(&mut self, owner: [i32; 2], calling: [i32; 2]) -> Result<()> {
        let packet = self.packet.as_ref().ok_or(StatusCode::BadValue)?;
        if self.epoch != 0
            || self.count != 1
            || self.total != 1
            || packet.args != [1, 42, 105, 1]
            || packet.payload != expected()?
            || packet.owner_pid != owner[0]
            || packet.owner_uid != owner[1]
            || packet.calling_pid != 0
            || packet.calling_uid != 1000
            || calling != owner
        {
            return Err(StatusCode::BadValue);
        }
        println!(
            "ADD-PDN-CAPTURE-RESET owner_pid={} owner_uid={} query_pid={} query_uid={} fixture_only=true previous_epoch=0 previous_count=1 total=1 new_epoch=1 new_count=0",
            owner[0], owner[1], calling[0], calling[1]
        );
        std::io::Write::flush(&mut std::io::stdout())?;
        self.epoch = 1;
        self.count = 0;
        self.packet = None;
        self.query_logged = false;
        Ok(())
    }
    pub fn query(&mut self, reply: &mut Parcel, owner: [i32; 2], calling: [i32; 2]) -> Result<()> {
        let expected = expected()?;
        if !self.query_logged {
            println!(
                "ADD-PDN-CAPTURE-READY epoch={} total={} owner_pid={} owner_uid={} query_pid={} query_uid={} expected_serial=1 expected_client_id=42 expected_request=105 expected_slot=1 expected_bytes={} capture_count={}",
                self.epoch,
                self.total,
                owner[0],
                owner[1],
                calling[0],
                calling[1],
                expected.len(),
                self.count
            );
            std::io::Write::flush(&mut std::io::stdout())?;
            self.query_logged = true;
        }
        reply.write(&Status::from(StatusCode::Ok))?;
        reply.write_i32(1)?; // Fixed capture contract version, never a modem receipt.
        reply.write_i32(self.epoch)?;
        reply.write_i32(self.count)?;
        reply.write_i32(self.total)?;
        let args = self.packet.as_ref().map_or([0; 4], |packet| packet.args);
        for value in args {
            reply.write_i32(value)?;
        }
        let identities = self
            .packet
            .as_ref()
            .map_or([owner[0], owner[1], 0, 0], |p| {
                [p.owner_pid, p.owner_uid, p.calling_pid, p.calling_uid]
            });
        for value in identities {
            reply.write_i32(value)?;
        }
        let payload = self
            .packet
            .as_ref()
            .map_or(&[][..], |packet| packet.payload.as_slice());
        reply.write(payload)
    }
}
