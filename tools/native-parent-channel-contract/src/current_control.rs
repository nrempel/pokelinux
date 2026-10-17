//! Genuine original/current API process control: no retained-channel declarations,
//! feature, shim, synthetic READY, or guessed future method appears in this bin.
#![forbid(unsafe_code)]
mod common;
use common::*;
use pokelinux_process_owner::{
    bootstrap::{BoundParent, WORKER_SELECTOR},
    supervisor::{StartError, Supervisor},
};
use std::{
    io::{self, Write},
    path::PathBuf,
};

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|value| value == WORKER_SELECTOR) {
        let bound = BoundParent::from_stdin()?;
        require(
            std::fs::read_link("/proc/self/fd/0")? == std::path::Path::new("/dev/null"),
            "fd0 escaped bootstrap",
        )?;
        writeln!(io::stdout(), "CURRENT {}", bound.identity().hex())?;
        io::stdout().flush()?;
        // BoundParent stays on the worker main thread until actual Stop.
        loop {
            std::thread::park();
        }
    }
    require(
        args.len() == 1,
        "usage: parent-channel-current-control NEW_PRIVATE_RUNTIME",
    )?;
    let runtime = PathBuf::from(&args[0]);
    private_dir(&runtime)?;
    let began = now();
    let operation_until = plus(began, 4_000_000_000)?;
    let cleanup_until = plus(began, 6_000_000_000)?;
    let mut owner = Supervisor::new(runtime.join("admission"));
    let mut worker = command("current-body", &[])?;
    let identity = start(&mut owner, &mut worker, cleanup_until)?;
    let result = (|| -> io::Result<()> {
        require(
            owner.occupied() && !owner.admits(identity),
            "initializing child already admitted",
        )?;
        ready(&mut owner, identity, operation_until)?;
        let mut output = Vec::new();
        require(
            line(&mut owner, &mut output, operation_until)?
                == format!("CURRENT {}\n", identity.hex()),
            "actual BoundParent body/identity absent",
        )?;
        owner.stop()?;
        require(
            owner.occupied() && !owner.admits(identity),
            "Stop released custody or retained admission",
        )?;
        require(
            owner.start_worker(&mut worker) == Err(StartError::Occupied),
            "unwaited Stop allowed a successor",
        )?;
        killed(reap(&mut owner, cleanup_until)?)?;
        journal_unchanged(&mut owner, &mut worker, &runtime, identity)?;
        println!(
            "CURRENT-CONTROL actual-Supervisor READY/GO BoundParent fd0-null Stop wait9 durable-refusal"
        );
        Ok(())
    })();
    dispose(&mut owner, cleanup_until)?;
    result
}
