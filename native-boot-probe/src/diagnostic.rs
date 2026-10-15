//! Bounded admission to the kernel ring, separate from console or persistence.
use rustix::{
    fd::OwnedFd,
    fs::{FileType, Mode, OFlags, SeekFrom, fstat, major, minor, open, seek},
    io::{Errno, read, write},
    rand::{GetRandomFlags, getrandom},
};
use std::{
    cell::{Cell, RefCell},
    fmt, io,
    time::Duration,
};

const BUDGET_NS: u64 = 100_000_000;
const MAX_RECORDS: usize = 128;
const MAX_PMSG_ATTEMPTS: u8 = 8;

// Exactly one deferred startup record; this is not a diagnostic backlog.
enum PmsgState {
    Pending { startup: Option<Vec<u8>> },
    Active { fd: OwnedFd, attempts: u8 },
    Disabled,
}

pub struct Reporter {
    reader: OwnedFd,
    writer: OwnedFd,
    epoch: String,
    event: Cell<u64>,
    pmsg: RefCell<PmsgState>,
}
impl Reporter {
    pub fn open() -> io::Result<Self> {
        let device = |access| -> io::Result<OwnedFd> {
            let fd = open(
                "/dev/kmsg",
                access | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOFOLLOW,
                Mode::empty(),
            )?;
            let stat = fstat(&fd)?;
            if FileType::from_raw_mode(stat.st_mode) != FileType::CharacterDevice
                || major(stat.st_rdev) != 1
                || minor(stat.st_rdev) != 11
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
            Ok(fd)
        };
        // Early boot must fail visibly if the kernel RNG is not ready. Never
        // wait indefinitely for entropy, or substitute a reusable clock ID.
        let mut random = [0u8; 16];
        if getrandom(&mut random, GetRandomFlags::NONBLOCK)? != random.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "short-epoch-read",
            ));
        }
        Ok(Self {
            reader: device(OFlags::RDONLY)?,
            writer: device(OFlags::WRONLY)?,
            epoch: random.iter().map(|byte| format!("{byte:02x}")).collect(),
            event: Cell::new(0),
            pmsg: RefCell::new(PmsgState::Pending { startup: None }),
        })
    }

    /// Try once after sysfs/dev mounts. Optional failure never changes boot policy.
    pub fn attach_pmsg(&self) {
        let startup = match self.pmsg.replace(PmsgState::Disabled) {
            PmsgState::Pending { startup } => startup,
            state => {
                *self.pmsg.borrow_mut() = state;
                return;
            }
        };
        match crate::pmsg::discover() {
            Ok(fd) => {
                *self.pmsg.borrow_mut() = PmsgState::Active { fd, attempts: 0 };
                // Take ownership before attempting; never replay this occurrence.
                if let Some(record) = startup {
                    self.pmsg_record(1, &record);
                }
            }
            Err(error) => self.pmsg_unavailable("discover", &error),
        }
    }

    fn pmsg_record(&self, event: u64, record: &[u8]) {
        // The caller has already validated the canonical record and its512-byte
        // kmsg bound. Its three-byte priority prefix is not part of these bytes.
        match self.pmsg.replace(PmsgState::Disabled) {
            PmsgState::Pending { mut startup } => {
                // main's fixed first event is startup. Later events cannot replace it.
                if event == 1 && startup.is_none() {
                    startup = Some(record.to_vec());
                }
                *self.pmsg.borrow_mut() = PmsgState::Pending { startup };
            }
            PmsgState::Active { fd, attempts } => {
                if attempts == MAX_PMSG_ATTEMPTS {
                    // State is already disabled; drop only the local descriptor.
                    drop(fd);
                    self.pmsg_unavailable("budget", &io::Error::other("pmsg-attempt-budget"));
                    return;
                }
                // Consume the attempt before I/O. An uncertain failure stays disabled.
                let attempts = attempts + 1;
                match crate::pmsg::submit(&fd, record) {
                    Ok(count) if count == record.len() => {
                        *self.pmsg.borrow_mut() = PmsgState::Active { fd, attempts };
                        use io::Write;
                        let mut output = io::stdout().lock();
                        let _ = writeln!(
                            output,
                            "POKELINUX_PMSG_SUBMITTED epoch={} event={event} bytes={count}",
                            self.epoch
                        );
                        let _ = output.flush();
                    }
                    Ok(_) => {
                        drop(fd);
                        self.pmsg_unavailable(
                            "submit",
                            &io::Error::new(io::ErrorKind::WriteZero, "incomplete-pmsg-submit"),
                        );
                    }
                    Err(error) => {
                        drop(fd);
                        self.pmsg_unavailable("submit", &error);
                    }
                }
            }
            PmsgState::Disabled => {}
        }
    }

    fn pmsg_unavailable(&self, phase: &str, error: &io::Error) {
        use io::Write;
        // Even a status-output error is optional; do not recurse into emit or panic.
        let _ = writeln!(
            io::stderr().lock(),
            "POKELINUX_PMSG_UNAVAILABLE epoch={} phase={phase} kind={:?} errno={:?} reason={error}",
            self.epoch,
            error.kind(),
            error.raw_os_error()
        );
    }

    pub fn emit(&self, args: fmt::Arguments<'_>) -> io::Result<()> {
        let message = args.to_string();
        if message.is_empty() || message.contains(['\n', '\r', '\0']) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid-message",
            ));
        }
        let event = self
            .event
            .get()
            .checked_add(1)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "event-exhausted"))?;
        // Consume the ordinal even if emission fails. No uncertain operation
        // may reuse its identity. This is occurrence correlation under the
        // restricted probe's trusted-writer assumption, not PID authentication.
        self.event.set(event);
        let expected = format!("{message} epoch={} event={event}", self.epoch);
        let record = format!("<6>{expected}\n");
        if record.len() > 512 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "record-too-long",
            ));
        }
        self.pmsg_record(event, &record.as_bytes()[3..]);
        // Keep serial-console observations available, but its success cannot
        // establish kernel-ring admission. Neither does a full kmsg write count:
        // the kernel can silently discard messages on rate limits or disable.
        let until = crate::now().saturating_add(BUDGET_NS);
        seek(&self.reader, SeekFrom::End(0))?;
        if write(&self.writer, record.as_bytes())? != record.len() {
            return Err(io::ErrorKind::WriteZero.into());
        }
        let mut buffer = [0u8; 16384];
        let mut previous: Option<u64> = None;
        let mut records = 0;
        while crate::now() < until && records < MAX_RECORDS {
            match read(&self.reader, &mut buffer) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(count) => {
                    records += 1;
                    let text = std::str::from_utf8(&buffer[..count])
                        .map_err(|_| io::ErrorKind::InvalidData)?;
                    let (header, payload) =
                        text.split_once(';').ok_or(io::ErrorKind::InvalidData)?;
                    let sequence = header
                        .split(',')
                        .nth(1)
                        .and_then(|v| v.parse::<u64>().ok())
                        .ok_or(io::ErrorKind::InvalidData)?;
                    if previous.is_some_and(|v| v.checked_add(1) != Some(sequence)) {
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                    previous = Some(sequence);
                    if payload.strip_suffix('\n') == Some(expected.as_str()) && crate::now() < until
                    {
                        // Only a distinct raw stdout ACK establishes readback.
                        // The kernel may console-print its payload before we
                        // successfully read it. Consumers must require this ACK.
                        crate::ack::emit(&self.epoch, event, sequence, &message)?;
                        return Ok(());
                    }
                }
                Err(Errno::INTR) => {}
                Err(Errno::AGAIN) => std::thread::sleep(Duration::from_millis(1)),
                Err(error) => return Err(error.into()),
            }
        }
        if records == MAX_RECORDS {
            Err(io::Error::other("kernel-record-budget"))
        } else {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "kernel-readback-deadline",
            ))
        }
    }
}
