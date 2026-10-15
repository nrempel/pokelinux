//! One bounded private diagnostic request, bound to the exact socket-origin pidfd.
use pokelinux_cellular_service::{
    ipc::{Client, Request},
    now_ns, output,
    status::Status,
};
use std::{io, path::PathBuf, time::Duration};
fn run() -> io::Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if !(a.len() == 4 || a.len() == 5)
        || a[0] != "--runtime"
        || a[2] != "--until-ns"
        || (a.len() == 5 && a[4] != "--stop")
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let path = PathBuf::from(&a[1]);
    let until: u64 = a[3].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
    let now = now_ns();
    if !path.is_absolute()
        || path.as_os_str().len() > 256
        || until <= now
        || until - now > 60_000_000_000
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    output::configure()?;
    let request = if a.len() == 5 {
        Request::Stop
    } else {
        Request::Status
    };
    let client = Client::connect(&path.join("status.sock"))?;
    let mut sent = false;
    let mut response = None;
    loop {
        if now_ns() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if !sent {
            match client.send(request) {
                Ok(()) => sent = true,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(e),
            }
        } else if response.is_none() {
            match client.receive() {
                Ok(Some(bytes)) => {
                    if request == Request::Status {
                        let status: Status = serde_json::from_slice(&bytes)
                            .map_err(|_| io::ErrorKind::InvalidData)?;
                        if status.schema != 1
                            || status.server_pid != client.server_pid()
                            || status.sampled_at_ns > now_ns()
                        {
                            return Err(io::ErrorKind::InvalidData.into());
                        }
                        response = Some(status.encode()?);
                    } else {
                        let wanted = b"{\"schema\":1,\"stop_ack\":true}\n";
                        if bytes != wanted {
                            return Err(io::ErrorKind::InvalidData.into());
                        }
                        response = Some(
                            format!(
                                "{{\"schema\":1,\"stop_ack\":true,\"server_pid\":{}}}\n",
                                client.server_pid()
                            )
                            .into_bytes(),
                        );
                    }
                }
                Ok(None) => {}
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if let Some(bytes) = &response {
            // Exit readiness is diagnostic only. The guest's owned Main wait is separate.
            if request == Request::Status || client.server_exited()? {
                if now_ns() >= until {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                if !output::publish(bytes)? {
                    return Err(io::ErrorKind::WouldBlock.into());
                }
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn main() {
    std::process::exit(if run().is_ok() { 0 } else { 70 });
}
