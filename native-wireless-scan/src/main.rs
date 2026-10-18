//! Bounded one-shot worker. The supervising caller owns wait, output EOF and Stop.
use pokelinux_wireless_scan::{Completed, Error, Request, scan_once};
use std::process::ExitCode;

fn decimal<T: std::str::FromStr>(value: &str) -> Option<T> {
    if value.is_empty() || !value.bytes().all(|v| v.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}
fn request() -> Option<Request> {
    let mut args = std::env::args().skip(1);
    let ifindex = decimal::<u32>(&args.next()?)?;
    let frequency_mhz = decimal::<u32>(&args.next()?)?;
    let original_until_ns = decimal::<u64>(&args.next()?)?;
    if args.next().is_some() {
        return None;
    }
    Some(Request {
        ifindex,
        frequency_mhz,
        original_until_ns,
    })
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
    let Some(request) = request() else {
        println!(
            "{{\"schema\":1,\"status\":\"invalid_request\",\"reason\":\"invalid_arguments\"}}"
        );
        return ExitCode::from(64);
    };
    let (status, reason, code) = match scan_once(request) {
        Ok(value) => {
            let text = completed(value);
            if text.len() <= 32768 && before(request.original_until_ns) {
                println!("{text}");
                return ExitCode::SUCCESS;
            }
            (
                "unconfirmed",
                "publication_expired_or_oversized".to_string(),
                70,
            )
        }
        Err(Error::Unavailable) => ("unavailable", "unsupported".to_string(), 2),
        Err(Error::InvalidRequest) => ("invalid_request", "invalid_request".to_string(), 64),
        Err(Error::Failed(cause)) => ("failed", format!("{cause:?}"), 70),
        Err(Error::Uncertain(cause)) => ("uncertain", format!("{cause:?}"), 70),
    };
    println!(
        "{{\"schema\":1,\"status\":\"{}\",\"reason\":\"{}\",\"ifindex\":{},\"frequency_mhz\":{},\"original_until_ns\":{}}}",
        status, reason, request.ifindex, request.frequency_mhz, request.original_until_ns
    );
    ExitCode::from(code)
}
