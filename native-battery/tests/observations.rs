//! Real filesystem boundaries plus controlled absolute BOOTTIME policy samples.
use pokelinux_battery::{BatteryModel, COLLECTION_NS, VALIDITY_NS, read_supply};
use pokelinux_phone_state::{BatteryObservation, ChargingStatus};
use std::{
    fs,
    os::unix::fs::symlink,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "pokelinux-battery-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::create_dir(p.join("class")).unwrap();
        fs::create_dir(p.join("device")).unwrap();
        // Actual Linux class entries are symlinks; refusing them rejects real sysfs.
        symlink("../device", p.join("class/test_battery")).unwrap();
        Self(p)
    }
    fn write(&self, body: &str) {
        fs::write(self.0.join("device/uevent"), body).unwrap();
    }
    fn read(&self) -> std::io::Result<BatteryObservation> {
        read_supply(&self.0.join("class"), "test_battery")
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn packet(capacity: &str, status: &str) -> String {
    format!(
        "POWER_SUPPLY_NAME=test_battery\nPOWER_SUPPLY_TYPE=Battery\nPOWER_SUPPLY_PRESENT=1\nPOWER_SUPPLY_CAPACITY={capacity}\nPOWER_SUPPLY_STATUS={status}\nPOWER_SUPPLY_VOLTAGE_NOW=3300000\n"
    )
}
#[test]
fn real_class_symlink_reads_zero_full_and_charging_without_writes() {
    let t = Tree::new();
    for (capacity, status, expected) in [
        ("0", "Discharging", ChargingStatus::Discharging),
        ("73", "Charging", ChargingStatus::Charging),
        ("100", "Full", ChargingStatus::Full),
        ("50", "Not charging", ChargingStatus::NotCharging),
        ("25", "Unknown", ChargingStatus::Unknown),
    ] {
        let before = packet(capacity, status);
        t.write(&before);
        assert_eq!(
            t.read().unwrap(),
            BatteryObservation {
                capacity_percent: capacity.parse().unwrap(),
                status: expected
            }
        );
        assert_eq!(
            fs::read_to_string(t.0.join("device/uevent")).unwrap(),
            before
        );
    }
}
#[test]
fn malformed_absent_duplicate_and_nonbattery_values_are_unavailable() {
    let t = Tree::new();
    for invalid in [
        packet("101", "Charging"),
        packet("-1", "Full"),
        packet("NaN", "Full"),
        packet("50", "Overheating"),
        packet("50", "Full").replace("PRESENT=1", "PRESENT=0"),
        packet("50", "Full").replace("TYPE=Battery", "TYPE=Mains"),
        packet("50", "Full") + "POWER_SUPPLY_CAPACITY=51\n",
        packet("50", "Full").replace("POWER_SUPPLY_STATUS=Full\n", ""),
        "x".repeat(4097),
    ] {
        t.write(&invalid);
        assert!(t.read().is_err(), "accepted malformed packet {invalid:?}");
    }
    fs::remove_file(t.0.join("device/uevent")).unwrap();
    assert!(t.read().is_err());
    assert!(read_supply(&t.0.join("class"), "../device").is_err());
    assert!(read_supply(&t.0.join("class"), "/device").is_err());
}
#[test]
fn suspend_expiry_and_late_completion_cannot_keep_a_percentage() {
    let mut model = BatteryModel::default();
    let start = 20_000_000_000;
    let first = model.begin(start).expect("one collection must be admitted");
    assert_eq!(first.until_ns, start + COLLECTION_NS);
    assert!(
        model.begin(start + 1).is_none(),
        "pending collection cannot be replaced"
    );
    let value = BatteryObservation {
        capacity_percent: 73,
        status: ChargingStatus::Charging,
    };
    assert!(model.complete(first, start + 5, Some(value)));
    let view = model.view(start + 6);
    assert_eq!(view.observation, Some(value));
    assert_eq!(view.until_ns, start + VALIDITY_NS);
    assert_eq!(
        model.view(start + VALIDITY_NS).observation,
        None,
        "suspend-sized BOOTTIME jump must expire without a reader reply"
    );
    let second = model.begin(start + VALIDITY_NS + 1).unwrap();
    assert!(
        !model.complete(first, start + VALIDITY_NS + 2, Some(value)),
        "old collection must not affect successor"
    );
    assert!(
        !model.complete(second, second.until_ns, Some(value)),
        "deadline is inclusive and cannot be renewed by late success"
    );
    assert_eq!(model.view(second.until_ns).observation, None);
    let third = model.begin(second.until_ns + 1).unwrap();
    assert!(model.complete(third, second.until_ns + 2, Some(value)));
    let fourth = model.begin(second.until_ns + 3).unwrap();
    assert!(model.complete(fourth, second.until_ns + 4, None));
    assert_eq!(
        model.view(second.until_ns + 5).observation,
        None,
        "read failure must clear old percentage immediately"
    );
    let fifth = model.begin(second.until_ns + 6).unwrap();
    model.withdraw();
    assert!(
        !model.complete(fifth, second.until_ns + 7, Some(value)),
        "withdrawal must fence queued success"
    );
    assert_eq!(model.view(second.until_ns + 8).observation, None);
}
