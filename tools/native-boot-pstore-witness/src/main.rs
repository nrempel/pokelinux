//! Independent real-kernel pstore witness, never an installed host program.
use rustix::{
    fs::{CWD, FileType, Mode, OFlags, fstat, fstatfs, major, makedev, minor, mknodat, openat},
    mount::{MountFlags, mount},
    process::{getpid, getppid, getuid},
    rand::{GetRandomFlags, getrandom},
    system::uname,
};
use std::{
    fs::File,
    io::{self, Read, Write},
    time::Duration,
};

const RELEASE: &str = "6.6.127-android15-8-g91d825611104-ab16252310-4k";
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn bytes(path: &str, maximum: usize, fs_magic: u64) -> io::Result<Vec<u8>> {
    let fd = openat(
        CWD,
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    )?;
    if fstatfs(&fd)?.f_type as u64 != fs_magic {
        return Err(invalid());
    }
    let mut value = Vec::new();
    File::from(fd)
        .take(maximum as u64 + 1)
        .read_to_end(&mut value)?;
    if value.len() > maximum {
        return Err(invalid());
    }
    Ok(value)
}
fn hex(value: &[u8]) -> String {
    value.iter().map(|b| format!("{b:02x}")).collect()
}
fn run() -> io::Result<()> {
    let pid = getpid().as_raw_nonzero().get();
    if pid <= 1
        || getppid().map(|p| p.as_raw_nonzero().get()) != Some(1)
        || getuid().as_raw() != 0
        || uname().release().to_bytes() != RELEASE.as_bytes()
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    println!("POKELINUX_CHILD_EXEC pid={pid} parent=1");
    mount(
        "pstore",
        "/pstore",
        "pstore",
        MountFlags::RDONLY | MountFlags::NOSUID | MountFlags::NODEV | MountFlags::NOEXEC,
        None,
    )?;
    let boot = String::from_utf8(bytes("/proc/sys/kernel/random/boot_id", 64, 0x9fa0)?)
        .map_err(|_| invalid())?;
    let boot = boot.trim();
    if boot.len() != 36 || !boot.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-') {
        return Err(invalid());
    }
    let old = match bytes("/pstore/pmsg-ramoops-0", 512, 0x6165_676c) {
        Ok(value) => Some(value),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    if let Some(old) = old {
        let console = bytes("/pstore/console-ramoops-0", 65536, 0x6165_676c)?;
        if old.is_empty() || console.is_empty() {
            return Err(invalid());
        }
        println!(
            "POKELINUX_PSTORE_RETRIEVED current_boot={boot} pmsg_bytes={} pmsg_hex={} console_bytes={} console_hex={}",
            old.len(),
            hex(&old),
            console.len(),
            hex(&console)
        );
        io::stdout().flush()?;
        return Ok(());
    }
    let dev = String::from_utf8(bytes("/sys/class/pmsg/pmsg0/dev", 64, 0x6265_6572)?)
        .map_err(|_| invalid())?;
    let (ma, mi) = dev.trim().split_once(':').ok_or_else(invalid)?;
    let ma: u32 = ma.parse().map_err(|_| invalid())?;
    let mi: u32 = mi.parse().map_err(|_| invalid())?;
    if ma == 0 || mi != 0 {
        return Err(invalid());
    }
    mknodat(
        CWD,
        "/dev/pmsg-witness",
        FileType::CharacterDevice,
        Mode::RUSR | Mode::WUSR,
        makedev(ma, mi),
    )?;
    let fd = openat(
        CWD,
        "/dev/pmsg-witness",
        OFlags::WRONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOFOLLOW,
        Mode::empty(),
    )?;
    let st = fstat(&fd)?;
    if FileType::from_raw_mode(st.st_mode) != FileType::CharacterDevice
        || major(st.st_rdev) != ma
        || minor(st.st_rdev) != mi
    {
        return Err(invalid());
    }
    let mut epoch = [0_u8; 16];
    if getrandom(&mut epoch, GetRandomFlags::NONBLOCK)? != epoch.len() {
        return Err(invalid());
    }
    let epoch = hex(&epoch);
    let record = format!("POKELINUX_RECOVERY_CANARY epoch={epoch} boot={boot} kernel={RELEASE}\n");
    if record.len() > 512 || rustix::io::write(fd, record.as_bytes())? != record.len() {
        return Err(invalid());
    }
    println!(
        "POKELINUX_PSTORE_READY epoch={epoch} boot={boot} major={ma} minor={mi} wrote={}",
        record.len()
    );
    io::stdout().flush()?;
    loop {
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn main() {
    match run() {
        Ok(()) => std::process::exit(37),
        Err(error) => {
            eprintln!(
                "POKELINUX_PSTORE_WITNESS_FAILURE kind={:?} errno={:?} reason={error}",
                error.kind(),
                error.raw_os_error()
            );
            std::process::exit(3);
        }
    }
}
