use pokelinux_appearance::{Configuration, collector, service};
use std::{
    fs::OpenOptions,
    io::{self, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--collector" {
        return collector::run(args[2].parse().map_err(|_| io::ErrorKind::InvalidInput)?);
    }
    if args.len() != 4 || args[1] != "--serve" {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let f = OpenOptions::new()
        // Configuration ancestry is installer-controlled as well as its leaf.
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(&args[3])?;
    service::trusted_path(Path::new(&args[3]), false)?;
    let m = f.metadata()?;
    if !m.is_file() || m.uid() != 0 || m.mode() & 0o022 != 0 || m.len() > 4096 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mut bytes = Vec::new();
    f.take(4097).read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let config: Configuration = serde_json::from_slice(&bytes)?;
    service::run(Path::new(&args[2]), config, &std::env::current_exe()?)
}
