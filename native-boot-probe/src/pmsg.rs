//! One optional, identity-checked PMSG descriptor. Submission is not retention.
use rustix::{
    fd::OwnedFd,
    fs::{CWD, FileType, Mode, OFlags, fstat, fstatfs, major, makedev, minor, mknodat, openat},
    io::{read, write},
};
use std::io;

const SYSFS_MAGIC: u64 = 0x6265_6572;
const TMPFS_MAGIC: u64 = 0x0102_1994;
const NODE: &str = "pmsg-pokelinux";

fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}

pub(super) fn discover() -> io::Result<OwnedFd> {
    let attribute = openat(
        CWD,
        "/sys/class/pmsg/pmsg0/dev",
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOFOLLOW,
        Mode::empty(),
    )?;
    if fstatfs(&attribute)?.f_type as u64 != SYSFS_MAGIC
        || FileType::from_raw_mode(fstat(&attribute)?.st_mode) != FileType::RegularFile
    {
        return Err(invalid());
    }
    // One bounded read; reject an oversized or incomplete sysfs attribute.
    let mut buffer = [0u8; 65];
    let count = read(&attribute, &mut buffer[..])?;
    if count == 0 || count > 64 {
        return Err(invalid());
    }
    let line = buffer[..count].strip_suffix(b"\n").ok_or_else(invalid)?;
    let text = std::str::from_utf8(line).map_err(|_| invalid())?;
    let (ma, mi) = text.split_once(':').ok_or_else(invalid)?;
    if ma.is_empty()
        || mi.is_empty()
        || !ma.bytes().all(|byte| byte.is_ascii_digit())
        || !mi.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    let ma: u32 = ma.parse().map_err(|_| invalid())?;
    let mi: u32 = mi.parse().map_err(|_| invalid())?;
    let device = makedev(ma, mi);
    if ma == 0 || major(device) != ma || minor(device) != mi {
        return Err(invalid());
    }

    // Retain the mounted tmpfs directory and create one new private node.
    let directory = openat(
        CWD,
        "/dev",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    )?;
    if fstatfs(&directory)?.f_type as u64 != TMPFS_MAGIC {
        return Err(invalid());
    }
    mknodat(
        &directory,
        NODE,
        FileType::CharacterDevice,
        Mode::RUSR | Mode::WUSR,
        device,
    )?;
    let fd = openat(
        &directory,
        NODE,
        OFlags::WRONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOFOLLOW,
        Mode::empty(),
    )?;
    let stat = fstat(&fd)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::CharacterDevice
        || stat.st_mode & 0o7777 != 0o600
        || major(stat.st_rdev) != ma
        || minor(stat.st_rdev) != mi
    {
        return Err(invalid());
    }
    Ok(fd)
}

// Return the sole write's actual count; the caller handles short counts/errors.
pub(super) fn submit(fd: &OwnedFd, canonical_record: &[u8]) -> io::Result<usize> {
    let payload = canonical_record
        .strip_suffix(b"\n")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid-pmsg-record"))?;
    if payload.is_empty()
        || canonical_record.len() > 512
        || payload
            .iter()
            .any(|byte| matches!(*byte, b'\n' | b'\r' | 0))
        || std::str::from_utf8(canonical_record).is_err()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid-pmsg-record",
        ));
    }
    write(fd, canonical_record).map_err(Into::into)
}
