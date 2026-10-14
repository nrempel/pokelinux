//! Bounded admission to the kernel ring, separate from console or persistence.
use rustix::{
    fd::OwnedFd,
    fs::{FileType, Mode, OFlags, SeekFrom, fstat, major, minor, open, seek},
    io::{Errno, read, write},
    rand::{GetRandomFlags, getrandom},
};
use std::{cell::Cell, fmt, io, time::Duration};

const BUDGET_NS: u64 = 100_000_000;
const MAX_RECORDS: usize = 128;

pub struct Reporter {
    reader: OwnedFd,
    writer: OwnedFd,
    epoch: String,
    event: Cell<u64>,
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
        })
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
        // Keep serial-console observations available, but its success cannot
        // establish kernel-ring admission. Neither does a full kmsg write count:
        // the kernel can silently discard messages on rate limits or disable.
        use io::Write;
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
                        let mut output = io::stdout().lock();
                        writeln!(
                            output,
                            "POKELINUX_DIAGNOSTIC_ACK epoch={} event={event} kernel_seq={sequence} {message}",
                            self.epoch
                        )?;
                        output.flush()?;
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
