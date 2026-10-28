//! Trusted worker entries. Real service installation/app permission wiring is pending.
fn main() -> std::io::Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if matches!(
        arguments.first().map(String::as_str),
        Some("profile-up" | "profile-down")
    ) {
        return pokelinux_connectivity_control::fixed_address::run_cli(&arguments);
    }
    if arguments.first().map(String::as_str)
        != Some(pokelinux_process_owner::bootstrap::WORKER_SELECTOR)
    {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    pokelinux_connectivity_control::service::worker(&arguments[1..])
}
