//! Worker entry only. Real service installation/app permission wiring is pending.
fn main() -> std::io::Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().map(String::as_str)
        != Some(pokelinux_process_owner::bootstrap::WORKER_SELECTOR)
    {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    pokelinux_connectivity_control::service::worker(&arguments[1..])
}
