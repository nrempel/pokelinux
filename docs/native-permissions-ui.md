# Native Permissions UI and desktop launch

The Permissions app now exposes Allow and Revoke for Sounds audio. The request and
confirmation engine is Rust; Qt is the rendering/input adapter and uses the shared
Omarchy appearance client. The fixed desktop launcher selects the installed
UID11045 scoped controller through the lifecycle actor. Opening it grants nothing.
The frozen human-preview recipe remains the earlier Sounds-only cohort.

## What passed on Harbour

- A genuine missing-dispatch RED followed by the fixed Permissions desktop route.
  Fifteen combined Sounds/Settings/Permissions actor cases passed; three Permissions
  CLI controls, six staging controls, 138 default lifecycle tests and strict Clippy
  also passed. The actor launched a native fixture, not the real GUI.
- A genuine client stub RED followed by authenticated status and explicit writes.
  Twelve independent real Unix-socket scenarios cover confirmation, wrong target,
  wrong permission, unknown fields, wrong sender PID, lost/late write replies,
  focus loss before send, stale cache/queued choice, original deadline and an explicit
  choice during a held read-only refresh. The last case failed before the refresh
  fix and passed afterward. These peers are authored root processes, not the actor.
- Actual unshown QML press/release events failed when handlers were absent, then
  passed with Allow and Revoke wired through the normal Qt adapter and Rust C ABI.
  The peer observed exactly the two intended writes, true then false. This is an
  actual input boundary test, not a visible desktop or compositor-focus result.
- Normal release Rust/Qt builds, strict client Clippy, exact UI staging of three
  files and normal executable argument/UID refusals passed. At that initial checkpoint, all 29 frozen build
  input hashes matched the then-current source. The normal session-owner release uses
  no fixture features. Owner staging separately checks twelve files.

The client preserves one original BOOTTIME deadline, authenticates both connection
and packet credentials, and retains owner/revision solely in Rust. A fresh explicit
choice may cancel a read-only refresh; a pending Set is never replaced or retried.
Loss of authority or an uncertain reply cannot be displayed as a confirmed change.
A later status read reports the current durable fact without confirming the old
request. Closing the app closes the socket without joining a worker.

## Failed attempts retained

Initial client runs lacked credential-drop capabilities and failed setup; an early
classifier wrongly labeled one as RED. Its correction is retained. The clean owned
stub RED is the behavioral result. One eleven-case run passed all cases but its
obsolete outer marker classifier failed; the corrected run passed. Initial Qt RED
failed click-test setup and is not the behavioral RED; the corrected real-click
consumer supplied that result. An initial archive output-directory permission
failure was corrected before the successful build. The first aggregate launcher
run lacked GNU Make: six install tests and Clippy ran afterward against the same
immutable source. None of these setup/reporting failures establishes behavior.

## Remaining acceptance

The complete normal Omarchy flow is still open: denied/quiet → Allow → explicit
Play → measured tone → Revoke → measured silence → reallow remains quiet after
returning focus to Sounds → explicit Play → close/reopen and normal shutdown.
The separate passing boundaries above do not substitute for that joined test.
No new screenshot, native QWindow density, ARM execution or phone behavior is
claimed. No permissions/audio completion-matrix row is closed. Physical phones
remain untouched. [Machine-readable evidence](evidence/native-permissions-ui.json).

## First joined Omarchy attempt

The first genuine Omarchy VM attempt reaches the correct Permissions menu row
through three actual external input actions. A direct child of the actor is briefly
observed with UID11045, then exits before its executable/arguments can be captured.
No normal Permissions window appears within the original deadline. No permission
change or PCM acceptance runs. The menu geometry and selected desktop entry match;
the precise early-exit cause remains unresolved because app stderr is discarded.
This is not yet a qualified production behavioral RED. Next is a bounded loader/
startup diagnosis against the guest's Qt6.12 runtime; the host component build uses
Qt6.8.2, which is a difference to investigate, not proof of incompatibility.

Guest process cleanup and exact container absence pass. The external viewer fails
when its server ends during failure cleanup; that is not normal viewer acceptance.
All inputs and failed results remain preserved. No new screenshot is published.

## Loader defect isolated

