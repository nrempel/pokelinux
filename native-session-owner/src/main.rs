use std::{io, path::Path};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--compositor-bootstrap" {
        return pokelinux_session_owner::bootstrap_entry();
    }
    if args.len() == 1 && args[0] == "--display-context-worker" {
        return pokelinux_display::supervisor::worker_entry();
    }
    if args.first().is_some_and(|a| a == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&args[1..]);
    }
    if args.first().is_some_and(|a| a == "app-entry") {
        let entry = args[1..]
            .iter()
            .map(|arg| {
                arg.clone()
                    .into_string()
                    .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
            })
            .collect::<io::Result<Vec<_>>>()?;
        return pokelinux_lifecycle::service::entry(&entry);
    }
    if args.first().is_some_and(|a| a == "--stop") {
        let [_, runtime] = args.as_slice() else {
            return Err(io::ErrorKind::InvalidInput.into());
        };
        return pokelinux_session_owner::stop::request(Path::new(runtime));
    }
    let [path] = args.as_slice() else {
        return Err(io::ErrorKind::InvalidInput.into());
    };
    let config = pokelinux_session_owner::config::Config::load(Path::new(path))?;
    pokelinux_session_owner::runtime::run(&config)
}
