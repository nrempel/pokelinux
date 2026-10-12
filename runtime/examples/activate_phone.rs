//! Offline integration harness for the production fixed-app activation path.
#[cfg(target_os = "linux")]
fn main() {
    let result = std::env::var("DBUS_SESSION_BUS_ADDRESS")
        .map_err(|_| ())
        .and_then(|address| {
            futures_lite::future::block_on(omarchy_phone_core::notification_bus::activate_phone(
                &address,
            ))
            .map_err(|_| ())
        });
    if result.is_err() {
        eprintln!("Phone activation failed");
        std::process::exit(1);
    }
}
#[cfg(not(target_os = "linux"))]
fn main() {
    std::process::exit(1);
}
