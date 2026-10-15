//! Private normal-GUI tracer launch. No authentication or service-query API.
//! The lifecycle launcher has already dropped credentials and armed PDEATHSIG.
//! Root provisions the runtime ancestors and validates the fixed target ELF;
//! this process only creates its private diagnostic sink and directly execs it.
#![forbid(unsafe_code)]
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        process::CommandExt,
    },
    path::Path,
    process::{Command, Stdio},
};

const TARGET: &str = "/opt/service-restoration/pokelinux-messages";
const MAX_ARGUMENT_BYTES: usize = 256;
const MAX_STATUS_BYTES: u64 = 16_384;

fn field<'a>(status: &'a str, name: &str) -> io::Result<&'a str> {
    let mut values = status.lines().filter_map(|line| line.strip_prefix(name));
    let value = values.next().ok_or(io::ErrorKind::InvalidData)?;
    if values.next().is_some() {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(value.trim())
}

fn identity() -> io::Result<()> {
    let mut status = String::new();
    File::open("/proc/self/status")?
        .take(MAX_STATUS_BYTES + 1)
        .read_to_string(&mut status)?;
    if status.len() as u64 > MAX_STATUS_BYTES {
        return Err(io::ErrorKind::InvalidData.into());
    }
    for name in ["Uid:", "Gid:"] {
        let ids: Vec<_> = field(&status, name)?.split_whitespace().collect();
        if ids.len() != 4 || ids.iter().any(|id| *id != "11010") {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    if !field(&status, "Groups:")?.is_empty() || field(&status, "CapEff:")? != "0000000000000000" {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 6
        || args.iter().any(|arg| {
            let bytes = arg.as_os_str().as_bytes();
            bytes.len() > MAX_ARGUMENT_BYTES || bytes.contains(&0)
        })
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    identity()?;
    let runtime = Path::new(&args[1]);
    let directory = fs::symlink_metadata(runtime)?;
    if !runtime.is_absolute()
        || !directory.is_dir()
        || directory.uid() != 11010
        || directory.mode() & 0o7777 != 0o700
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    // create_new uses O_CREAT|O_EXCL: an existing node, including a symlink,
    // is refused rather than followed or truncated. Root binds the resulting
    // post-exec fd2 device/inode and the actual normal executable separately.
    let log = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(runtime.join("wayland-client.log"))?;
    let metadata = log.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != 11010
        || metadata.mode() & 0o7777 != 0o600
        || metadata.nlink() != 1
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    // exec preserves PID, credentials and the existing parent-death contract.
    // No subprocess, signal handling, resource limit or extra Qt argument exists.
    Err(Command::new(TARGET)
        .args(&args)
        .env_clear()
        .env("WAYLAND_DEBUG", "client")
        .stderr(Stdio::from(log))
        .exec())
}
