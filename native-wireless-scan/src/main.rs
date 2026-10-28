//! Bounded one-shot worker. The supervising caller owns wait, output EOF and Stop.
use pokelinux_wireless_scan::{
    Associated, AssociationRequest, Completed, DisconnectRequest, Disconnected, Error, Request,
    associate_open_once, disconnect_once, scan_once,
};
use std::process::ExitCode;

fn decimal<T: std::str::FromStr>(value: &str) -> Option<T> {
    if value.is_empty() || !value.bytes().all(|v| v.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}
enum Operation {
    Scan(Request),
    Associate(AssociationRequest),
    Disconnect(DisconnectRequest),
}
fn bytes(value: &str) -> Option<Vec<u8>> {
    if value.is_empty()
        || value.len() > 64
        || !value.len().is_multiple_of(2)
        || !value
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
    {
        return None;
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let part = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(part, 16).ok()
        })
        .collect()
}
fn request() -> Option<Operation> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let first = args.first()?.as_str();
    if first == "associate-open" && args.len() == 6 {
        return Some(Operation::Associate(AssociationRequest {
            ifindex: decimal(&args[1])?,
            frequency_mhz: decimal(&args[2])?,
            bssid: bytes(&args[3])?.try_into().ok()?,
            ssid: bytes(&args[4])?,
            original_until_ns: decimal(&args[5])?,
        }));
    }
    if first == "disconnect" && args.len() == 3 {
        return Some(Operation::Disconnect(DisconnectRequest {
            ifindex: decimal(&args[1])?,
            original_until_ns: decimal(&args[2])?,
        }));
    }
    if args.len() != 3 {
        return None;
    }
    Some(Operation::Scan(Request {
        ifindex: decimal(&args[0])?,
        frequency_mhz: decimal(&args[1])?,
        original_until_ns: decimal(&args[2])?,
    }))
}
fn hex(value: &[u8]) -> String {
    value.iter().map(|v| format!("{v:02x}")).collect()
}
fn associated(value: Associated) -> String {
    format!(
        "{{\"schema\":1,\"status\":\"associated\",\"operation\":\"associate-open\",\"ifindex\":{},\"frequency_mhz\":{},\"original_until_ns\":{},\"wiphy\":{},\"wdev\":{},\"bssid_hex\":\"{}\",\"ssid_hex\":\"{}\",\"started_boottime_ns\":{},\"completed_boottime_ns\":{}}}",
        value.request.ifindex,
        value.request.frequency_mhz,
        value.request.original_until_ns,
        value.wiphy,
        value.wdev,
        hex(&value.request.bssid),
        hex(&value.request.ssid),
        value.started_boottime_ns,
        value.completed_boottime_ns
    )
}
fn disconnected(value: Disconnected) -> String {
    format!(
        "{{\"schema\":1,\"status\":\"disconnected\",\"operation\":\"disconnect\",\"ifindex\":{},\"original_until_ns\":{},\"wiphy\":{},\"wdev\":{},\"started_boottime_ns\":{},\"completed_boottime_ns\":{}}}",
        value.request.ifindex,
        value.request.original_until_ns,
        value.wiphy,
        value.wdev,
        value.started_boottime_ns,
        value.completed_boottime_ns
    )
}

fn completed(value: Completed) -> String {
    let rows = value.bss.iter().map(|b| format!(
        "{{\"bssid_hex\":\"{}\",\"ssid_hex\":\"{}\",\"frequency_mhz\":{},\"last_seen_boottime_ns\":{}}}",
        b.bssid_hex,b.ssid_hex,b.frequency_mhz,b.last_seen_boottime_ns)).collect::<Vec<_>>().join(",");
    format!(
        "{{\"schema\":1,\"status\":\"completed\",\"ifindex\":{},\"frequency_mhz\":{},\"original_until_ns\":{},\"wiphy\":{},\"wdev\":{},\"started_boottime_ns\":{},\"completed_boottime_ns\":{},\"bss\":[{}]}}",
        value.request.ifindex,
        value.request.frequency_mhz,
        value.request.original_until_ns,
        value.wiphy,
        value.wdev,
        value.started_boottime_ns,
        value.completed_boottime_ns,
        rows
    )
}
fn before(until: u64) -> bool {
    let v = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    let now = u64::try_from(v.tv_sec).expect("nonnegative BOOTTIME") * 1_000_000_000
        + u64::try_from(v.tv_nsec).expect("nonnegative BOOTTIME nanoseconds");
    now < until
}
fn main() -> ExitCode {
    let Some(operation) = request() else {
        println!(
            "{{\"schema\":1,\"status\":\"invalid_request\",\"reason\":\"invalid_arguments\"}}"
        );
        return ExitCode::from(64);
    };
    let (result, until, context) = match operation {
        Operation::Scan(request) => (
            scan_once(request).map(completed),
            request.original_until_ns,
            format!(
                "\"ifindex\":{},\"frequency_mhz\":{},\"original_until_ns\":{}",
                request.ifindex, request.frequency_mhz, request.original_until_ns
            ),
        ),
        Operation::Associate(request) => {
            let until = request.original_until_ns;
            let context = format!(
                "\"operation\":\"associate-open\",\"ifindex\":{},\"frequency_mhz\":{},\"original_until_ns\":{}",
                request.ifindex, request.frequency_mhz, until
            );
            (associate_open_once(request).map(associated), until, context)
        }
        Operation::Disconnect(request) => (
            disconnect_once(request).map(disconnected),
            request.original_until_ns,
            format!(
                "\"operation\":\"disconnect\",\"ifindex\":{},\"original_until_ns\":{}",
                request.ifindex, request.original_until_ns
            ),
        ),
    };
    let (status, reason, code) = match result {
        Ok(text) if text.len() <= 32768 && before(until) => {
            println!("{text}");
            return ExitCode::SUCCESS;
        }
        Ok(_) => (
            "unconfirmed",
            "publication_expired_or_oversized".to_string(),
            70,
        ),
        Err(Error::Unavailable) => ("unavailable", "unsupported".to_string(), 2),
        Err(Error::InvalidRequest) => ("invalid_request", "invalid_request".to_string(), 64),
        Err(Error::Failed(cause)) => ("failed", format!("{cause:?}"), 70),
        Err(Error::Uncertain(cause)) => ("uncertain", format!("{cause:?}"), 70),
    };
    println!("{{\"schema\":1,\"status\":\"{status}\",\"reason\":\"{reason}\",{context}}}");
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_bytes_are_bounded_exact_lowercase_hex() {
        assert_eq!(bytes("00ff80"), Some(vec![0, 255, 128]));
        for value in ["", "0", "0x02", "AA", "00 02"] {
            assert_eq!(bytes(value), None);
        }
        assert!(bytes(&"00".repeat(32)).is_some());
        assert!(bytes(&"00".repeat(33)).is_none());
    }
}
