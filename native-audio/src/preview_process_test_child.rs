//! Standalone test-only finite process. No service/graph/audio behavior is faked.
#![forbid(unsafe_code)]
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Duration;

fn mark(root: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(bytes)?;
    file.flush()
}
fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 5 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mode = args[1].to_str().ok_or(io::ErrorKind::InvalidInput)?;
    let root = Path::new(&args[2]);
    let expected_len: usize = args[3]
        .to_str()
        .ok_or(io::ErrorKind::InvalidInput)?
        .parse()
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    let expected_hash: u64 = args[4]
        .to_str()
        .ok_or(io::ErrorKind::InvalidInput)?
        .parse()
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    if mode == "capture-gated" {
        if expected_len != 4 * 48_000 * 4 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // Mixed future-tail bytes are independently checked by the parent test;
        // the original payload hash argument is not claimed to describe them.
        let _ = expected_hash;
        return capture_gated(root, expected_len);
    }
    if expected_len > 192_000 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    mark(root, "ready", b"ready\n")?;
    match mode {
        "exit-ready" => {
            io::stderr().write_all(b"actual buffered normal exit\n")?;
            return Ok(());
        }
        "stderr-overflow" => {
            io::stderr().write_all(&[b'x'; 4097])?;
            io::stderr().flush()?;
            std::thread::sleep(Duration::from_secs(30));
            return Ok(());
        }
        "hold-before-input" => {
            std::thread::sleep(Duration::from_secs(30));
            return Ok(());
        }
        "finite" | "hold-after-input" => {}
        _ => return Err(io::ErrorKind::InvalidInput.into()),
    }
    let mut bytes = Vec::new();
    io::stdin()
        .take((expected_len + 1) as u64)
        .read_to_end(&mut bytes)?;
    let mut actual_hash = 0xcbf29ce484222325_u64;
    for byte in &bytes {
        actual_hash ^= u64::from(*byte);
        actual_hash = actual_hash.wrapping_mul(0x100000001b3);
    }
    if bytes.len() != expected_len || actual_hash != expected_hash {
        return Err(io::ErrorKind::InvalidData.into());
    }
    mark(root, "consumed", b"exact\n")?;
    io::stderr().write_all(b"finite input and actual stdin EOF\n")?;
    if mode == "hold-after-input" {
        std::thread::sleep(Duration::from_secs(30));
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        let _ = fs::write("/dev/stderr", format!("finite child failed: {error}\n"));
        std::process::exit(1);
    }
}

// Only the NEW private unit mode uses these three finite consumption gates.
// Parent Process retains actual child/pidfd/pipes and original6s work+2s close.
fn capture_gated(root: &Path, expected_len: usize) -> io::Result<()> {
    use std::time::Instant;
    let close = Instant::now() + Duration::from_secs(8);
    let gate = |name: &str| -> io::Result<()> {
        while !root.join(name).is_file() {
            if Instant::now() >= close {
                return Err(io::ErrorKind::TimedOut.into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    };
    mark(root, "ready", b"real finite capture ready\n")?;
    gate("go-0")?;
    let mut input = io::stdin().lock();
    let mut bytes = Vec::with_capacity(expected_len);
    (&mut input).take(16_384).read_to_end(&mut bytes)?;
    if bytes.len() != 16_384 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    mark(root, "stage-0", b"actual first16384bytes consumed\n")?;
    gate("go-1")?;
    (&mut input)
        .take((expected_len + 1 - bytes.len()) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() != expected_len {
        return Err(io::ErrorKind::InvalidData.into());
    }
    mark(root, "captured.pcm", &bytes)?;
    mark(root, "consumed", b"actual full finite input and stdinEOF\n")?;
    // Keep the exact child live AFTER all input was queued/consumed, so refusal
    // at fullyqueued cannot be satisfied merely by a previously dead producer.
    gate("go-2")?;
    io::stderr().write_all(b"actual gated finite input and normal stderrEOF\n")?;
    Ok(())
}
