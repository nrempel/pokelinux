//! Machine interface for native apps: one current-protocol request on stdin.
#[cfg(target_os = "linux")]
fn run() -> Result<(), (u8, &'static str)> {
    use omarchy_phone_core::{
        MAX_FRAME,
        local_client::{ExchangeError, exchange},
        local_reply::Body,
    };
    use std::{
        io::{self, Read, Write},
        path::PathBuf,
        time::Duration,
    };
    let mut args = std::env::args_os().skip(1);
    let usage = (
        2,
        "usage: omarchy-phone --socket /absolute/bridge.sock rpc < request.json",
    );
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--socket")) {
        return Err(usage);
    }
    let path = PathBuf::from(args.next().ok_or(usage)?);
    if !path.is_absolute()
        || args.next().as_deref() != Some(std::ffi::OsStr::new("rpc"))
        || args.next().is_some()
    {
        return Err(usage);
    }
    let mut frame = Vec::new();
    io::stdin()
        .lock()
        .take((MAX_FRAME + 1) as u64)
        .read_to_end(&mut frame)
        .map_err(|_| (2, "not_sent: cannot read request"))?;
    let reply = exchange(&path, &frame, Duration::from_secs(2)).map_err(|error| match error {
        ExchangeError::NotSent(_) => (2, "not_sent: request was not submitted"),
        ExchangeError::Unknown(_) => (
            3,
            "unknown: recover with the original command ID; do not retry the command",
        ),
    })?;
    let mut output =
        serde_json::to_vec(&reply).map_err(|_| (3, "unknown: cannot encode validated reply"))?;
    output.push(b'\n');
    io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|_| (3, "unknown: cannot deliver reply to caller"))?;
    if matches!(reply.body(), Body::Error { .. }) {
        return Err((4, "service rejected the request; see validated reply"));
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    #[cfg(target_os = "linux")]
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err((code, message)) => {
            eprintln!("{message}");
            std::process::ExitCode::from(code)
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("omarchy-phone requires Linux");
        std::process::ExitCode::FAILURE
    }
}
