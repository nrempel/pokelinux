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
