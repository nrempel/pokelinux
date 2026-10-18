//! Private cfg(test) collector substitution. Normal binary has no such modes.
#![forbid(unsafe_code)]
#[path = "../src/service.rs"]
mod service;
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};
fn fixture_collect(parent: &str, until: &str, directory: &str) -> io::Result<()> {
    let (parent, until) = service::child_setup(parent, until)?;
    let directory = Path::new(directory);
    let pid = std::process::id();
    let pending = directory.join(format!("ready-{pid}.pending"));
    fs::write(
        &pending,
        serde_json::to_vec(&serde_json::json!({
            "pid": pid, "parent": parent.as_raw_nonzero().get(), "until_ns": until,
        }))?,
    )?;
    fs::rename(pending, directory.join(format!("ready-{pid}.json")))?;
    let mut byte = [0];
    fs::File::open(directory.join("release-result"))?.read_exact(&mut byte)?;
    service::child_check(parent, until)?;
    // Authored counts are ONLY for Reader/process lifecycle controls.
    std::io::stdout()
        .lock()
        .write_all(b"{\"local_interfaces\":3,\"address_records\":7}\n")?;
    std::io::stdout().flush()?;
    fs::write(directory.join(format!("written-{pid}")), b"written")?;
    fs::File::open(directory.join("release-exit"))?.read_exact(&mut byte)?;
    service::child_check(parent, until)
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.as_slice() {
        [_, mode, socket, directory] if mode == "--fixture-serve" => {
            service::serve_fixture(socket, Path::new(directory))
        }
        [_, mode, parent, until, directory] if mode == "--fixture-collect" => {
            fixture_collect(parent, until, directory)
        }
        [_, mode, socket] if mode == "--serve" => service::serve(socket),
        [_, mode, parent, until, dev, ino] if mode == "--collect" => {
            service::collect(parent, until, dev, ino)
        }
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}
