//! Offline test fixture only; never packaged. Reserves but never dispatches.
#[cfg(target_os = "linux")]
fn main() {
    use omarchy_phone_core::{decode_request, ledger::Ledger};
    let path = std::env::args_os()
        .nth(1)
        .expect("fresh journal directory required");
    let path = std::path::Path::new(&path);
    assert!(!path.exists(), "fixture refuses existing state");
    let mut ledger = Ledger::open(path).unwrap();
    let frame = format!(
        "{}\n",
        serde_json::json!({"v":1,"id":"cccccccccccccccccccccccccccccccc","epoch":ledger.epoch(),"session":"dddddddddddddddddddddddddddddddd","command":{"kind":"dial","generation":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","offer":"ffffffffffffffffffffffffffffffff","number":"5550100"}})
    );
    ledger
        .reserve(&decode_request(frame.as_bytes()).unwrap())
        .unwrap();
}
#[cfg(not(target_os = "linux"))]
fn main() {
    panic!("Linux test fixture required");
}
