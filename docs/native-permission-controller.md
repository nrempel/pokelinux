# Native permission controller

The next user-facing slice is a separate trusted controller for Sounds Allow/Revoke,
followed by explicit Play, measured tone, revocation and measured silence in the
normal Omarchy session. Consent and authorization remain in the Rust session owner;
QML will present the result. Sounds remains an ordinary app.

## Installed role boundary

`AppProfile` now requires an explicit installation-owned `role`. There is no
missing-role fallback. Ordinary profiles use:

```json
"role": { "kind": "application" }
```

The first controller configuration is narrowly scoped:

```json
"role": {
  "kind": "permission_controller",
  "target": "org.omarchy.Sounds",
  "permission": "media_route"
}
```

The protected profile loader requires an installed ordinary Sounds target and
rejects self-targeting, missing targets and other permissions. Existing root-owned
profile/executable checks remain in force. The role is configuration metadata;
it grants no new app request or access to the root admin socket by itself.

The same five protected-file consumers first execute with two expected failures:
the old loader rejects the new role and accepts an omitted role. After the change,
all five pass. This is host Rust configuration evidence, not app IPC, user consent,
UI, ARM or hardware qualification. An earlier container traversal/setup failure
executes no Cargo tests and remains separate.

Broader host checks pass 122 lifecycle cases (26 unrelated isolated cases remain
ignored), 33 owner configuration cases, formatting and strict Clippy. The 19
display cases pass separately after supplying the root fixture observer with
`DAC_OVERRIDE` for its UID1000 private directory. The earlier five-capability
display run fails on that traversal and remains failed; it is not a production
RED or repair. [Evidence](evidence/native-permission-profile.json) records the
separate results.

Current ordinary Rust fixtures and the packaged example are migrated explicitly.
Frozen historical runtime cohorts retain their old binary/profile pairs; their
results are not transferred to this changed schema.

## Installed owner and display selection

The session owner now accepts an explicitly installed `org.omarchy.Permissions`
profile at UID11045, with exactly the Sounds/MediaRoute controller role. The
protected profile selects this optional app; ordinary Sounds, Messages and Settings
retain application roles. The same canonical endpoints, six arguments, trusted
executable and private per-app directories remain required.

The positive installation test first fails against the old owner, while identity,
scope and endpoint refusals pass. After the change both pass. A separate actual
Linux process/socket test first refuses the controller and then passes: its private
Wayland listener belongs to the fixed app group, stopping withdraws that listener,
and the owned SDK child is actually reaped. This tests a preparing display context,
not a rendered controller or compositor consent. An earlier run fails during fixture
cleanup because the root observer cannot traverse a UID1000 private directory;
that environment failure is preserved separately. The corrected observer environment
uses DAC_OVERRIDE, with no production privilege change.

All 17 owner configuration and 20 display tests pass, as do formatting and strict
owner Clippy. [Owner selection evidence](evidence/native-permission-owner-selection.json)
records the separate scopes. A desktop launch endpoint, native controller executable
and visible consent remain to be joined to this selection.

## Scoped permission requests

The app socket accepts `permission_status` with `deadline_ns`, and
`permission_set` with `allowed`, `expected_owner`, `expected_revision` and
`deadline_ns`. The target and permission come from the installed controller role;
requests cannot supply them. The owner authenticates the registered live
PID/UID/generation and requires current Source focus, an unlocked/awake session
and an original BOOTTIME deadline no more than one second ahead. `permission_state`
reports the durable `allowed` fact, fixed target/permission, owner incarnation and
policy revision. It does not authorize playback by itself.

Set uses the existing sole durable writer. Busy refuses without changing the active
operation. Revoke fences existing media before persistence. Immediately before a
queued write can be sent, current authority is rechecked; only the exact provably
unsent request can be withdrawn. After submission, the writer stays owned through
its original deadline even if the controller disconnects. No uncertain write is
retried automatically. Positive replies are rechecked before every nonblocking send,
including retries after socket backpressure.

A successful durable write remains a fact after focus loss or a missing reply.
`Denied` means the current request/reply could not be authorized; it is not proof
that no disk change occurred. Regaining focus requires a fresh status read, not an
automatic repeated Set. Conversely, canceling an unsent Revoke leaves the previous
durable Allowed state intact: the old media lease is fenced, but a new explicit Play
can still be authorized. The UI must not display Revoked merely because the audio
provider disconnected.

The desktop launch route, Rust client and visible Allow/Revoke surface remain
unwired. The full acceptance sequence remains open: denied/quiet → Allow → explicit
Play → tone → Revoke → silence → reallow stays quiet until a new Play → close/reopen
and normal shutdown. No permission, audio or phone completion-matrix row is closed.

## Host evidence and limits

A clean baseline runs the same five credentialed JSON consumers: three refusal
controls pass, and the two missing status/set operations fail with Denied instead
of PermissionState. The final 16 targeted cases pass. They cover actual Linux
credentials, ordinary-app refusal, stale owner/revision, deadlines, focus/lock/suspend,
real disk persistence, exact unsent cancellation, inert busy refusal, disconnected
callers, generation changes and actual EAGAIN reply backpressure. Fresh Source
heartbeats do not extend the original request deadline or invalidate a semantic
stamp merely because the previous sample expired.

The held-worker cases pause the existing isolated worker after synced replacement
and before its reply. They preserve the commit through focus loss or disconnect;
a separate Stop case refuses an unclean reopen. This is not a blocked-filesystem
syscall or complete actor-loop test. Source and media facts are authored; no real
compositor consent, audio backend, ARM or physical phone is qualified here.

Broader checks pass 138 lifecycle tests (26 unrelated isolated cases remain ignored),
35 owner configuration tests, formatting and strict Clippy. The 20 display cases
pass separately with the observer capability described above. All containers are
reaped/removed and selected sources remain unchanged. The [ingress evidence](evidence/native-permission-ingress.json)
records these results and the failed fixture attempts separately. One intermediate
RED classifier incorrectly accepted an extra absent-file fixture failure; its
original result is preserved with a correction, and the replacement baseline
requires exactly the two named behavioral failures.

## Next visible join

Use a separate Permissions surface with a Rust request/state client and the existing
Omarchy appearance client. Qt presents state and forwards explicit choices. A failed
or lost Set reply must clear confirmation; regain focus and read current state, never
repeat the Set automatically. A completed disk write may outlive its acknowledgement.

Install a fixed desktop launcher and use a normal Alt+Tab window-switch binding so
Permissions can regain actual Source focus while Sounds continues playing. Existing
per-app custody already permits both apps; relaunching an existing instance does not
refocus it. Recompute the genuine menu geometry after adding the desktop entry.

The fresh VM cohort must record both app identities, Pixel10 output/window metadata
and actual focus at each action. Reuse the PCM observer's tone, silence and producer
retirement criteria; replace its historical root-admin revoke trigger with the real
controller touch and committed state. Preserve all old runtime cohorts and their
binary/profile pairs. These are planned joins, not executed evidence.
