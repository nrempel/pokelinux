//! Credential-free fixed fixture observations; no result/receipt authority.
use crate::owner::{Snapshot, WaitReceipt};

pub const SIZE: usize = 48;
pub const MAGIC: &[u8; 8] = b"PKSCOPE2";
pub const STATE: u32 = 1;
pub const DIRECT_WAIT: u32 = 2;
pub const ADOPTED_WAIT: u32 = 3;
pub const CLOSED: u32 = 4;
pub const PARENT: u32 = 5;

pub enum Decoded {
    State(Snapshot, bool),
    Wait(WaitReceipt),
}

pub fn decode(bytes: &[u8; SIZE]) -> std::io::Result<Decoded> {
    let invalid = || std::io::Error::from(std::io::ErrorKind::InvalidData);
    let value = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    if &bytes[..8] != MAGIC || bytes[40..] != [0; 8] {
        return Err(invalid());
    }
    let event = value(8);
    if matches!(event, DIRECT_WAIT | ADOPTED_WAIT) {
        if value(12) != 0
            || value(24) != 0
            || value(28) != 0
            || !(2..=i32::MAX as u32).contains(&value(16))
        {
            return Err(invalid());
        }
        return Ok(Decoded::Wait(WaitReceipt {
            pid: value(16),
            raw_status: value(20) as i32,
            worker: event == DIRECT_WAIT,
        }));
    }
    if !matches!(event, STATE | CLOSED)
        || value(12) & !0x1fff != 0
        || !(2..=i32::MAX as u32).contains(&value(16))
    {
        return Err(invalid());
    }
    let flags = value(12);
    let retired = match value(28) {
        0 => None,
        1 => Some(crate::owner::Retired::Cancel),
        2 => Some(crate::owner::Retired::Stop),
        3 => Some(crate::owner::Retired::Deadline),
        4 => Some(crate::owner::Retired::ParentGone),
        5 => Some(crate::owner::Retired::Protocol),
        6 => Some(crate::owner::Retired::Setup),
        7 => Some(crate::owner::Retired::Observation),
        _ => return Err(invalid()),
    };
    let snapshot = Snapshot {
        worker_pid: value(16),
        occupied: flags & 1 != 0,
        retired,
        candidate_buffered: flags & 2 != 0,
        worker_ready: flags & 4 != 0,
        worker_stdout_eof: flags & 1024 != 0,
        worker_wait: (flags & 8 != 0).then_some(value(20) as i32),
        adopted_waits: value(24),
        final_echild: flags & 16 != 0,
        populated: flags & 32 != 0,
        cleanup_fault: flags & 64 != 0,
        start_released: flags & 128 != 0,
        direct_kill_attempted: flags & 256 != 0,
        direct_kill_returned: flags & 512 != 0,
        scope_kill_attempted: flags & 2048 != 0,
        scope_kill_returned: flags & 4096 != 0,
    };
    if snapshot.scope_kill_returned && !snapshot.scope_kill_attempted
        || snapshot.scope_kill_attempted && snapshot.retired.is_none()
        || snapshot.direct_kill_returned && !snapshot.direct_kill_attempted
        || snapshot.retired.is_some() && snapshot.candidate_buffered
        || event == CLOSED && snapshot.occupied
        || (event == CLOSED || !snapshot.occupied)
            && (snapshot.worker_wait.is_none()
                || !snapshot.final_echild
                || !snapshot.worker_stdout_eof
                || snapshot.populated
                || snapshot.cleanup_fault)
    {
        return Err(invalid());
    }
    Ok(Decoded::State(snapshot, event == CLOSED))
}

pub fn parent(snapshot: crate::parent::Snapshot) -> [u8; SIZE] {
    let mut result = [0; SIZE];
    result[..8].copy_from_slice(MAGIC);
    put(&mut result, 8, PARENT);
    put(
        &mut result,
        12,
        u32::from(snapshot.occupied)
            | u32::from(snapshot.supervisor_wait.is_some()) << 3
            | u32::from(snapshot.scope_reported_closed) << 4
            | u32::from(snapshot.stdout_eof) << 5
            | u32::from(snapshot.cleanup_fault) << 6,
    );
    put(&mut result, 16, snapshot.supervisor_pid);
    put(
        &mut result,
        20,
        snapshot.supervisor_wait.unwrap_or(0) as u32,
    );
    put(
        &mut result,
        28,
        snapshot.retired.map_or(0, |reason| reason as u32),
    );
    result[32..40].copy_from_slice(&crate::boottime_ns().to_le_bytes());
    result
}

