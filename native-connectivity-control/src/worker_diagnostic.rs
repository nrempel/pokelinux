//! Best-effort failure evidence before the worker releases its parent channel.
use rustix::{
    fd::{AsFd, AsRawFd},
    fs::{FileType, Mode, OFlags, fstat, open},
    net::{self, AddressFamily, SendFlags, SocketType},
};
use std::{fmt, fmt::Write as _, io};

// Linux PIPE_BUF bounds FIFO writes. Streams have no atomic record guarantee.
const LIMIT: usize = 4096;
struct Buffer<const N: usize> {
    bytes: [u8; N],
    used: usize,
}
impl<const N: usize> fmt::Write for Buffer<N> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.used.checked_add(text.len()).ok_or(fmt::Error)?;
        let target = self.bytes.get_mut(self.used..end).ok_or(fmt::Error)?;
        target.copy_from_slice(text.as_bytes());
        self.used = end;
        Ok(())
    }
}

pub(super) fn emit(error: &io::Error) {
    emit_to(rustix::stdio::stderr(), error);
}

fn emit_to(sink: impl AsFd, error: &io::Error) {
    let Ok(original) = fstat(&sink) else { return };
    let sink_type = FileType::from_raw_mode(original.st_mode);
    if sink_type != FileType::Fifo && sink_type != FileType::Socket {
        return;
    }
    let mut record = Buffer {
        bytes: [0; LIMIT],
        used: 0,
    };
    if writeln!(&mut record, "connectivity_worker_failure_v1 {error:?}").is_err() {
        return; // A truncated formatter result is never emitted.
    }
    if sink_type == FileType::Socket {
        if net::sockopt::socket_type(&sink) != Ok(SocketType::STREAM)
            || net::sockopt::socket_domain(&sink) != Ok(AddressFamily::UNIX)
        {
            return;
        }
        // Per-call flags leave Main's shared open-file description unchanged.
        // A stream may accept only a prefix; there is no retry/completeness claim.
        let _ = net::send(
            &sink,
            &record.bytes[..record.used],
            SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
        );
        return;
    }
    // A dup shares O_NONBLOCK with Main's inherited stderr. Reopen the exact
    // pipe into an independent open-file description instead.
    let mut path = Buffer {
        bytes: [0; 64],
        used: 0,
    };
    if write!(&mut path, "/proc/self/fd/{}", sink.as_fd().as_raw_fd()).is_err() {
        return;
    }
    let Ok(path) = std::str::from_utf8(&path.bytes[..path.used]) else {
        return;
    };
    let Ok(writer) = open(
        path,
        OFlags::WRONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    ) else {
        return;
    };
    let Ok(reopened) = fstat(&writer) else { return };
    if FileType::from_raw_mode(reopened.st_mode) != FileType::Fifo
        || reopened.st_dev != original.st_dev
        || reopened.st_ino != original.st_ino
    {
        return;
    }
    // No retry, wait, flag change, protocol message or replacement error.
    let _ = rustix::io::write(&writer, &record.bytes[..record.used]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustix::{
        fs::{CWD, fcntl_getfl, fcntl_setfl, mkfifoat},
        io::Errno,
    };
    use std::{
        fs,
        os::unix::fs::DirBuilderExt,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fifo {
        directory: PathBuf,
        reader: rustix::fd::OwnedFd,
        writer: rustix::fd::OwnedFd,
    }
    impl Fifo {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "pokelinux-worker-diagnostic-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&directory)
                .unwrap();
            let path = directory.join("pipe");
            mkfifoat(CWD, &path, Mode::RUSR | Mode::WUSR).unwrap();
            let reader = open(
                &path,
                OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .unwrap();
            let writer = open(
                &path,
                OFlags::WRONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .unwrap();
            let flags = fcntl_getfl(&writer).unwrap();
            fcntl_setfl(&writer, flags & !OFlags::NONBLOCK).unwrap();
            Self {
                directory,
                reader,
                writer,
            }
        }
        fn drain(&self) -> Vec<u8> {
            let mut bytes = Vec::new();
            for _ in 0..512 {
                let mut block = [0; LIMIT];
                match rustix::io::read(&self.reader, &mut block) {
                    Ok(n) if n > 0 => bytes.extend_from_slice(&block[..n]),
                    Err(Errno::AGAIN) => return bytes,
                    other => panic!("actual pipe drain: {other:?}"),
                }
            }
            panic!("actual pipe drain bound");
        }
    }
    impl Drop for Fifo {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).unwrap();
        }
    }

    #[test]
    fn complete_nested_error_is_one_record_and_original_flags_stay_blocking() {
        use super::super::{BackendFailure, WorkerFailure, WorkerStep};
        let pipe = Fifo::new();
        let error = io::Error::new(
            io::ErrorKind::InvalidData,
            WorkerFailure {
                stage: WorkerStep::RefreshBackend,
                original_until_ns: Some(123),
                detected_at_ns: 99,
                source: io::Error::new(
                    io::ErrorKind::InvalidData,
                    BackendFailure {
                        operation: "refresh_wireless",
                        source: crate::kernel::Error::Observation(
                            pokelinux_network_state::ObserveError::InvalidData,
                        ),
                    },
                ),
            },
        );
        let expected = format!("connectivity_worker_failure_v1 {error:?}\n");
        let flags = fcntl_getfl(&pipe.writer).unwrap();
        assert!(!flags.contains(OFlags::NONBLOCK));
        emit_to(&pipe.writer, &error);
        assert_eq!(pipe.drain(), expected.as_bytes());
        assert_eq!(fcntl_getfl(&pipe.writer).unwrap(), flags);
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn full_pipe_returns_promptly_without_a_partial_record_or_shared_flag_change() {
        let pipe = Fifo::new();
        let flags = fcntl_getfl(&pipe.writer).unwrap();
        let filler = open(
            pipe.directory.join("pipe"),
            OFlags::WRONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .unwrap();
        let mut filled = 0;
        for _ in 0..256 {
            match rustix::io::write(&filler, &[b'F'; LIMIT]) {
                Ok(n) => {
                    assert_eq!(n, LIMIT);
                    filled += n;
                }
                Err(Errno::AGAIN) => break,
                other => panic!("actual pipe fill: {other:?}"),
            }
        }
        assert!(filled > 0);
        assert_eq!(
            rustix::io::write(&filler, &[b'F'; LIMIT]),
            Err(Errno::AGAIN)
        );
        // A regression to blocking I/O is released by this independently owned
        // drainer and fails the promptness assertion instead of hanging tests.
        let (elapsed, mut drained) = std::thread::scope(|scope| {
            let drainer = scope.spawn(|| {
                std::thread::park_timeout(Duration::from_millis(250));
                pipe.drain()
            });
            let start = Instant::now();
            emit_to(&pipe.writer, &io::Error::from(io::ErrorKind::InvalidData));
            let elapsed = start.elapsed();
            drainer.thread().unpark();
            (elapsed, drainer.join().unwrap())
        });
        drained.extend(pipe.drain());
        assert!(
            elapsed < Duration::from_millis(100),
            "blocking diagnostic: {elapsed:?}"
        );
        assert_eq!(drained, vec![b'F'; filled]);
        assert_eq!(fcntl_getfl(&pipe.writer).unwrap(), flags);
    }

    #[test]
    fn closed_reader_and_oversized_record_are_skipped() {
        let mut pipe = Fifo::new();
        emit_to(&pipe.writer, &io::Error::other("x".repeat(LIMIT * 2)));
        assert!(pipe.drain().is_empty());
        let reader = std::mem::replace(
            &mut pipe.reader,
            open("/dev/null", OFlags::RDONLY | OFlags::CLOEXEC, Mode::empty()).unwrap(),
        );
        drop(reader);
        let start = Instant::now();
        emit_to(&pipe.writer, &io::Error::from(io::ErrorKind::InvalidData));
        assert!(start.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn regular_file_sink_is_unchanged() {
        let pipe = Fifo::new();
        let path = pipe.directory.join("regular");
        fs::write(&path, b"existing record\n").unwrap();
        let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        let flags = fcntl_getfl(&file).unwrap();
        emit_to(&file, &io::Error::from(io::ErrorKind::InvalidData));
        assert_eq!(fs::read(path).unwrap(), b"existing record\n");
        assert_eq!(fcntl_getfl(&file).unwrap(), flags);
    }

    fn stream_error() -> io::Error {
        use super::super::{BackendFailure, WorkerFailure, WorkerStep};
        io::Error::new(
            io::ErrorKind::InvalidData,
            WorkerFailure {
                stage: WorkerStep::RefreshBackend,
                original_until_ns: Some(123),
                detected_at_ns: 99,
                source: io::Error::new(
                    io::ErrorKind::InvalidData,
                    BackendFailure {
                        operation: "refresh_wireless",
                        source: crate::kernel::Error::Observation(
                            pokelinux_network_state::ObserveError::InvalidData,
                        ),
                    },
                ),
            },
        )
    }

    fn drain_stream(peer: &std::os::unix::net::UnixStream) -> Vec<u8> {
        let mut bytes = Vec::new();
        for _ in 0..512 {
            let mut block = [0; LIMIT];
            match rustix::net::recv(peer, &mut block, rustix::net::RecvFlags::DONTWAIT) {
                Ok((n, _)) if n > 0 => bytes.extend_from_slice(&block[..n]),
                Ok((0, _)) | Err(Errno::AGAIN) => return bytes,
                other => panic!("actual stream drain: {other:?}"),
            }
        }
        panic!("actual stream drain bound");
    }

    #[test]
    fn unix_stream_emits_complete_nested_error_before_sink_teardown_without_flag_change() {
        use std::os::unix::net::UnixStream;
        let (writer, peer) = UnixStream::pair().unwrap();
        let alias = writer.try_clone().unwrap();
        let flags = fcntl_getfl(&writer).unwrap();
        assert!(!flags.contains(OFlags::NONBLOCK));
        assert_eq!(
            FileType::from_raw_mode(fstat(&writer).unwrap().st_mode),
            FileType::Socket
        );
        let error = stream_error();
        let original_error = (error.kind(), error.raw_os_error(), format!("{error:?}"));
        let expected = format!("connectivity_worker_failure_v1 {error:?}\n");
        assert!(expected.len() <= LIMIT);
        emit_to(&writer, &error);
        let actual = drain_stream(&peer);
        let still_open = rustix::net::recv(&peer, &mut [0; 1], rustix::net::RecvFlags::DONTWAIT)
            == Err(Errno::AGAIN);
        let flags_after = (fcntl_getfl(&writer).unwrap(), fcntl_getfl(&alias).unwrap());
        let error_after = (error.kind(), error.raw_os_error(), format!("{error:?}"));
        // The record was sampled while both original inherited references were
        // still alive. Close the actual sockets before any desired assertion.
        drop(alias);
        drop(writer);
        drop(peer);
        assert!(still_open);
        assert_eq!(flags_after, (flags, flags));
        assert_eq!(error_after, original_error);
        assert_eq!(
            actual,
            expected.as_bytes(),
            "UNIX_STREAM_DIAGNOSTIC_REQUIRED"
        );
    }

    #[test]
    fn full_unix_stream_returns_promptly_without_flag_or_error_change() {
        use rustix::net::{SendFlags, send};
        use std::os::unix::net::UnixStream;
        let (writer, peer) = UnixStream::pair().unwrap();
        let alias = writer.try_clone().unwrap();
        let flags = fcntl_getfl(&writer).unwrap();
        assert!(!flags.contains(OFlags::NONBLOCK));
        let mut filled = 0;
        let mut unavailable = false;
        for _ in 0..256 {
            match send(
                &writer,
                &[b'F'; LIMIT],
                SendFlags::DONTWAIT | SendFlags::NOSIGNAL,
            ) {
                Ok(n) if n > 0 => filled += n,
                Err(Errno::AGAIN) => {
                    unavailable = true;
                    break;
                }
                other => panic!("actual stream fill: {other:?}"),
            }
        }
        assert!(filled > 0 && unavailable);
        let error = stream_error();
        let original_error = (error.kind(), error.raw_os_error(), format!("{error:?}"));
        let expected = format!("connectivity_worker_failure_v1 {error:?}\n");
        // Test-only watchdog follows the existing FIFO test: a blocking
        // regression is released, measured and rejected rather than hanging.
        let (elapsed, mut drained) = std::thread::scope(|scope| {
            let drainer = scope.spawn(|| {
                std::thread::park_timeout(Duration::from_millis(250));
                drain_stream(&peer)
            });
            let start = Instant::now();
            emit_to(&writer, &error);
            let elapsed = start.elapsed();
            drainer.thread().unpark();
            (elapsed, drainer.join().unwrap())
        });
        drained.extend(drain_stream(&peer));
        let flags_after = (fcntl_getfl(&writer).unwrap(), fcntl_getfl(&alias).unwrap());
        let error_after = (error.kind(), error.raw_os_error(), format!("{error:?}"));
        drop(alias);
        drop(writer);
        drop(peer);
        assert!(
            elapsed < Duration::from_millis(100),
            "blocking stream diagnostic: {elapsed:?}"
        );
        assert_eq!(flags_after, (flags, flags));
        assert_eq!(error_after, original_error);
        assert!(drained.starts_with(&vec![b'F'; filled]));
        // A stream send is not atomic. Any best-effort tail must be a prefix
        // of this one bounded record; no retry/completeness is inferred here.
        assert!(expected.as_bytes().starts_with(&drained[filled..]));
    }

    #[test]
    fn closed_unix_stream_and_oversized_record_are_best_effort() {
        use std::os::unix::net::UnixStream;
        let (writer, peer) = UnixStream::pair().unwrap();
        let flags = fcntl_getfl(&writer).unwrap();
        emit_to(&writer, &io::Error::other("x".repeat(LIMIT * 2)));
        let oversized = drain_stream(&peer);
        drop(peer);
        let error = io::Error::from_raw_os_error(110);
        let original_error = (error.kind(), error.raw_os_error(), format!("{error:?}"));
        let start = Instant::now();
        emit_to(&writer, &error);
        let elapsed = start.elapsed();
        let flags_after = fcntl_getfl(&writer).unwrap();
        let error_after = (error.kind(), error.raw_os_error(), format!("{error:?}"));
        drop(writer);
        assert!(oversized.is_empty());
        assert!(elapsed < Duration::from_millis(100));
        assert_eq!(flags_after, flags);
        assert_eq!(error_after, original_error);
    }
}
