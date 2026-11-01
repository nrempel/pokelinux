# Installed ARM wireless cold-boot integration

The first combined installed boot fails before the normal desktop or permission
flow starts. This is generic ARM64/QEMU evidence on Harbour, with simulated radios;
no physical phone or Pixel kernel is involved.

The prepared image passes offline manifest, loader and unit checks. The actual
boot then reports failure of the Omarchy theme provisioning oneshot and the
retained hwsim provisioning service. Their dependent session and test dispatcher
do not start. Build and offline checks therefore do not qualify installed startup.

Source/command comparison identifies a definite wireless fixture mismatch: the
owner requires `ipv6.disable=1`, while this boot command omits it. The console does
not identify the first failing assertion. The theme provisioner requires the
selected user runtime directory, but its unit does not order itself after that
directory's service. The console confirms theme failure precedes runtime-directory
startup. This is a second concrete prerequisite mismatch; the exact theme error
is not captured. The prepared image also retains an enabled older appearance
service on the same socket as the selected normal owner; that competing service
must be disabled in the next image.

The test dispatcher also requires the target it is intended to observe, preventing
it from collecting service journals when that target fails. Its next revision must
run after the target job finishes, including failure, and retain explicit shutdown
ownership rather than inherited unconditional post-stop poweroff.

The failed VM was ended early by a signal through a verified QEMU pidfd. Its
existing parent recorded the actual wait; process handles closed, the outer
container was removed and its exact identity was checked absent. The failed
writable disk is preserved. This is infrastructure closure, **not** normal guest
shutdown or native-service/vendor cleanup evidence.

The intended acceptance remains two boots of one writable disk: actual UI consent,
explicit connection and packet exchange, app close/reopen, explicit disconnection
and quiet, then retained permission without automatic reconnection after reboot.
Neither boot's functional acceptance is qualified. The full smartphone and
physical-hardware gates remain open.

[Curated failed-run receipt](evidence/native-installed-wireless-coldboot-red.json).

## Corrected theme startup

A second fresh installed boot completes theme provisioning and starts the selected
normal appearance service after adding the explicit user runtime-directory
prerequisite. Image preparation also verifies the older appearance service is
disabled. This is actual ARM/PID1 integration evidence for the theme unit change;
it does not establish rendered UI or complete session startup.

The simulated-wireless service still fails, so the full acceptance remains RED.
The observer now starts after that failure. Its cleanup exposes another test
runner defect: it requires functional viewer success before service journal
collection/Stop, although the never-started viewer has already published a matching
no-future-writes fence. The next runner correction must separate those facts while
retaining functional success as a requirement for acceptance. This run closes its
VM/container infrastructure but does not qualify normal guest shutdown or native
cleanup. No permission or connection action runs.

[Corrected-startup receipt](evidence/native-installed-wireless-coldboot-theme.json).

## Wireless provisioning advances; desktop readiness remains open

Subsequent failed-startup runs separate the viewer cleanup fence from functional
success. They collect service journals and Stop the selected services. An actual
partial-read failure then identifies non-atomic bootstrap-report publication;
publishing through a complete temporary file and rename preserves strict JSON
validation and now records actual QEMU wait0 and guest-requested shutdown.

The wireless fixture's exact initial failure is a missing `ExecStopPost` key in
`systemctl show` output. Adding `--all` does not repair it: the empty complex
command array is still omitted. A separate read-only host control observes an
explicit empty array through the [typed systemd interface](https://www.freedesktop.org/software/systemd/man/262/org.freedesktop.systemd1.html).
The fixture now requires that exact type and empty array through an authenticated
ARM busctl dependency. The next actual ARM boot reaches fixture READY, starts the
connectivity service, and later records the fixture's provisioning-only closure
marker and PID1 exit0. This does not qualify a normal UI connection.

That boot fails later: the session owner returns `Kind(TimedOut)` with exit1 under
its original 120-second readiness deadline. This differs from PID1 classifying a
start as a timeout. The bare error cannot distinguish an earlier rejected or
expired compositor Source from missing authenticated Wayland readiness. Child
stdout/stderr are discarded, and compositor logs/ring artifacts were not retained.
The next run must retain those existing diagnostics. The hwsim capture also needs
to prioritize primary receipts before optional logs exhaust its existing bounds.

[Current startup evidence](evidence/native-installed-wireless-startup-progress.json).
No rendered UI, consent, connection flow, two-boot persistence or phone hardware
is qualified by this checkpoint.

## Preserve early compositor diagnostics

The next boot retains the original 4096-byte startup ring with only its header,
an empty Hyprland log, and no applied/ordered display-profile receipts. These
observations do not identify the timeout cause. The child stdout/stderr were
explicitly discarded, so errors before compositor file logging could be lost.

The native owner now inherits those streams; its installed unit explicitly routes
both to the journal. A real host child-exec regression first fails on missing
stderr, then passes for both streams alongside valid authored Source frames and
actual child/actor closure. All 29 relevant tests, formatting and targeted Clippy
pass. This is host process-boundary evidence, not original compositor execution.
Inherited destinations can backpressure the child; the owner adds no pipe reader
or log-file lifecycle, and existing deadlines remain unchanged. The next ARM
build and installed run must still expose the original startup failure.

[Host logging RED/GREEN evidence](evidence/native-compositor-inherited-diagnostics.json).
