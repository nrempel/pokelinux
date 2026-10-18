//! Normal one-shot CLI; the current baseline only reports explicit unavailability.
use pokelinux_wireless_scan::{Error, Request, scan_once};
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
    Some(Request { ifindex, frequency_mhz, original_until_ns })
}

fn main() -> ExitCode {
    let Some(request) = request() else {
        println!("{{\"schema\":1,\"status\":\"invalid_request\",\"reason\":\"invalid_arguments\"}}");
        return ExitCode::from(64);
    };
    match scan_once(request) {
        Err(Error::Unavailable) => {
            println!("{{\"schema\":1,\"status\":\"unavailable\",\"reason\":\"unsupported\",\"ifindex\":{},\"frequency_mhz\":{},\"original_until_ns\":{}}}",
                request.ifindex, request.frequency_mhz, request.original_until_ns);
            ExitCode::from(2)
        }
        Ok(_) => {
            // No Completed value is constructible by this refusing implementation.
            // Its future wire shape is frozen separately in CONTRACT.json.
            println!("{{\"schema\":1,\"status\":\"unavailable\",\"reason\":\"unexpected_completion\"}}");
            ExitCode::from(70)
        }
    }
}
