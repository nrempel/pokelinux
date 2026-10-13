# Native package on virtual graphics

The local validation VM boots an Arch ARM kernel and runs the current native
Phone package on Hyprland through virtio-gpu/virgl. The QEMU host forces software
rendering. It exposes no physical GPU, phone, network interface or KVM device.
This extends the Arch userspace package tests to a real guest kernel, DRM device,
compositor and Wayland window. It does not test Pixel PowerVR or Android.

The runs below are historical evidence at their recorded geometry. Future UI
runs and previews must first qualify the [Pixel 10 display profile](native-phone-display-profile.md):
1080 ×2424 native portrait pixels,422PPI and logical density420. The previous
320/800 fixtures are not suitable for a new preview. Capture the native output
buffer without resizing and keep virtual physical dimensions separate from
measured device hardware.

The separately selected Pixel10 profile now passes three actual ARM groups:
nominal physical metadata reaches Wayland/Qt, a native1080 ×2424 SHM buffer matches
its decoded PNG without resize, and normal adapter unload restores the original
virtual extent. The selected surface is a measurement probe; these groups do not
qualify a new phone-app/Source/PAM preview. See the
[profile report](native-phone-display-profile.md) and
[tools](../tools/native-phone-display-profile/README.md) for the current run path.

## Historical build and run

Run from the project root. Build `sim/Dockerfile.arch-local` and
`sim/Dockerfile.omarchy-local` first, following their existing local image inputs.
Then build `sim/Dockerfile.qemu-local` as `pixel-phone-qemu-native:20261012`.
Export a stopped container from `pixel-phone-omarchy-native:20261012` into
`sim/local-vm/rootfs.tar`; remove that export container afterwards.

Run `tools/prepare-native-vm.sh` in the QEMU image with the project mounted
read-only at `/workspace` and `sim/local-vm` writable at `/vm`. It refuses to
overwrite an existing guest disk. It extracts only this task's image, removes
the Docker marker, disables guest passwords, installs a bounded test service,
and creates a sparse ext4 image without a host loop mount.

Run `tools/run-native-vm.sh` in the QEMU image with:

- `--network none --cpus 4 --memory 6g --pids-limit 512`.
- Read-only project at `/workspace` and `sim/local-vm` at `/vm`.
- A fresh `/results` on the container's Linux filesystem.
- A writable host output directory at `/output`.

Copy the runner to `/tmp/run-native-vm.sh` before executing it. Use an EXIT trap
to copy `/results/.` to `/output/`, and save stdout/stderr as `serial.log`.
Do not bind a macOS directory directly at `/results`: QEMU's `mapped-xattr`
metadata is unsupported by that share. Each run uses a temporary disk snapshot;
the prepared base disk stays read-only. QEMU has a 900-second host timeout.
The runner checks the guest JSON after shutdown and fails if it is absent or
does not explicitly report success with `hardware_verified: false`.

`sim/guest/native_graphics.py` installs package 0.1.4 with pacman, checks installed
files, starts seatd and Hyprland, requires an active virtual monitor, starts the
installed Qt app on a session bus, finds its native Wayland window, and captures
`phone-wayland.png`. It starts without enrollment, so bridge-unavailable state
is expected. It does not claim call-command validation.

The extended run `native-vm-05` also loads the pinned Omarchy shell, applies the
existing explicit Qt 6.12 namespace patch, waits for plugin discovery, and
starts the native app on the same session bus. It discovers 37 plugins. Two
synthetic theme directories are selected through Omarchy's current-theme
symlink; the same app PID and Wayland window follow both selections. A lower
window region matches the expected dark and light background pixels exactly.
Evidence and hashes: `artifacts/rust-native-local/omarchy-native-theme-result.json`.
These are fixture palettes, not an invocation of the complete Omarchy theme
activation command. The shell's runtime theme IPC still needs that test.

The source fixture contains only part of Omarchy. The shell warns about missing
helper programs and PipeWire configuration. This run establishes shell loading,
plugin discovery and native-app theme following, not a complete functioning
desktop. The expanded image `pixel-phone-omarchy-session:20261012` adds PipeWire,
WirePlumber, UWSM and GTK's real desktop launcher for the next guest build.

