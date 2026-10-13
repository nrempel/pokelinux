//! Authored actual-exec/wait control for the isolated early-init probe.
fn main() {
    let pid = rustix::process::getpid().as_raw_nonzero().get();
    let parent = rustix::process::getppid().map(|p| p.as_raw_nonzero().get());
    if pid <= 1 || parent != Some(1) {
        std::process::exit(2);
    }
    println!("POKELINUX_CHILD_EXEC pid={pid} parent=1");
    std::process::exit(37);
}
