//! Local service-account diagnostics only. Stop observes server exit, not reaping.
#![forbid(unsafe_code)]
use pokelinux_process_owner::deadline::{BootClock, Clock};
use pokelinux_radio::status::{Client, Command, Reply};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    fd::AsFd,
    fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat},
    io::Errno,
};
use std::{io, path::Path, time::Duration};
const BUDGET: Duration = Duration::from_secs(3);
fn before(until: Duration) -> io::Result<()> {
    if BootClock.now() >= until {
        Err(io::ErrorKind::TimedOut.into())
    } else {
        Ok(())
    }
}
fn retry(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(4 | 11))
}
fn wait(fd: impl AsFd, events: PollFlags, until: Duration) -> io::Result<()> {
    before(until)?;
    let left = until
        .saturating_sub(BootClock.now())
        .min(Duration::from_millis(20));
    let timeout = Timespec {
        tv_sec: 0,
        tv_nsec: left.as_nanos() as i64,
    };
    let mut fds = [PollFd::new(&fd, events)];
    match poll(&mut fds, Some(&timeout)) {
        Ok(_) if fds[0].revents().contains(PollFlags::NVAL) => {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(_) | Err(Errno::INTR) => {}
        Err(error) => return Err(error.into()),
    }
    before(until)
}
fn output(text: &str, until: Duration) -> io::Result<()> {
    // Bound normal terminal/pipe writes under the same budget. Restore original
    // flags on return; these flags are shared with other holders of stdout's OFD.
    struct Restore {
        fd: std::io::Stdout,
        flags: OFlags,
    }
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fcntl_setfl(&self.fd, self.flags);
        }
    }
    if text.len() > 5120 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let fd = std::io::stdout();
    let kind = FileType::from_raw_mode(fstat(&fd)?.st_mode);
    if !matches!(kind, FileType::Fifo | FileType::Socket)
        && !(kind == FileType::CharacterDevice && rustix::termios::isatty(&fd))
    {
        // O_NONBLOCK does not bound regular-file or block-device writes.
        return Err(io::ErrorKind::Unsupported.into());
    }
    let flags = fcntl_getfl(&fd)?;
    fcntl_setfl(&fd, flags | OFlags::NONBLOCK)?;
    let guard = Restore { fd, flags };
    before(until)?;
    match rustix::io::write(&guard.fd, text.as_bytes()) {
        Ok(count) if count == text.len() => before(until),
        Ok(_) => Err(io::ErrorKind::WriteZero.into()),
        Err(error) => Err(error.into()),
    }
}
fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let command = match args[1].to_str() {
        Some("status") => Command::Status,
        Some("stop") => Command::Stop,
        _ => return Err(io::ErrorKind::InvalidInput.into()),
    };
    let path = Path::new(&args[0]);
    if !path.is_absolute() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let until = BootClock
        .now()
        .checked_add(BUDGET)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let client = loop {
        before(until)?;
        match Client::connect(path) {
            Ok(client) => {
                before(until)?;
                break client;
            }
            Err(error) if retry(&error) => {
                // Linux AF_UNIX nonblocking connect EAGAIN has not connected.
                // Retry with a fresh socket, never with a renewed cutoff.
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error),
        }
    };
    loop {
        before(until)?;
        match client.send(command) {
            Ok(()) => {
                before(until)?;
                break;
            }
            Err(error) if retry(&error) => wait(&client, PollFlags::OUT, until)?,
            Err(error) => return Err(error),
        }
    }
    let reply = loop {
        before(until)?;
        match client.receive() {
            Ok(Some(reply)) => {
                before(until)?;
                break reply;
            }
            Ok(None) => wait(&client, PollFlags::IN, until)?,
            Err(error) if retry(&error) => {}
            Err(error) => return Err(error),
        }
    };
    match (command, reply) {
        (Command::Status, Reply::Status(status)) => {
            output(&status.json(client.server_pid())?, until)
        }
        (Command::Stop, Reply::StopAck) => {
            loop {
                before(until)?;
                match client.server_exited() {
                    Ok(true) => {
                        before(until)?;
                        break;
                    }
                    Ok(false) => wait(client.server_pidfd(), PollFlags::IN, until)?,
                    Err(error) if retry(&error) => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        }
        _ => Err(io::ErrorKind::InvalidData.into()),
    }
}

fn main() {
    // Avoid an unbounded stderr print after a timed-out IPC/Stop operation.
    if run().is_err() {
        std::process::exit(1);
    }
}
