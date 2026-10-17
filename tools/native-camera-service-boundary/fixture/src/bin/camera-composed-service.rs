//! Separately built public synthetic camera adapter; normal services have no selector.
use pokelinux_camera::Generation;
use pokelinux_camera_service::worker::Supervisor;
use pokelinux_lifecycle::{boottime_ns, camera::Provider};
use std::{io, path::Path};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "--camera-worker") {
        if args.len() != 9 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let fault = args[8]
            .to_str()
            .ok_or(io::ErrorKind::InvalidInput)?
            .to_owned();
        // This constructor performs no driver open or Binder lookup.
        let mut backend = native_camera_service_boundary_fixture::backend::Deferred::new(fault)?;
        return pokelinux_camera_service::worker::entry(&args[1..8], &mut backend);
    }
    if args.len() != 6 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let gated = args[5] == "eagain";
    let adapter = if gated {
        std::ffi::OsString::from("none")
    } else {
        args[5].clone()
    };
    let before = native_camera_service_boundary_fixture::pressure::descriptors()?;
    let mut camera = Supervisor::spawn(
        Path::new(&args[4]),
        &std::env::current_exe()?,
        11030,
        Generation {
            session: 1,
            configuration: 1,
        },
        &[adapter],
    )?;
    if gated {
        native_camera_service_boundary_fixture::pressure::hold(&mut camera, &before)?;
    }
    let result = pokelinux_lifecycle::service::run_with_services(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
        None,
        None,
        None,
        None,
        Some(&mut camera),
        None,
        None,
    );
    if !camera.drain_until(boottime_ns().saturating_add(2_000_000_000)) {
        std::process::abort();
    }
    result
}
