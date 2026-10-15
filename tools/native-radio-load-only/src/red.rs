//! Genuine missing-containment selection against the unchanged production owner.
use pokelinux_process_owner::supervisor::Supervisor;
use std::{
    fs,
    io::{self, Read},
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::PathBuf,
    process::Command,
    time::Duration,
};

fn now() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) },
        0
    );
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]);
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let marker = root.join("outside-marker");
    fs::write(&marker, "public-outside-root-marker\n").unwrap();
    let socket = root.join("outside.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut owner = Supervisor::new(root.join("admission"));
    let id = owner
        .start(Command::new(&args[2]).args([
            "outside-unix",
            marker.to_str().unwrap(),
            socket.to_str().unwrap(),
        ]))
        .unwrap();
    let until = now() + 5_000_000_000;
    let mut output = Vec::new();
    let mut connection = None;
    let mut buffer = [0u8; 256];
    while now() < until && (!output.contains(&b'\n') || connection.is_none()) {
        match owner.read_output(&mut buffer) {
            Ok(n) => {
                output.extend_from_slice(&buffer[..n]);
                assert!(output.len() <= 1024);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("bounded child read: {e}"),
        }
        if connection.is_none() {
            match listener.accept() {
                Ok(value) => connection = Some(value),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => panic!("fixture accept: {e}"),
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let observation: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let connected = connection.is_some();
    let mut payload = String::new();
    if let Some((mut stream, _)) = connection {
        stream
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        stream.read_to_string(&mut payload).ok();
    }
    let occupied_before = owner.occupied();
    let stop_start = now();
    owner.stop().unwrap();
    let stop_ns = now() - stop_start;
    let fenced = !owner.admits(id);
    let cleanup_until = now() + 2_000_000_000;
    let mut status = None;
    while now() < cleanup_until && status.is_none() {
        status = owner.poll_reap().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    let reaped = status.is_some();
    let occupied_after = owner.occupied();
    let retry = owner.start(&mut Command::new(&args[2]));
    let retained_uncertainty = matches!(
        retry,
        Err(pokelinux_process_owner::supervisor::StartError::AdmissionUncertain)
    );
    let genuine_missing_containment = observation["outside_read"] == true
        && observation["unix_connect"] == true
        && connected
        && payload == "public-harmless-probe\n";
    let cleanup_passed =
        occupied_before && fenced && reaped && !occupied_after && retained_uncertainty;
    println!(
        "{}",
        serde_json::json!({"expected_red":genuine_missing_containment && cleanup_passed,"red_signature":"current_supervisor_allows_outside_root_read_and_unix_connect","desired_outside_read":false,"desired_unix_connect":false,"observation":observation,"independent_unix_accept":connected,"public_payload_matches":payload=="public-harmless-probe\n","existing_stop_ns":stop_ns,"actual_reaped":reaped,"occupied_after":occupied_after,"durable_uncertainty_retained":retained_uncertainty,"cleanup_passed":cleanup_passed,"registration_admission":false,"vendor_execution":false})
    );
    if !reaped {
        std::process::abort();
    }
    std::process::exit(if genuine_missing_containment && cleanup_passed {
        1
    } else {
        2
    });
}
