//! Real descriptor probe for tests/pidfd_read_count.py. No synthetic procfs.
use pokelinux_kernel_identity::ProcessView;
use rustix::process::{Pid, PidfdFlags, pidfd_open};
use std::{io, process::Command};

fn main() -> io::Result<()> {
    // Cargo's ordinary test selection also exercises both live comparisons.
    let mode = std::env::args().nth(1).unwrap_or_else(|| "both".into());
    let mut child = Command::new("/bin/sleep").arg("30").spawn()?;
    let result = (|| {
        let owned = pidfd_open(
            Pid::from_raw(child.id() as i32).unwrap(),
            PidfdFlags::empty(),
        )?;
        let duplicate = rustix::io::fcntl_dupfd_cloexec(&owned, 3)?;
        let view = ProcessView::open()?;
        for (name, right) in [("same", &owned), ("distinct", &duplicate)] {
            if (mode == name || mode == "both") && !view.matches(&owned, right, child.id())? {
                return Err(io::Error::other("live retained identity was refused"));
            }
        }
        println!("probe_pid={}", std::process::id());
        Ok(())
    })();
    // Reap the actual child even when any identity operation failed.
    let kill = child.kill();
    let wait = child.wait();
    result?;
    kill?;
    wait?;
    Ok(())
}