fn put(record: &mut [u8; SIZE], at: usize, value: u32) {
    record[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn state(snapshot: Snapshot, event: u32) -> [u8; SIZE] {
    let mut result = [0; SIZE];
    result[..8].copy_from_slice(MAGIC);
    put(&mut result, 8, event);
    let flags = u32::from(snapshot.occupied)
        | u32::from(snapshot.candidate_buffered) << 1
        | u32::from(snapshot.worker_ready) << 2
        | u32::from(snapshot.worker_wait.is_some()) << 3
        | u32::from(snapshot.final_echild) << 4
        | u32::from(snapshot.populated) << 5
        | u32::from(snapshot.cleanup_fault) << 6
        | u32::from(snapshot.start_released) << 7
        | u32::from(snapshot.direct_kill_attempted) << 8
        | u32::from(snapshot.direct_kill_returned) << 9
        | u32::from(snapshot.worker_stdout_eof) << 10
        | u32::from(snapshot.scope_kill_attempted) << 11
        | u32::from(snapshot.scope_kill_returned) << 12;
    put(&mut result, 12, flags);
    put(&mut result, 16, snapshot.worker_pid);
    put(&mut result, 20, snapshot.worker_wait.unwrap_or(0) as u32);
    put(&mut result, 24, snapshot.adopted_waits);
    put(
        &mut result,
        28,
        snapshot.retired.map_or(0, |reason| reason as u32),
    );
    result[32..40].copy_from_slice(&crate::boottime_ns().to_le_bytes());
    // Bytes 40..48 are reserved zero, checked by the independent decoder.
    result
}

pub fn wait(receipt: WaitReceipt) -> [u8; SIZE] {
    let mut result = [0; SIZE];
    result[..8].copy_from_slice(MAGIC);
    put(
        &mut result,
        8,
        if receipt.worker {
            DIRECT_WAIT
        } else {
            ADOPTED_WAIT
        },
    );
    put(&mut result, 16, receipt.pid);
    put(&mut result, 20, receipt.raw_status as u32);
    result[32..40].copy_from_slice(&crate::boottime_ns().to_le_bytes());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closed() -> Snapshot {
        Snapshot {
            worker_pid: 100,
            occupied: false,
            retired: Some(crate::owner::Retired::Cancel),
            candidate_buffered: false,
            worker_ready: true,
            worker_stdout_eof: true,
            worker_wait: Some(9),
            adopted_waits: 2,
            final_echild: true,
            populated: false,
            cleanup_fault: false,
            start_released: true,
            direct_kill_attempted: true,
            direct_kill_returned: true,
            scope_kill_attempted: true,
            scope_kill_returned: true,
        }
    }

    #[test]
    fn terminal_record_requires_every_scope_barrier_and_no_reserved_data() {
        let valid = closed();
        assert!(decode(&state(valid, CLOSED)).is_ok());
        for invalid in [
            Snapshot {
                occupied: true,
                ..valid
            },
            Snapshot {
                populated: true,
                ..valid
            },
            Snapshot {
                final_echild: false,
                ..valid
            },
            Snapshot {
                worker_wait: None,
                ..valid
            },
            Snapshot {
                cleanup_fault: true,
                ..valid
            },
            Snapshot {
                candidate_buffered: true,
                ..valid
            },
        ] {
            assert!(decode(&state(invalid, CLOSED)).is_err());
        }
        let mut bytes = state(valid, CLOSED);
        bytes[47] = 1;
        assert!(decode(&bytes).is_err());
        let mut bytes = state(valid, CLOSED);
        bytes[13] |= 0x80;
        assert!(decode(&bytes).is_err());
    }
}
