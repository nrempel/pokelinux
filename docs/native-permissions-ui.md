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
  files and normal executable argument/UID refusals passed. All 29 frozen build
  input hashes match the working source. The normal session-owner release uses
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
