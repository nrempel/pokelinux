//! Isolated fixture introspection; production exposes no test descriptor.
use pokelinux_camera_service::worker::Supervisor;
use pokelinux_lifecycle::{boottime_ns, camera::Provider, ipc};
use rustix::{
    net::{self, SocketType},
    process::{self, Pid, PidfdFlags, PidfdGetfdFlags, Signal},
};
use std::{collections::BTreeSet, fs, io, time::Duration};

pub fn descriptors() -> io::Result<BTreeSet<i32>> {
    Ok(fs::read_dir("/proc/self/fd")?
        .filter_map(|entry| {
            entry.ok().and_then(|entry| {
                fs::read_link(entry.path())
                    .ok()
                    .filter(|path| path.to_string_lossy().starts_with("socket:["))?;
                entry.file_name().to_str()?.parse::<i32>().ok()
            })
        })
        .collect())
}
pub fn hold(camera: &mut Supervisor, before: &BTreeSet<i32>) -> io::Result<()> {
    let until = boottime_ns().saturating_add(1_000_000_000);
    while !camera.available() {
        if boottime_ns() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        camera.poll()?;
        std::thread::sleep(Duration::from_millis(2));
    }
    let own = process::pidfd_open(process::getpid(), PidfdFlags::NONBLOCK)?;
    let mut sockets = Vec::new();
    for raw in descriptors()?.difference(before).copied() {
        if fs::read_link(format!("/proc/self/fd/{raw}"))
            .is_ok_and(|path| path.to_string_lossy().starts_with("socket:["))
        {
            // The Supervisor retains this sole new parent socket throughout.
            let fd = process::pidfd_getfd(&own, raw, PidfdGetfdFlags::empty())?;
            if net::sockopt::socket_type(&fd)? == SocketType::SEQPACKET {
                sockets.push(fd);
            }
        }
    }
    if sockets.len() != 1 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let children = fs::read_to_string(format!("/proc/self/task/{}/children", std::process::id()))?;
    let children: Vec<u32> = children
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|_| io::ErrorKind::InvalidData)?;
    if children.len() != 1 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let pid = children[0];
    if !fs::read(format!("/proc/{pid}/cmdline"))?
        .split(|byte| *byte == 0)
        .any(|arg| arg == b"--camera-worker")
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let pidfd = process::pidfd_open(
        Pid::from_raw(pid as i32).ok_or(io::ErrorKind::InvalidData)?,
        PidfdFlags::NONBLOCK,
    )?;
    process::pidfd_send_signal(&pidfd, Signal::STOP)?;
    while !fs::read_to_string(format!("/proc/{pid}/status"))?
        .lines()
        .any(|line| line.starts_with("State:") && line.contains("T (stopped)"))
    {
        if boottime_ns() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let fd = &sockets[0];
    net::sockopt::set_socket_send_buffer_size(fd, 4096)?;
    let mut packets = 0;
    loop {
        match ipc::send(fd, b"bounded-fixture-channel-pressure") {
            Ok(()) => {
                packets += 1;
                if packets > 64 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                println!(
                    "ACTUAL-CAMERA-CHANNEL-FULL worker={pid} packets={packets} errno={}",
                    error.raw_os_error().unwrap_or(-1)
                );
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
}
