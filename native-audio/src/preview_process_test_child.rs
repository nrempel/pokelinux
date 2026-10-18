//! Standalone test-only finite process. No service/graph/audio behavior is faked.
#![forbid(unsafe_code)]
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Duration;

fn mark(root: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(root.join(name))?;
    file.write_all(bytes)?;
    file.flush()
}
fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 5 { return Err(io::ErrorKind::InvalidInput.into()); }
    let mode = args[1].to_str().ok_or(io::ErrorKind::InvalidInput)?;
    let root = Path::new(&args[2]);
    let expected_len: usize = args[3].to_str().ok_or(io::ErrorKind::InvalidInput)?
        .parse().map_err(|_| io::ErrorKind::InvalidInput)?;
    let expected_hash: u64 = args[4].to_str().ok_or(io::ErrorKind::InvalidInput)?
        .parse().map_err(|_| io::ErrorKind::InvalidInput)?;
    if expected_len > 192_000 { return Err(io::ErrorKind::InvalidInput.into()); }
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
    io::stdin().take((expected_len + 1) as u64).read_to_end(&mut bytes)?;
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
