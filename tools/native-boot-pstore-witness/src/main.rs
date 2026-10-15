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

// This branch is independent test infrastructure. It never opens or writes PMSG.
fn native_profile() -> io::Result<Option<String>> {
    let fd = match openat(
        CWD,
        "/native-pmsg-profile",
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let info = fstat(&fd)?;
    if FileType::from_raw_mode(info.st_mode) != FileType::RegularFile
        || info.st_uid != 0
        || info.st_size > 32
    {
        return Err(invalid());
    }
    let mut value = String::new();
    File::from(fd).take(33).read_to_string(&mut value)?;
    if !matches!(value.as_str(), "success\n" | "failure\n" | "no-backend\n") {
        return Err(invalid());
    }
    Ok(Some(value.trim_end().to_owned()))
}
fn optional_bytes(path: &str, maximum: usize) -> io::Result<Option<Vec<u8>>> {
    match bytes(path, maximum, 0x6165_676c) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
fn native_read_only(profile: &str, boot: &str) -> io::Result<i32> {
    let device = match bytes("/sys/class/pmsg/pmsg0/dev", 64, 0x6265_6572) {
        Ok(value) => {
            let text = std::str::from_utf8(&value).map_err(|_| invalid())?;
            let (major_text, minor_text) = text.trim().split_once(':').ok_or_else(invalid)?;
            if major_text.is_empty()
                || minor_text.is_empty()
                || !major_text.bytes().all(|b| b.is_ascii_digit())
                || !minor_text.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(invalid());
            }
            let ma = major_text.parse::<u32>().map_err(|_| invalid())?;
            let mi = minor_text.parse::<u32>().map_err(|_| invalid())?;
            if ma == 0 || major(makedev(ma, mi)) != ma || minor(makedev(ma, mi)) != mi {
                return Err(invalid());
            }
            Some((ma, mi))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    if (profile == "no-backend") != device.is_none() {
        return Err(invalid());
    }
    let console = optional_bytes("/pstore/console-ramoops-0", 65536)?;
    let pmsg = optional_bytes("/pstore/pmsg-ramoops-0", 4096)?;
    if console.as_ref().is_some_and(|value| value.is_empty())
        || (console.is_none() && pmsg.is_some())
    {
        return Err(invalid());
    }
    if profile == "no-backend" && (console.is_some() || pmsg.is_some()) {
        return Err(invalid());
    }
    let stage = if console.is_some() {
        "retained"
    } else {
        "first"
    };
    let backend = if device.is_some() {
        "present"
    } else {
        "absent"
    };
    let (ma, mi) = device.unwrap_or((0, 0));
    let pmsg_status = if pmsg.is_some() { "present" } else { "missing" };
    let old_pmsg = pmsg.as_deref().unwrap_or_default();
    let old_console = console.as_deref().unwrap_or_default();
    println!(
        "POKELINUX_NATIVE_PSTORE_PHASE profile={profile} current_boot={boot} stage={stage} backend={backend} major={ma} minor={mi} pmsg_status={pmsg_status} pmsg_bytes={} pmsg_hex={} console_bytes={} console_hex={}",
        old_pmsg.len(),
        hex(old_pmsg),
        old_console.len(),
        hex(old_console)
    );
    io::stdout().flush()?;
    // Actual retained console, rather than missing PMSG, selects the second boot.
    // This intentional child-status control is not a native PID1 failure injector.
    Ok(if console.is_none() && profile == "failure" {
        3
    } else {
        37
    })
}

fn run() -> io::Result<i32> {
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
    if let Some(profile) = native_profile()? {
        return native_read_only(&profile, boot);
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
        return Ok(37);
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
        Ok(exit) => std::process::exit(exit),
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
