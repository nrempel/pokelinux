//! Interactive test driver, not an installed connectivity service.
use pokelinux_connectivity::Endpoint;
use pokelinux_power::clock::{BootTime, Clock};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
struct ExpiredDuringRead(AtomicUsize);
impl Clock for ExpiredDuringRead {
    fn now(&self) -> BootTime {
        BootTime::from_elapsed(Duration::from_secs(
            if self.0.fetch_add(1, Ordering::SeqCst) < 3 {
                0
            } else {
                2
            },
        ))
    }
}
use serde_json::json;
use std::io::{BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::args()
        .nth(1)
        .ok_or("explicit test bus address required")?;
    let mut endpoint = None;
    for line in std::io::stdin().lock().lines() {
        let request: serde_json::Value = serde_json::from_str(&line?)?;
        let response = match request["op"].as_str() {
            Some("pin") => match Endpoint::discover(&address) {
                Ok(value) => {
                    let response = json!({"ok":true,"owner":value.owner()});
                    endpoint = Some(value);
                    response
                }
                Err(error) => json!({"ok":false,"error":error.to_string()}),
            },
            Some("snapshot_expired") => match endpoint.as_ref().ok_or("no_pinned_owner") {
                Ok(value) => {
                    match value.snapshot_with_clock(&ExpiredDuringRead(AtomicUsize::new(0))) {
                        Ok(snapshot) => json!({"ok":true,"snapshot":snapshot}),
                        Err(error) => json!({"ok":false,"error":error.to_string()}),
                    }
                }
                Err(error) => json!({"ok":false,"error":error}),
            },
            Some("snapshot") => match endpoint.as_ref().ok_or("no_pinned_owner") {
                Ok(value) => match value.snapshot() {
                    Ok(snapshot) => json!({"ok":true,"snapshot":snapshot}),
                    Err(error) => json!({"ok":false,"error":error.to_string()}),
                },
                Err(error) => json!({"ok":false,"error":error}),
            },
            _ => json!({"ok":false,"error":"invalid_test_operation"}),
        };
        println!("{response}");
        std::io::stdout().flush()?;
    }
    Ok(())
}
