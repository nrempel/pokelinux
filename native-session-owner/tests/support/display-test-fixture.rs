//! Private fixed-role process fixture; never an installed production entry.
use rustix::{
    fs::{self, OFlags},
    process::{Gid, Uid},
};
use std::{
    io::{self, Read, Write},
    os::{fd::AsFd, unix::net::UnixListener},
    path::Path,
    time::Duration,
};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--display-context-worker"] {
        return pokelinux_display::supervisor::worker_entry();
    }
    if args.len() != 3
        || !["--held-peer", "--delayed-peer"].contains(&args[0].as_str())
        || rustix::process::getuid().as_raw() != 0
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let uid: u32 = args[2].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
    if ![0, 1000].contains(&uid) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    rustix::thread::set_thread_groups(&[])?;
    rustix::thread::set_thread_res_gid(Gid::from_raw(uid), Gid::from_raw(uid), Gid::from_raw(uid))?;
    rustix::thread::set_thread_res_uid(Uid::from_raw(uid), Uid::from_raw(uid), Uid::from_raw(uid))?;
    let input = io::stdin();
    let flags = fs::fcntl_getfl(input.as_fd())?;
    fs::fcntl_setfl(input.as_fd(), flags | OFlags::NONBLOCK)?;
    let until = pokelinux_display::boottime_ns() + 10_000_000_000;
    if args[0] == "--delayed-peer" {
        println!(
            "{}",
            serde_json::json!({"event":"waiting_for_bind","pid":std::process::id(),"uid":uid,"gid":uid})
        );
        io::stdout().flush()?;
        loop {
            if pokelinux_display::boottime_ns() >= until {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let mut control = [0; 1];
            match input.lock().read(&mut control) {
                Ok(1) if control[0] == b'B' => break,
                Ok(0) | Ok(1) => return Ok(()),
                Ok(_) => unreachable!(),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let listener = UnixListener::bind(Path::new(&args[1]))?;
    listener.set_nonblocking(true)?;
    println!(
        "{}",
        serde_json::json!({"event":"listening","pid":std::process::id(),"uid":uid,"gid":uid})
    );
    io::stdout().flush()?;
    let mut accepted = None;
    let mut reported = false;
    while pokelinux_display::boottime_ns() < until {
        let mut control = [0; 1];
        match input.lock().read(&mut control) {
            Ok(0) | Ok(1) => return Ok(()),
            Ok(_) => unreachable!(),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e),
        }
        if accepted.is_none() {
            match listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(true)?;
                    accepted = Some(stream);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }
        }
        if !reported && let Some(stream) = accepted.as_mut() {
            let mut data = [0; 256];
            match stream.read(&mut data) {
                Ok(bytes) if bytes > 0 => {
                    println!(
                        "{}",
                        serde_json::json!({"event":"actual_registry_request","bytes":bytes})
                    );
                    io::stdout().flush()?;
                    reported = true;
                }
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }
        }
        // No Wayland reply is ever fabricated: actual SDK stays Preparing.
        std::thread::sleep(Duration::from_millis(2));
    }
    Err(io::ErrorKind::TimedOut.into())
}
