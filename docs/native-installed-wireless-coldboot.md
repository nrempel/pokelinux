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
or log-file lifecycle, and existing deadlines remain unchanged. A subsequent
ARM build and installed run now expose the original compositor's startup failure,
as described below.

[Host logging RED/GREEN evidence](evidence/native-compositor-inherited-diagnostics.json).

## Missing display seat in the installed VM profile

The next ARM build and image preparation pass. In the following cold boot,
inherited diagnostics reach the journal: Hyprland reaches backend creation and
aborts at about 25.3 seconds. The owner later reports `TimedOut` under its original
readiness deadline. The boot completes normal guest shutdown and actual QEMU
wait0, with qualified outer cleanup; the full functional result remains failed.

A read-only extraction from that closed disk recovers the exact named crash
report without another VM execution. Its log tail records a missing seatd socket,
no logind primary session for the compositor user, failure to open a seat, and
DRM backend failure. The Wayland fallback also has no server to connect to.
Original disks and their recorded identities remain unchanged. The corresponding
compositor log is empty; raw reports remain private.

The earlier working ARM graphical fixture explicitly launched `seatd -g alarm`
with `SEATD_VTBOUND=0` before UID/GID1000 Hyprland and observed seat0 opening.
The installed profile omitted that prerequisite. The
[optional VM seat profile](../native-session-owner/packaging/vm/README.md) now
packages an owned seatd unit and owner dependency. It reuses the existing Rust
helper to check the socket's MainPID/UID/GID before startup, orders compositor
Stop before seatd, and refuses a pre-existing socket. Automatic restart is off;
native socket removal must be observed, without a cleanup command deleting it.
The next installed run reaches active seatd and session-owner units, followed by
matching PID1 exit0 observations and native seat socket absence after Stop. The
observer then blocks the functional sequence: its strict INI parser rejects valid
repeated `Environment=` lines. Complete seat identity/socket qualification remains
false because binding never finished. Guest shutdown, actual QEMU wait0 and outer
cleanup qualify separately. The observer is subsequently repaired with actual
selected-unit host RED/GREEN; unrelated repeated directives are permitted while
the checked service contract remains strict. On the unchanged image, the later
run qualifies seat identity, peer/socket checks and normal Stop. Its first bar
click still fails to open the menu. An early diagnostic and final guest report
agree on the menu-layer timeout. Guest cleanup is empty, guest shutdown finishes,
and the actual QEMU wait returns0. Full UI acceptance remains failed.
The menu helper is staged under the upstream data directory, while the bar calls
it by command name. The stager now exports the exact original helper to
`/usr/bin/omarchy-shell`; actual file-boundary RED/GREEN passes with all prior
rows unchanged. Image preparation verifies the existing `qs` alias against the
retained QuickShell ELF. The next boot reaches a menu layer, then fails to
observe painted items in its first capture. The detector scans 2,116,000 pixels
one at a time in Python; its failure is observed 48.5 seconds after the original
menu deadline. Exact function duration was not instrumented. An optimized detector
now passes host frame parity and rejection controls. The next ARM run still fails
the original 10-second navigation allowance. A fresh, explicitly declared
30-second navigation cohort shows the painted “Nothing here yet” state: the
image omitted the original default menu definition. The expected ten-entry check
still fails. Native action and aggregate deadlines remain unchanged; this is no
pass of the old timing target. The existing native-menu staging overlay must also
be merged so AppLibrary uses the published native launch adaptation.
Guest cleanup remains empty, normal shutdown completes and actual QEMU wait0
passes. The full installed flow remains failed. The captured bar also has missing glyphs; font resources require a
separate correction. No new public screenshot is qualified.
[Current partial evidence](evidence/native-installed-menu-red.json).
The subsequent corrected image renders the ten-entry root menu and opens Apps.
Its catalog has 15 entries, but the menu folds the list into nine complete rows
and a faded tenth-row peek. The current observer rejects this legitimate
scrolling viewport; no app launch, consent or packet flow passed. Guest closure
is unqualified because a capture helper's actual wait was observed late, while
QEMU exits0 and outer cleanup passes. The next test uses the genuine menu's
keyboard navigation and retains full catalog/desktop identity checks.
[Partial installed startup evidence](evidence/native-installed-vm-seat-startup.json).
This supplies
Linux display-device access beneath the Rust owner; it qualifies no Pixel display
backend. Full installed UI/consent/connection and two-boot persistence remain open.

[Curated ARM startup and extraction evidence](evidence/native-installed-display-seat-red.json).


## Permissions process launches; window qualification remains open

Image 14 replaces only the installed capture helper with the ARM-compiled encoder.
Its separate replay matched all 24 default/candidate decodes to two retained native
captures. All other 325 installed rows, 396 packages and existing font assets
remain exact. The installed call path is unchanged. Image preparation and full VM/container closure pass.