A separate disposable guest executes the exact normal Permissions ELF against the
unchanged provisioned libraries. No arguments should return64; six absolute
arguments as root should return65. Both instead return127 before `main`, with the
same306-byte loader error: a copy relocation for `QSocketNotifier::staticMetaObject`
conflicts with the library's protected-symbol/indirect-access requirement.
This proves an executable build defect independently of Qt object creation,
Wayland, app registration or consent policy. It does not prove there are no later
startup defects. Both direct waits/output EOFs, normal guest shutdown, QEMU0 and
exact container absence pass, with all selected inputs unchanged.

The repair must compile Qt-facing code as PIC and verify the resulting ELF, then
rerun the unchanged64/65 consumer. PIE alone is insufficient for this class of
protected external symbol, as described in the
[GNU C Library documentation](https://www.sourceware.org/glibc/manual/latest/html_node/Dynamic-Linker-Hardening.html).
The repair now passes. All four Qt-facing C++ targets explicitly compile with
`-fPIC`. A compiled-ELF regression first fails on the original executable's actual
COPY relocations; both rebuilt executables pass that same contract. The normal
release build, real unshown QML Allow/Revoke clicks, root argument/UID controls and
exact three-file installation staging pass. The Rust archives and UI behavior are
unchanged; all30 selected build-source hashes match the repaired source.

The fixed normal ELF (`3b73a5e51404031a97d1740ce181d2c69c157fd06a0d0491c0b1ec1233078012`)
then passes the same two guest probes with exits64/65. All six execution sources,
probe deadlines and unchanged guest libraries match RED. Actual waits/EOFs,
ordinary guest shutdown, QEMU0, exact container absence and input preservation
pass. This closes the demonstrated loader defect; the full normal Omarchy
permission/audio retry remains open.

## Normal window observed after the repair

The second genuine Omarchy attempt now observes the fixed normal Permissions
executable at UID11045 with all six arguments, its owned private Wayland socket,
a mapped native window, and actual compositor focus with healthy authenticated
Source state. This advances beyond the first attempt's early process exit.

The observer stops on the first frame about0.24 seconds after the executable
witness: the caption shows Permission unavailable and both labels are dimmed.
Button geometry is correct, but the visible antialiased label pixels do not equal
the expected muted RGB exactly. The observer aborts instead of collecting another
frame within its existing bound. One early frame cannot distinguish transient
readiness from persistent unavailability. No Set or PCM acceptance runs. Full
guest/QEMU/container cleanup and input preservation pass.

Review also finds a downstream observer mistake: permission grants do not append
to the durable receipt list. The next consumer must observe the requested grant
transition and changed persisted content, not require a receipt-count increment.
Neither correction changes production authorization. Validate the visual classifier
against independent real QML enabled/disabled states before retrying; preserve
separate visible-state, durable-state and PCM evidence, with no wire-ACK claim.
The frame remains private and has no new QWindow density or screenshot qualification.


## Actual Allow and menu access over Permissions

Permissions now requests a maximized window instead of fullscreen so the Omarchy
bar remains accessible. The retained fullscreen run hid the bar icon; this is a
retained negative observation, not a fresh VM RED. The new normal build passes
both compiled-ELF relocation checks, real unshown QML Allow/Revoke clicks, root
argument/UID refusals and exact three-file staging. An initial build attempt
failed because its bind-source directory was missing; it is not a behavioral RED.

The third normal x86 VM run launches Permissions through the genuine menu, then
receives one actual remote Allow click. Independent read-only policy observation
sees the previously absent Sounds/media_route grant persisted as allowed, with
zero receipts as expected. A later native frame shows Allow disabled and Revoke
enabled. This is visible-state and durable-state evidence, not an independently
observed wire acknowledgement.

Clicking the visible bar opens the menu over Permissions. The run nevertheless
fails after eight menu-recognition attempts, before selecting Apps or Sounds.
The retained final frame visibly contains the menu; this is not evidence that
the bar click failed. Playback, Revoke, reallow silence and ordinary app closes
remain unexecuted in this cohort. Normal guest shutdown, QEMU wait, exact container
absence and unchanged inputs pass. Fresh QWindow density metadata and publishable
screenshots remain open; all diagnostic frames stay private.


## Sounds launches; desktop tiling clips playback controls

The fourth normal x86 VM run keeps the same production binaries and deadlines.
Only menu recognition and failure-observation retention change: a contiguous
bordered menu component replaces whole-row background extrema. The original
observer fails on the retained menu-over-Permissions frame; the corrected observer
passes it and the original root/Apps frames. Six authored negative controls reject
blank, borderless, ambiguous, wrong-width, off-center and empty-label images.
Strict controls require the intended rejection stage; this is observer evidence,
not a new app or hardware qualification.

The new VM repeats actual Allow and durable/visible confirmation, then completes
bar→Apps→Sounds with normal executable/window identity and authenticated focus.
Its eight-frame Sounds readiness bound fails. The final native frame shows both
apps tiled vertically, leaving Sounds Play/Stop below the visible window; the
phone needs a single-app presentation layout with the bar still accessible.
No playback, Revoke or reallow-silence acceptance occurs. The original visibility
check remains intact. Full guest/QEMU/container cleanup and unchanged inputs pass.
The frames remain private without fresh QWindow density qualification.


## Phone layout and guest consent/audio sequence pass

The scoped [phone window profile](../native-session-owner/packaging/presentation/phone-windows.lua)
selects maximized internal/client mode1 for Sounds and Permissions, transfers that
mode on focus changes, and retains it on close. It preserves the shell reservation
and configured gaps/decorations. The prior actual split-window frame is retained
integration RED, not a new synthetic test. The fifth installed x86 VM appends the
exact fragment to its selected normal compositor configuration; production app
and Rust service binaries, PCM thresholds and original deadlines are unchanged.

All observed focused windows report internal/client mode1, at[21,47] with logical
size369×855. Genuine bar/menu navigation, actual Source focus and Alt-Tab work.
The guest completes one Allow, explicit Speaker/50%/Play, Revoke, reallow without
Play, and both normal closes. Independent paired48kHz PCM windows contain24,000
samples per output: the speaker tone has RMS0.0883867 and440Hz amplitude0.1249475;
the other output is zero. After Revoke, both outputs are exactly zero, the old
producer pidfd is terminal and its process/node/client/links are absent before
the original pre-natural-retirement bound. Reallow/refocus remains exactly zero
without a second Play. This is three bounded audio windows, not continuous audio
coverage or physical speaker validation.

Guest and boot results pass, including normal app retirement and QEMU0. The outer
result remains failed: the external client sends all16 requested actions but
reports `unexpected server message` while reading its final frame. Exact container
absence, host cleanup and unchanged inputs pass. No exact unexpected message type
or cause is inferred from that assertion alone. The viewer boundary must be
resolved before claiming the complete run passes. Fresh QWindow density metadata,
new published screenshots, ARM, device audio and all physical-phone gates remain
open. No phone was accessed.


The sixth integrated run repeats the guest consent/audio/close sequence successfully,
then the metadata-only viewer diagnostic identifies actual server message type3.
The outer run remains failed. Type3 is the standard ServerCutText message; the
subsequent reader change discards bounded text and retains the final-frame/EOF
requirements. Socket RED→GREEN checks pass; a fresh integrated retry is required.
The fifth run's unrecorded byte is not retrospectively relabeled as observed.

A separate real-server RED→GREEN pair now qualifies that reader boundary: an
isolated authored X11 cut buffer produces actual ServerCutText; the old reader
rejects type3, while the repaired reader discards29 bytes and receives the full
frame and EOF. Exact owned cleanup passes. This does not recover the earlier VM
payload or identify its originating application; the integrated retry remains open.

## Seventh joined attempt: no observed playback

The next x86 VM selects the bounded ServerCutText repair with unchanged app and
service executables, phone profile, input sequence and audio bounds. Normal
Permissions/Sounds launch, Allow and the actual Play touch run. The tone observer
stops after its original 64 graph observations with `SETUP_FAILURE`: no
`source.media` producer appears in those snapshots. Both recorded output files
contain 108,544 silent F32 frames each. This is a partial observation, not a
completed paired tone/silence acceptance window. A short producer between
snapshots or a later start is not excluded.

No app wire request, acknowledgment or refusal reason was recorded. Periodic
read-only Status currently disables Play in the Rust client; a separate held-Status
client test is the next investigation, not an established explanation for this
run. Do not extend the observation count or original deadlines to call it passing.
Revoke, reallow and normal app closes are not completed. Guest/boot closure,
actual container CLI wait, exact container absence and unchanged inputs pass.
The external viewer stops after ten actions when the server ends during guest
failure; this run never tests its repaired final-frame boundary. No screenshot,
ARM or physical-phone qualification follows.
