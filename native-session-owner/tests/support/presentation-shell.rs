//! Native exec target for the isolated process contract; this is not Quickshell/Qt.
#![forbid(unsafe_code)]
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let exe = std::fs::read_link("/proc/self/exe").unwrap();
    println!(
        "{{\"pid\":{},\"exe\":{:?},\"args\":{:?},\"display\":{:?},\"signature\":{:?},\"keep\":{:?},\"unexpected\":{}}}",
        std::process::id(),
        exe,
        args,
        std::env::var("WAYLAND_DISPLAY").unwrap(),
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap(),
        std::env::var("PRESENTATION_TEST_KEEP").unwrap(),
        std::env::var_os("PRESENTATION_TEST_UNEXPECTED").is_some()
    );
}
