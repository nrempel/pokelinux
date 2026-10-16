# Native x86 session runtime

The native CPU execution environment on Harbour now passes one actual offline
KVM boot. The production Rust payload also compiles for x86_64 from the current
public source. This prepares the full session composition test; it does not
qualify the compositor, normal Sounds app, Source, audio or phone hardware.
[Hash-only evidence](evidence/native-session-x86-runtime-provenance.json) binds the
inputs, builds, execution and qualifications. Private logs and VM artifacts stay
private.

## Runtime and authenticated guest

The existing graphics runtime is actually Debian13 amd64; its historical image
name does not establish an Arch or Hyprland installation. A derived image adds
matching-family QEMU x86, SeaBIOS, ipxe-qemu and genisoimage. Actual package
inventories and tool reports bind QEMU `1:10.0.13+ds-0+deb13u1`; the original
runtime and its renderer dependencies remain intact. An earlier Dockerfile
setup failure remains failed and is not a production RED.

The official `Arch-Linux-x86_64-cloudimg-20261015.611680.qcow2` is576,680,960 bytes.
Both its image signature and signed checksum were verified with the Arch CI key
published in the [pinned arch-boxes README](https://github.com/archlinux/arch-boxes/blob/7f733af26fe9a0c93fdabba13e7803cbe803374a/README.md).
The original is read-only, has no external backing/data file, and retains its2GiB
virtual disk. Only a fresh disposable child overlay receives writes. This is an
x86 Arch kernel, not Google's matched Pixel kernel.

## Actual boot and closure

A separate isolated API query first returns KVM API12. The actual VM then uses
explicit `-accel kvm -cpu host`, with no TCG fallback, no network and only the
host `/dev/kvm` device admitted to its container. Stock BIOS/GRUB, kernel,
initramfs and systemd discover a local NoCloud seed. No kernel extraction,
custom PID1, login/password or package installation runs.

The private bootstrap runtime-masks the offline time-wait service. This is an
explicit fixture change, not an unchanged boot-policy claim or installed phone
policy. The stock no-NIC network-online wait still consumes about120 seconds.
Actual cloud-init26.2 reaches the root bootstrap, which observes Linux
`7.2.9-arch1-1`, x86_64 and UID0. Its final receipt is flushed before it returns;
normal cloud-init power_state then powers off. The guest does not claim a
separately owned shutdown-child wait.

The same authenticated QMP socket identifies the owned QEMU process, confirms
KVM enabled/present, and later records `SHUTDOWN` with guest true and reason
`guest-shutdown`, followed by socket EOF and actual QEMU wait0. Serial identity
and the guest-result hash match the retained file. Seven host CLI waits and14
PIPE EOFs, four boot children, five guest commands and one early command all
close within their original cutoffs. The two CLI exit1 receipts establish exact
name/CID absence; they are expected. QEMU and Docker CLI return0, the same daemon
reports the exact container absent, and selected input hashes remain unchanged
under the shared lock. The complete host envelope takes132.46 seconds against
its240-second work/270-second closure limits; QEMU retains180/210 seconds.

## Matching native Rust payload

All83 selected production Cargo/source/lock files match public commit
`387fe72bc375def6dacd79fae8df2758f537e58a`; the four Sounds exec-fixture inputs
are separately bound. Strict formatting, selected-bin Clippy and native builds
pass for appearance, audio and sounds-exec. Packaging renames
`pokelinux-appearance` to `native-appearance` without changing bytes. The existing
host session owner and fixed Sounds client are separately hash-verified and
bound to that matching source. All five selected binaries are ELF machine62.
Ten CLI waits0 and20 PIPE EOFs close within their original bounds, with exact
container absence and no shared writable target/cache.

This compilation reruns no behavioral tests or Qt checks. `audio-client-test`
is a Qt/CMake target and was not treated as a Rust binary. The earlier32 Qt
checks are not current native-guest evidence. No production behavior changes,
new behavioral RED/GREEN result or screenshot follows from this checkpoint.

## Next integration

Acquire and verify an offline signed package closure for the original Hyprland,
Qt, Quickshell, PipeWire and build dependencies. Bind the actual loaded Hyprland
binary and SDK to the selected commit; a matching version label alone is
insufficient. Build the normal Qt app and plugins against that guest, then run
the unchanged four-step session composition: denied quiet launch, current grant
and real focused touch with PCM, null-focus background continuation with disabled
touch, and terminal Source loss without app/audio revival. Preserve per-app
Wayland, independent grants,1080×2424/422PPI/420 logical density, capture guards,
all graph/PCM assertions and the100ms lease.

Native x86 success would qualify that environment only. The earlier ARM Source
expiry remains failed and does not establish a TCG cause. ARM availability,
matched Google kernel integration, complete Omarchy shell/startup and every
physical Pixel/carrier gate remain open. No physical phone has been accessed.
