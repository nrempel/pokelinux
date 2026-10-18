# ARM kernel brightness probe

`probe.rs` is the exact original Rust source compiled and executed in the
[generic ARM kernel qualification](../../docs/evidence/native-brightness-arm-kernel-provenance.json).
It uses the production `pokelinux-brightness` Client and root-only actor socket.
It is test infrastructure, separate from the installed service and Settings app.

The qualified build adds a temporary `[[bin]]` entry named
`backlight-kernel-probe` to an isolated copy of `native-brightness/Cargo.toml`,
with `path` pointing to a copied `probe.rs`. Use that crate's locked dependencies,
normal features and matching ARM64 toolchain. The published file is byte-identical
to that compiled input; no build or deployment wrapper is added here.

Run only in the disposable test guest with the explicit LinuxBacklight service
and authored `pokelinux-virtual-backlight` device. The executable takes an action
(`initial`, `set75`, `set25`, or `unavailable`) and one absolute CLOCK_BOOTTIME
cutoff, at most one second ahead. Each invocation uses that same cutoff for
Inspect and its optional single Set, on the same verified connection. It neither
reconnects nor retries a Set. A fresh invocation/connection is used for each action.

The GPIO backlight has max1.75% therefore requests raw1/current100%, and25%
requests raw0/current0%. Its actual_brightness is the kernel's requested-value
fallback, not independent light output. The guest separately reads sysfs,
checks the same service Main, removes only the authored virtual device, then
checks Unavailable and normal PID1 Stop. The preceding identical no-device
baseline reaches a verified Inspect response before the positive assertion fails.

No phone, host sysfs, graphical app, Source, cold autostart, continuous brightness
or physical luminance is qualified by this fixture.