## Real desktop launch and official themes

`native-vm-07` uses the expanded image exported to `sim/session-vm`, the complete
pinned Omarchy archive, and the guest's real `user@1000.service` and session bus.
The installed Phone desktop entry launches through `uwsm-app -- gtk-launch`.
Its process cgroup is under `app.slice/app-graphical.slice`, and its window is
native Wayland. PipeWire and WirePlumber start through the user manager.

The actual `omarchy-theme-set` commands apply Tokyo Night and Catppuccin Latte.
The same Phone PID and window render both backgrounds with a 100% match in the
sampled empty body region. The compositor screenshots also show the shell bar
and background changing. The host runner exits zero. Evidence and source hashes
are in `artifacts/rust-native-local/omarchy-session-result.json`.

That run was not a clean result for every theme hook: both command logs report that
`/usr/bin/omarchy-theme-set-browser-policy` is absent. The full source supplies
that helper and `etc/sudoers.d/omarchy-theme-browser`; the latter grants wheel
only the fixed helper with exactly six lowercase hex digits. The expanded
guest lacked sudo/visudo too. `native-vm-08` installs the signed sudo package
offline and installs the actual upstream helper and rule at their canonical
paths. `visudo` validates the rule; the permitted color command succeeds without
a password and an invalid argument is rejected. Both official theme commands
then finish with empty output logs and the same-window pixel checks pass.
No browser is installed, so browser repaint itself is not tested. BlueZ, a
network backend and UPower remain unprovisioned; the shell reports those
omissions explicitly.

The same run starts the installed bridge unit without enrollment. It exits 1
before publishing a socket, enters the unit's configured auto-restart state,
and can be stopped cleanly. This proves rejection of missing enrollment, not
successful pairing or Android connectivity.

`native-vm-06` timed out while collecting launcher output through a pipe. The
successful run captures command output in a file, so a launched child retaining
stdout does not keep the harness waiting for pipe EOF. The guest journal is now
retained. A transient shell startup ping error may remain in
`failed-command.log`; the final result, not that last diagnostic alone, records
whether the bounded readiness check succeeded.

## Initial failures retained

- `artifacts/rust-native-local/native-vm-01/serial.log`: kernel and virtio GPU
  booted; macOS-backed results mount rejected xattr operations.
- `artifacts/rust-native-local/native-vm-02/`: guest test passed and captured the
  app, but the host runner exited 127 because its source changed while Bash was
  reading it. This is not a clean end-to-end runner pass. Subsequent runs copy
  the runner first.

## Remaining integration

The clean run in `artifacts/rust-native-local/native-vm-03/` passed with host
exit 0. Arch kernel 7.2.10 created `card0` and `renderD128`; Hyprland reported
an active 800x600 virtual monitor and a visible `org.omarchy.Phone` window with
`xwayland: false`. Pacman reported six package files with zero altered files.
`artifacts/rust-native-local/native-vm-result.json` records source, package and
output hashes and the exact runner image identity.

The complete Omarchy installation, menu interaction and enrolled Android command
path still need tests inside this VM. Launch through the actual desktop resolver and official theme
commands is now covered within the limits above.
The existing older Python Phone plugin is not used by this test and is not the
target implementation. Hardware modem/IMS, audio, PowerVR scanout and suspend
remain separate Pixel gates. The full smartphone goal remains active.

## Installed service lifecycle (native-vm-10)

The runner exited 0. With disposable private enrollment files and an unavailable
loopback Android endpoint, the packaged user service starts and answers local
RPC. Runtime and state directories are owned by the user with mode 0700; the
socket and SQLite journal have mode 0600. The unit reports NoNewPrivileges=yes.
A real systemd restart preserves the journal epoch and changes the service
session. Stop removes the socket. Missing enrollment still fails before socket
publication. Both official theme commands produce empty logs and the same Phone
window matches each expected background color.

Evidence: artifacts/rust-native-local/native-vm-10/result.json and serial.log.
This proves installed Linux lifecycle behavior, not authenticated Android
connectivity, pairing UX, cellular operation or Pixel hardware behavior.
