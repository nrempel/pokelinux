#![forbid(unsafe_code)]
mod service;
fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.as_slice() {
        [_, mode, socket] if mode == "--serve" => service::serve(socket),
        [_, mode, parent, until, dev, ino] if mode == "--collect" => {
            service::collect(parent, until, dev, ino)
        }
        _ => Err(std::io::ErrorKind::InvalidInput.into()),
    }
}