Cold 15 was frozen but never launched: preflight found a stale outer expectation.
A one-line correction changes the expected boot 1/boot 2 action counts from 15/14 to
21/20. Actual closed host Unix-socket controls first fail the old count, then pass
both corrected counts, while refusing missing, extra, failed and mismatched
receipts. The 24-action cap and deadlines are unchanged. Authored server pixels
and bootstrap-success input keep this control separate from real GUI acceptance.

Cold 16 accepts the scrolling app viewport. Actual PageDown, Down and Return input
launches normal Permissions under UID 11045; its selected executable is observed at
80.601 s guest BOOTTIME. The normal-window predicate remains unmet at the original
30-second navigation cutoff, 81.247 s. No window/QWindow qualification, consent,
Connect, packet flow or boot 2 is reached, and the cause of the window timeout
remains unresolved. Guest custody and normal guest shutdown qualify, QEMU exits 0,
and exact outer cleanup passes with no cleanup errors. Functional acceptance
remains failed; the outer run retains exit 1. Earlier failures remain unchanged.

[Curated image, host-control and partial-launch receipts](evidence/native-installed-permissions-launch-partial.json).


## Allow reaches the policy; the observation cutoff still fails

Cold 17 opens the normal Permissions window through the genuine menu. Actual
Allow input commits Settings network control as allowed, with other grants
unchanged. The post-Allow frame visibly shows the allowed state. Its native
capture is ready at 90.638 s guest BOOTTIME, but the observer's ready result is
recorded at 97.633 s after the original 93.498 s cutoff. Functional acceptance
therefore remains failed; no deadline is extended, and Connect or boot 2 is not
reached. Guest custody, normal guest shutdown, actual QEMU exit 0 and exact outer
cleanup qualify independently.

The two raw before/after frames in [README](../README.md#see-it-running) are copied
unchanged from this generic ARM VM. They use the configured 1080 × 2424 Pixel 10
profile and are illustrations of the milestone. Fresh QWindow DPR and full
screenshot acceptance remain unqualified; they provide no phone hardware claim.

[Policy, native capture hashes and closure receipts](evidence/native-installed-network-consent-partial.json).

## Timed Allow and normal close (cold18/19)

Cold 18 passes the allowed-state observation at 83.687 seconds, before its
88.309-second cutoff. It then fails normal Permissions close. The selected Lua
config contained no Super+W binding; this did not establish an app lifecycle bug.
Image 15 preserves its entire prefix and appends the exact existing public
`window-bindings.lua` once. That config is the only changed installed row; the
other 325 records, native binaries and all deadlines remain unchanged.

Cold 19 passes genuine Super+W close: the retained Permissions pidfd is terminal,
with its process and private display listener absent before the original cutoff.
Settings then launches as a mapped native window, but advertises `pokelinux-settings` and
uses fullscreen mode 2/2. Its top edge is 0 while the bar reserves the first 26
logical pixels. The existing phone profile covers Sounds and Permissions only;
the unchanged mode 1/1 and workarea-containment predicate therefore rejects
Settings. No Connect, packets or second boot is reached. Guest cleanup is empty,
normal guest shutdown and actual QEMU exit 0 pass, and outer custody qualifies;
both functional aggregates remain failed.

The path-probe batch has separate actual host process controls, including failed
and blocked children with owned waits/EOF and closure. The pixel optimization has
exact retained-frame histogram/geometry parity and host measurements. Neither
host result alone qualifies ARM timing; cold 18 supplies the subsequent timed
Allow result. The Settings desktop-identity and phone-window correction then
passes the matched ARM build, loader and cold 20 layout checks below.

[Curated consent/close progression and receipt hashes](evidence/native-installed-consent-close-progress.json).

## Settings layout passes; Main receive retirement blocks connection

Cold 20 uses qualified image 17, changing only the Settings executable and normal
phone-window config; the other 324 installed rows equal image 15. Actual Settings
now advertises `org.omarchy.Settings`, uses mode 1/1 and fits inside the reserved
workarea. Its observed rectangle is `[21,47,390,902]`; exact workarea fill is not
claimed. The unchanged layout predicate passes before its original cutoff.

Main had already retired its worker on `receive_message / TimedOut` at
52.009320140 seconds, with an original cutoff of 51.995246469 seconds. Settings
opens later and truthfully displays Unavailable with both controls disabled. This
retirement record does not establish record consumption or which inner receive
check failed. No Connect, packet flow, functional 32-second clock or boot 2 occurs.

The initial-view observation fails on the fifth capture's original direct-wait
cutoff. Its actual exit 0, wait and EOF are retained but late, so guest strict
closure is false. Bootstrap custody and normal poweroff close, QEMU actually exits 0 after
guest-requested shutdown, and outer cleanup qualifies with unchanged inputs,
exact container absence and no cleanup errors. External peer, attach and container
all exit 1; those cleanup facts do not make the aggregate pass.

[Actual layout, retirement and closure receipt hashes](evidence/native-installed-consent-close-progress.json).
