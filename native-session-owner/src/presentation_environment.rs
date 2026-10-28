//! Two presentation values only. This file is not a permission or Source authority.
#![forbid(unsafe_code)]
use pokelinux_kernel_identity::{ProcessView, socket_peer_pidfd};
use rustix::{
    fs::{self, AtFlags, FileType, Mode, OFlags, RenameFlags, Stat},
    net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType},
    process,
};
use std::{
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::process::CommandExt,
    },
    process::Command,
    time::Duration,
};
const FILE: &str = "pokelinux-presentation.env";
const DISPLAY: &str = "wayland-1";
const MAX_BYTES: u64 = 256;
const WAIT_NS: u128 = 5_000_000_000;
const DIR: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::CLOEXEC)
    .union(OFlags::NOFOLLOW);

fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn now() -> u128 {
    let value = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    value.tv_sec as u128 * 1_000_000_000 + value.tv_nsec as u128
}
fn before(deadline: u128) -> io::Result<()> {
    if now() >= deadline {
        Err(io::ErrorKind::TimedOut.into())
    } else {
        Ok(())
    }
}
fn user() -> io::Result<()> {
    if process::getuid().as_raw() == 1000
        && process::geteuid().as_raw() == 1000
        && process::getgid().as_raw() == 1000
        && process::getegid().as_raw() == 1000
    {
        Ok(())
    } else {
        Err(io::ErrorKind::PermissionDenied.into())
    }
}
fn directory(parent: &OwnedFd, name: &str, uid: u32) -> io::Result<OwnedFd> {
    let fd = fs::openat(parent, name, DIR, Mode::empty())?;
    let stat = fs::fstat(&fd)?;
    if stat.st_uid != uid || stat.st_mode & 0o022 != 0 {
        return Err(invalid());
    }
    Ok(fd)
}
fn runtime() -> io::Result<OwnedFd> {
    // Pin every ancestor: NOFOLLOW on just an absolute final component is insufficient.
    let root = fs::open("/", DIR, Mode::empty())?;
    let run = directory(&root, "run", 0)?;
    let users = directory(&run, "user", 0)?;
    let runtime = directory(&users, "1000", 1000)?;
    let stat = fs::fstat(&runtime)?;
    if stat.st_gid != 1000 || stat.st_mode & 0o7777 != 0o700 {
        return Err(invalid());
    }
    Ok(runtime)
}
fn regular(stat: &Stat) -> io::Result<()> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_uid != 1000
        || stat.st_gid != 1000
        || stat.st_mode & 0o7777 != 0o600
        || stat.st_nlink != 1
        || stat.st_size < 0
        || stat.st_size as u64 > MAX_BYTES
    {
        return Err(invalid());
    }
    Ok(())
}
fn same(left: &Stat, right: &Stat) -> bool {
    left.st_dev == right.st_dev
        && left.st_ino == right.st_ino
        && left.st_size == right.st_size
        && left.st_mtime == right.st_mtime
        && left.st_mtime_nsec == right.st_mtime_nsec
        && left.st_ctime == right.st_ctime
        && left.st_ctime_nsec == right.st_ctime_nsec
}
#[derive(PartialEq)]
struct Values {
    signature: String,
}
impl Values {
    fn new(display: &str, signature: &str) -> io::Result<Self> {
        if display != DISPLAY
            || signature.is_empty()
            || signature.len() > 128
            || signature == "."
            || signature == ".."
            || !signature
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        {
            return Err(invalid());
        }
        Ok(Self {
            signature: signature.to_owned(),
        })
    }
    fn bytes(&self) -> Vec<u8> {
        format!(
            "WAYLAND_DISPLAY={DISPLAY}\nHYPRLAND_INSTANCE_SIGNATURE={}\n",
            self.signature
        )
        .into_bytes()
    }
    fn parse(bytes: &[u8]) -> io::Result<Self> {
        let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
        let (display, signature) = text.split_once('\n').ok_or_else(invalid)?;
        let value = Self::new(
            display
                .strip_prefix("WAYLAND_DISPLAY=")
                .ok_or_else(invalid)?,
            signature
                .strip_prefix("HYPRLAND_INSTANCE_SIGNATURE=")
                .and_then(|s| s.strip_suffix('\n'))
                .ok_or_else(invalid)?,
        )?;
        if value.bytes() != bytes {
            return Err(invalid());
        }
        Ok(value)
    }
}
fn read(runtime: &OwnedFd) -> io::Result<(Values, Stat)> {
    let fd = fs::openat(
        runtime,
        FILE,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )?;
    let initial = fs::fstat(&fd)?;
    regular(&initial)?;
    let mut file = File::from(fd);
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let final_stat = fs::fstat(&file)?;
    regular(&final_stat)?;
    if !same(&initial, &final_stat) || bytes.len() as i64 != initial.st_size {
        return Err(invalid());
    }
    Ok((Values::parse(&bytes)?, initial))
}
fn endpoint(directory: &OwnedFd, name: &str) -> io::Result<(OwnedFd, net::UCred)> {
    let initial = fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW)?;
    if FileType::from_raw_mode(initial.st_mode) != FileType::Socket
        || initial.st_uid != 1000
        || initial.st_gid != 1000
    {
        return Err(invalid());
    }
    // Linux has no connectat. Resolve through our pinned descriptor, never a glob
    // or a caller path; a final symlink is rejected above and checked again below.
    let address = SocketAddrUnix::new(format!("/proc/self/fd/{}/{name}", directory.as_raw_fd()))?;
    let socket = net::socket_with(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    net::connect(&socket, &address)?;
    let credentials = net::sockopt::socket_peercred(&socket)?;
    if credentials.uid.as_raw() != 1000
        || credentials.gid.as_raw() != 1000
        || !same(
            &initial,
            &fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW)?,
        )
    {
        return Err(invalid());
    }
    Ok((socket, credentials))
}
fn peers(runtime: &OwnedFd, values: &Values) -> io::Result<()> {
    let hypr = directory(runtime, "hypr", 1000)?;
    let instance = directory(&hypr, &values.signature, 1000)?;
    let (wayland, left) = endpoint(runtime, DISPLAY)?;
    let (control, right) = endpoint(&instance, ".socket.sock")?;
    if left != right {
        return Err(invalid());
    }
    let wayland_peer = socket_peer_pidfd(&wayland)?;
    let control_peer = socket_peer_pidfd(&control)?;
    if !ProcessView::open()?.matches(&wayland_peer, &control_peer, left.pid.as_raw_pid() as u32)? {
        return Err(io::ErrorKind::NotConnected.into());
    }
    Ok(())
}
fn publish() -> io::Result<()> {
    user()?;
    let display = std::env::var("WAYLAND_DISPLAY").map_err(|_| invalid())?;
    let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| invalid())?;
    let values = Values::new(&display, &signature)?;
    let runtime = runtime()?;
    peers(&runtime, &values)?;
    let temporary = format!(".pokelinux-presentation-{}.tmp", std::process::id());
    let fd = fs::openat(
        &runtime,
        temporary.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::RUSR | Mode::WUSR,
    )?;
    let result = (|| {
        let mut file = File::from(fd);
        file.write_all(&values.bytes())?;
        regular(&fs::fstat(&file)?)?;
        peers(&runtime, &values)?;
        fs::renameat_with(
            &runtime,
            temporary.as_str(),
            &runtime,
            FILE,
            RenameFlags::NOREPLACE,
        )?;
        Ok(())
    })();
    // Unlink only our fixed-directory temporary name; never follow its target.
    let cleanup = fs::unlinkat(&runtime, temporary.as_str(), AtFlags::empty());
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(rustix::io::Errno::NOENT)) | (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(error)) => Err(error.into()),
    }
}
fn clear() -> io::Result<()> {
    if process::getuid().as_raw() != 0 || process::geteuid().as_raw() != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    // The pre-start process needs no root filesystem bypass after this point.
    let uid = process::Uid::from_raw(1000);
    let gid = process::Gid::from_raw(1000);
    rustix::thread::set_thread_groups(&[])?;
    rustix::thread::set_thread_res_gid(gid, gid, gid)?;
    rustix::thread::set_thread_res_uid(uid, uid, uid)?;
    let runtime = match runtime() {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        value => value?,
    };
    let stat = match fs::statat(&runtime, FILE, AtFlags::SYMLINK_NOFOLLOW) {
        Err(rustix::io::Errno::NOENT) => return Ok(()),
        value => value?,
    };
    regular(&stat)?;
    fs::unlinkat(&runtime, FILE, AtFlags::empty())?;
    Ok(())
}
fn wait() -> io::Result<(Values, u128)> {
    // One BOOTTIME deadline, established before any filesystem work.
    let deadline = now() + WAIT_NS;
    user()?;
    loop {
        before(deadline)?;
        let attempt = (|| {
            let runtime = runtime()?;
            let (values, initial) = read(&runtime)?;
            peers(&runtime, &values)?;
            let (again, final_stat) = read(&runtime)?;
            if values != again || !same(&initial, &final_stat) {
                return Err(invalid());
            }
            before(deadline)?;
            Ok((values, deadline))
        })();
        match attempt {
            Ok(ready) => return Ok(ready),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        before(deadline)?;
        std::thread::sleep(Duration::from_nanos(
            deadline.saturating_sub(now()).min(10_000_000) as u64,
        ));
    }
}
fn launch() -> io::Result<()> {
    let (values, deadline) = wait()?;
    let mut command = Command::new("/usr/bin/quickshell");
    command
        .args(["-n", "-p", "/usr/share/pokelinux/omarchy/shell"])
        .env("WAYLAND_DISPLAY", DISPLAY)
        .env("HYPRLAND_INSTANCE_SIGNATURE", values.signature);
    before(deadline)?;
    Err(command.exec())
}
pub fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    match args[1].to_str() {
        Some("publish") => publish(),
        Some("clear") => clear(),
        Some("wait") => wait().map(|_| ()),
        Some("launch") => launch(),
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}
