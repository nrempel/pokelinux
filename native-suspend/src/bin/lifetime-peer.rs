// Synthetic Unix-stream lifetime peer used by actual-kernel integration tests.
// This is not a retained-vendor or Binder client.
use std::io::Read;
use std::os::unix::net::UnixStream;
fn main() -> std::io::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or_else(|| std::io::Error::other("socket path required"))?;
    let mut stream = UnixStream::connect(path)?;
    let mut byte = [0; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => return Ok(()),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}
