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
