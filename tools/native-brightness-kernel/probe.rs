//! Private native kernel fixture: real Rust Client, root actor role, no UI/Source.
use pokelinux_brightness::client::{Client, Event, Snapshot};
use rustix::time::{ClockId, clock_gettime};
use serde_json::json;
use std::{fs, io, path::Path, time::Duration};
const SOCKET: &str = "/run/pokelinux-brightness/actor.sock";
const CLASS: &str = "/sys/class/backlight";
const DEVICE: &str = "/sys/class/backlight/pokelinux-virtual-backlight";
fn now() -> u64 {
    let t = clock_gettime(ClockId::Boottime);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn event(client: &mut Client, until: u64) -> Event {
    loop {
        assert!(now() < until, "one original Client deadline");
        if let Some(e) = client.poll() {
            return e;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn raw(name: &str) -> u32 {
    let value =
        fs::read_to_string(Path::new(DEVICE).join(name)).expect("independent real sysfs read");
    let digits = value.strip_suffix('\n').expect("kernel newline");
    assert!(!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
    digits.parse().expect("bounded independent kernel value")
}
fn positive(view: &Snapshot, requested: u32) {
    assert!(view.connected && !view.pending && view.revision > 0);
    let obs = view
        .observation
        .expect("actual LinuxBacklight device unavailable after verified daemon reply");
    assert_eq!(
        (obs.max, obs.requested, obs.actual),
        (1, requested, Some(requested)),
        "binary gpio-backlight and explicit core fallback"
    );
    assert_eq!(
        (
            raw("max_brightness"),
            raw("brightness"),
            raw("actual_brightness")
        ),
        (1, requested, requested)
    );
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    assert_eq!(rustix::process::getuid().as_raw(), 0);
    let until = pokelinux_brightness::protocol::decimal(&args[2])
        .expect("canonical original BOOTTIME cutoff");
    let start = now();
    assert!(start < until && until - start <= 1_000_000_000);
    let class = fs::File::open(CLASS)?;
    assert_eq!(rustix::fs::fstatfs(&class)?.f_type as u64, 0x6265_6572);
    let mut client = Client::new(SOCKET.into())?;
    assert!(client.inspect(until)?);
    assert!(
        matches!(event(&mut client, until), Event::Observed),
        "actual credentialed daemon Inspect reply"
    );
    let before = client.snapshot();
    println!(
        "{}",
        json!({"phase":"actual-inspect","connected":before.connected,"pending":before.pending,"observation":before.observation,"revision":before.revision,"original_until_ns":until})
    );
    match args[1].as_str() {
        "initial" => positive(&before, 0),
        "unavailable" => {
            assert!(before.connected && !before.pending && before.observation.is_none());
            assert!(
                !Path::new(DEVICE).exists(),
                "missing device, not a fabricated optional attribute"
            );
        }
        action @ ("set75" | "set25") => {
            let (percent, initial, expected) = if action == "set75" {
                (75, 0, 1)
            } else {
                (25, 1, 0)
            };
            positive(&before, initial);
            assert!(
                client.set(1, percent, before.revision, until)?,
                "one explicit Set on the same verified connection"
            );
            assert!(client.snapshot().pending && client.snapshot().observation.is_none());
            match event(&mut client, until) {
                Event::Completed { id: 1, reply } => {
                    assert!(reply.ok && !reply.pending && !reply.last_set_uncertain);
                    assert_eq!(reply.requested_percent, Some(percent));
                }
                _ => panic!("matching completed native kernel Set required"),
            }
            positive(&client.snapshot(), expected);
            println!(
                "{}",
                json!({"phase":"actual-Set","percent":percent,"raw_requested":expected,"current_percent":expected*100,"optional_actual":expected,"actual_is_core_fallback":true,"same_connection_incarnation":client.incarnation()})
            );
        }
        _ => panic!("fixed probe action required"),
    }
    assert!(now() < until);
    Ok(())
}
