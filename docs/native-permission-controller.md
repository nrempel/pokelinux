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

## Remaining implementation

The installed owner still needs controller launch selection and narrowly scoped
request handling. Authenticate requests from the actor-owned PID/UID/generation;
never trust an app-supplied controller label. Writes require current Source focus,
unlocked/awake state, an original absolute deadline and a current policy revision.
A busy writer must refuse without disrupting the existing operation. Revoke must
fence media before persistence, and an uncertain write must not be retried implicitly.
A completed durable change remains a fact even if its UI acknowledgement is refused
because focus or the caller changed.

Then add the Rust client, installed native controller surface and visible Allow/Revoke.
The full acceptance sequence remains open: denied/quiet → Allow → explicit Play →
tone → Revoke → silence → reallow stays quiet until a new Play → close/reopen and
normal shutdown. No permission, audio or phone completion-matrix row is closed here.
