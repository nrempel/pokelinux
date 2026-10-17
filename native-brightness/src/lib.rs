//! Deployment-selected brightness I/O, independent of presentation and policy.
#![forbid(unsafe_code)]
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Component, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendKind {
    LinuxBacklight,
    SyntheticTree,
}
pub mod model;
pub mod protocol;
pub mod worker;
pub use model::Observation;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WriteReadback {
    pub target: u32,
    pub observed: Observation,
}
#[derive(Debug)]
pub struct Backend {
    directory: PathBuf,
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl Backend {
    pub fn new(class_root: PathBuf, device: String, kind: BackendKind) -> io::Result<Self> {
        if !class_root.is_absolute()
            || class_root
                .components()
                .any(|c| matches!(c, Component::ParentDir))
            || device.is_empty()
            || device.len() > 128
            || device == "."
            || device == ".."
            || !device
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            return Err(invalid("invalid deployment selection"));
        }
        if kind == BackendKind::LinuxBacklight {
            // This is a deployment backend, never an app-supplied arbitrary path.
            if class_root != std::path::Path::new("/sys/class/backlight") {
                return Err(invalid("Linux backlight class path required"));
            }
            let class = File::open(&class_root)?;
            // Linux uapi/linux/magic.h SYSFS_MAGIC. SyntheticTree never takes
            // this path, even if LinuxBacklight is unavailable on the host.
            if rustix::fs::fstatfs(&class)?.f_type as u64 != 0x6265_6572 {
                return Err(invalid("Linux backlight requires sysfs"));
            }
        }
        Ok(Self {
            directory: class_root.join(device),
        })
    }
    fn attribute(&self, name: &str) -> io::Result<u32> {
        let mut bytes = Vec::new();
        File::open(self.directory.join(name))?
            .take(33)
            .read_to_end(&mut bytes)?;
        let digits = bytes
            .strip_suffix(b"\n")
            .ok_or_else(|| invalid("unterminated attribute"))?;
        if digits.is_empty()
            || digits.len() > 10
            || (digits.len() > 1 && digits[0] == b'0')
            || !digits.iter().all(u8::is_ascii_digit)
        {
            return Err(invalid("noncanonical attribute"));
        }
        let mut value = 0_u32;
        for digit in digits {
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(u32::from(digit - b'0')))
                .ok_or_else(|| invalid("attribute overflow"))?;
        }
        Ok(value)
    }
    pub fn read(&self) -> io::Result<Observation> {
        let max = self.attribute("max_brightness")?;
        let requested = self.attribute("brightness")?;
        let actual = match self.attribute("actual_brightness") {
            Ok(value) => Some(value),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        let value = Observation {
            max,
            requested,
            actual,
        };
        if !value.valid() {
            return Err(invalid("out-of-range attribute"));
        }
        Ok(value)
    }
    /// The caller owns this potentially blocking operation in a retained child.
    /// There is exactly one write attempt: a short/interrupted write is uncertain,
    /// never a reason to send the property update again.
    pub fn set_percent(&self, percent: u8) -> io::Result<WriteReadback> {
        if percent > 100 {
            return Err(invalid("percent above 100"));
        }
        let before = self.read()?;
        let target = model::scaled(before.max, percent);
        let bytes = format!("{target}\n").into_bytes();
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.directory.join("brightness"))?;
        if file.write(&bytes)? != bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "uncertain short brightness write",
            ));
        }
        drop(file);
        let observed = self.read()?;
        if observed.max != before.max || observed.requested != target {
            return Err(invalid("brightness readback changed or mismatched"));
        }
        Ok(WriteReadback { target, observed })
    }
}
