//! Private installed-ARM readonly actor fixture; no Set, path selection or UI.
use pokelinux_brightness::client::{Client, Event};
use rustix::time::{ClockId, clock_gettime};
use serde_json::json;
use std::{fs::File, io, time::Duration};
fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 2);
    assert_eq!(rustix::process::getuid().as_raw(), 0);
    assert_eq!(rustix::process::getgid().as_raw(), 0);
    let until = pokelinux_brightness::protocol::decimal(&args[1])
        .expect("canonical original BOOTTIME deadline");
    let start = now();
    assert!(start < until && until - start <= 1_000_000_000);
    let class = File::open("/sys/class/backlight")?;
    assert_eq!(rustix::fs::fstatfs(&class)?.f_type as u64, 0x6265_6572);
    let mut client = Client::new("/run/pokelinux-brightness/actor.sock".into())?;
    assert!(client.inspect(until)?, "one explicit readonly Inspect");
    loop {
        assert!(now() < until, "one original caller deadline");
        if let Some(event) = client.poll() {
            assert!(
                matches!(event, Event::Observed),
                "actual credentialed Inspect reply"
            );
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let view = client.snapshot();
    assert!(view.connected && !view.pending && view.revision > 0);
    assert!(view.requested_percent.is_none() && !view.last_set_uncertain);
    assert!(now() < until);
    println!(
        "{}",
        json!({"schema":1,"actual_typed_Client_Inspect":true,
        "connected":view.connected,"pending":view.pending,"revision":view.revision,
        "observation":view.observation,"until_ns":view.until_ns.to_string(),
        "requested_percent":view.requested_percent,
        "last_set_uncertain":view.last_set_uncertain,
        "original_until_ns":until.to_string()})
    );
    Ok(())
}
