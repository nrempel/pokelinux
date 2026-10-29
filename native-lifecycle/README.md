# Native lifecycle authority

The installed UID11045 Permissions principal can read/set only Sounds' MediaRoute
policy through credentialed `app.sock` requests. Current Source focus, owner/policy
revision and the original BOOTTIME deadline gate writes and positive replies.
The existing owned writer handles persistence; only an exact unsent admission can
be canceled. A submitted write can commit after its UI loses authority, so failed
publication requires a fresh status read, never an automatic Set retry. See the
[controller contract and evidence](../docs/native-permission-controller.md).
The fixed UID1000 desktop route `launch-permissions.sock` selects only the installed
UID11045 controller. The [native UI](../native-permissions-ui/README.md) and launch
boundary pass separate host tests; the full Omarchy consent/audio flow remains open.

A separate fixed `launch-sounds.sock` accepts only desktop UID1000 and
`launch_sounds`, without requiring a Messages provider. It reuses existing app
reservation/ownership and exposes no grants or arbitrary app selection. Six
actual host cases pass after a compiled missing-dispatch RED; the matching Rust
`sounds-launch` client and ARM builds compile. Installed session/Source/audio
provisioning and normal desktop invocation remain open. See
[fixed Sounds launch](../docs/native-sounds-desktop.md).

The actor also mediates [current MediaRoute selections](../docs/native-audio-lifecycle.md)
through one `audio::Provider`, supplied explicitly to `run_with_services`.
Foreground admission and an approved background Media lease have separate
lifetimes. Revocation withdraws the actual owner connection, without claiming
remote cleanup. The matching Sounds client uses the actor's credentialed
`app.sock`; headless callers passing `None` expose no audio service.

The actor owns app children, durable policy work, Messages reads and session
facts. Display-enabled callers supply `display::Factory` through
`service::run_with_services`; headless callers use the same app slot algorithm.
A trusted Factory creates one owned Context for a reserved app instance. Setup
uses an original one-second BOOTTIME deadline within the two-second client RPC.
No app starts before Ready and an exact-owning-connection EOF check.

Retirement revokes data authority, withdraws the listener before SDK signals and
retains app/context ownership until actual waits. Snapshot and Stop stay bounded
while initialization is held. Source observation and the history writer remain
independent of app context retirement. Metadata is not permission, pathname
withdrawal does not disconnect accepted clients, and local reap is not vendor
cleanup.

Owned locker facts are checked at the typed session Provider boundary. Repeated
sequence/revision values cannot hide changed locker ownership, and unlocked
snapshots cannot carry a locker. Inconsistency fails the established Source
terminally. The same cached reference or independently opened handle for the same
live task remains valid without a policy fence. Three actual RED failures precede
four focused GREEN controls and12 component passes on host and actual Arch ARM
kernels. This is separate from Source IPC, authentication and trusted unlock;
see [scope and provenance](../docs/native-session-owned-locker.md).

The actor also validates exact-operation fact shapes and immutable repeated
sequence/revision values. Three failing protocol groups precede four focused
passes and16 full lifecycle passes on each host and actual Arch ARM kernel;
see [protocol actor evidence](../docs/evidence/native-session-protocol-actor-provenance.json).
Generation/event facts remain observations, with no authentication or restoration
capability added.

The private actor-attempt admission/lifetime primitive has no production authentication ingress.
Its opaque non-Clone handle grants no authentication or restoration authority.
One occupied slot retains its exact actor/attempt identity, Source incarnation,
accepted nonzero lock generation, live locker pidfd, checked lock-admission epoch
and original absolute BOOTTIME deadline of at most30s. Every accepted new lock
advances that epoch even when pending intent is already durable. Observed context
excursions, Source loss/expiry, locker death, explicit lock/suspend and degradation
retire the slot permanently. Matching live kernel tasks requires ProcessView even
for shared references. Retirement alone never frees occupancy. A private
capacity-only consumer now removes only the exactly matched actor/attempt/original-
deadline slot; no production closure producer is present. Stable heartbeats and
unrelated lifecycle revisions do not renew time.

The frozen API-prerequisite RED exits101 with21 missing-method `E0599` errors and
executes no attempt cases. After implementation,33 host lifecycle components pass,
including17 new controls, with formatting and strict normal/all-target Clippy.
The preceding one-test Provider control remains separate. The initial127 runner
setup failure and32/33 aggregate missing the existing storage helper remain failed;
fresh complete runs supply the exact same-build daemon helper. These are typed
Provider/real-pidfd component results, without Source IPC, PAM or broker dispatch.
That earlier selection enables no authentication result, restoration or durable
clearing.
No installed authentication or ARM/device result is claimed by this selection.

A later [capacity-consumer selection](../docs/native-auth-actor-capacity.md) adds
seven controls after a missing-private-API compile prerequisite. All40 lifecycle
tests pass with the existing root-owned storage fixtures and same-build daemon;
22 actor tests separately pass as UID1000 with18 explicitly filtered. Actual
Authority/Provider/pidfd behavior is exercised, while closed-capacity stamps are
cfg-test authored data with no production constructor. Invalid/replayed stamps
cannot remove a successor; original expiry and Source loss permit only capacity
reclamation, never handle/Source revival or policy clearing. Fresh explicit begin
still performs its own admission. Strict Clippy and separate AArch64 test-ELF
compilation pass; no Source IPC, broker, PAM/scope or unlock is enabled.
The unchanged22 actor cases separately pass on an
[actual generic ARM kernel](../docs/native-auth-actor-capacity-arm.md), with18
explicitly filtered. Full40 inventory is listing only; no ARM root-storage,
production stamp producer or authentication composition is qualified.

[Per-app ARM evidence](../docs/native-display-app-generations.md) qualifies
lifetime/relaunch and held-init Stop. Pending EOF passes its narrow boundary;
its successor aggregate is failed after Source InvalidData of unknown predicate.
The [native lock entry](../docs/native-lock-admission.md) now accepts only a fixed
UID1000 `{"op":"lock"}` request on the actor's `lock.sock`, using kernel credentials
on every packet. Memory authority is revoked before owned storage work; successful
ACK requires synced required-schema `Durable.lock_pending`. A repeated cached
pending lock also requires no unresolved actor/storage operation at planning time. The actual
[storage-barrier regression](../docs/native-lock-storage-barrier.md) now passes
both held-child cases after a behavioral RED; all 40 host regressions pass.
Refusal still fences memory immediately. A later
[queued-reply guard](../docs/native-lock-queued-storage.md) also rechecks success
at every send and permanently downgrades it when storage is busy. A genuine
queued-socket RED precedes its host GREEN, two planning controls and all 40
regressions. The older planning source separately passes two actual generic ARM
kernel cases. A later [current-code ARM run](../docs/native-lock-storage-arm.md)
passes all 40 library tests and both planning cases, but its aggregate fails the
queued fixture's main-poll setup before the test packets. Queued ARM behavior
remains unqualified; no guard is relaxed or stop retried. Those earlier runs qualify no authenticated-clear ordering. That intent is restored
before Ready and cannot be cleared by unlocked Source facts or root admin false.
`Gate::Lock` remains monotonic through irrelevant focus revisions. The fixed Rust
client verifies the connected root actor's reply within the original two-second
BOOTTIME budget before executing the exact retained original Omarchy action.

Initial 8 host/6 original ARM/8 component ingress groups and later 6 host/5 original
ARM/8 component persistence groups are separate frozen selections. The later actual
healthy actor Stop/wait and changed display/Source epoch preserve denial in the
same policy directory. SIGKILL establishes unclean-history refusal, not positive
crash recovery. A fresh unlocked session needs an explicit root-owned fresh storage
lifecycle. The full installed smartphone, trusted unlock/restore, PAM/secure frames,
camera plus new-lock runtime, incoming restricted-display composition, suspend/wake
and Pixel hardware gates remain open.

The newer [downstream restoration](../docs/native-session-restoration.md) retains
an exact completion's context atomically through Armed and one owned Clearing
write. A later exact protocol event is necessary; arming, locker death or forced
unlocked facts alone cannot clear. New lock during clear retains denial and
reserves a bounded true replacement through the same worker. Stop cannot seal
before that required write. Queued lock success now retains the worker admission
sequence, rejecting an intervening write even after it finishes. Both clear and
lock completion recheck actual BOOTTIME against their write deadlines.

After behavioral ordering/clock/service/socket REDs,53 default host tests pass,
with2 root integration tests explicitly ignored there and separately selected
and passing. They exercise the real actor, sole worker, actual UID11010 app reads,
held synced clear, actual UID1000 new lock and clean same-policy restart denial.
Authentication, typed Source and history are authored test inputs. AArch64 builds
are compile-only; no new ARM execution is implied. No production completion
constructor/ingress exists. Genuine authentication, real Messages history/normal
Qt composition, negative service matrix, secure presentation and phone gates
remain open.

The preceding coverage-only selection passed all53 default tests with6 explicitly
ignored cases separately selected and passing. It includes the original positives
and actual-loop forced/expired Source, late completion after a new idle lock, and
held-clear Stop with failed fresh-storage-worker reopen. A capacity-bounded
Source-observation barrier has an absolute2s deadline and no auth authority.
Source100ms and all production deadlines stay unchanged. Host daemon/launcher and
AArch64 daemon bytes match the preceding selection. See the
[additional provenance](../docs/evidence/native-session-restoration-negative-provenance.json).
The full actor's unclean reopen, remaining negative combinations, real history/Qt
composition, genuine authentication and physical gates remain separate.

The newest [retained-history tracer](../docs/native-session-restoration-history.md)
adds a dev-only facade around the real Projector writer/readonly child, preserving
its full Provider contract and original deadlines. Two actual cases now obtain
native Messages pages through UID11010 and retain identical message receipt IDs
through actual relock, clean actor reopen and fresh restoration without reseeding.
53 default tests pass with8 explicitly ignored integrations separately passing;
strict checks and AArch64 compilation are separate from runtime. The earlier6
stress cases still use authored history. Auth/Source/SMS seed remain authored;
normal Qt/original compositor, genuine authentication and hardware remain open.

The subsequent [actual Qt consumer](../docs/native-session-restoration-qt-client.md)
adds a trusted private QCore app profile linked to unchanged production client
sources. That checkpoint passes53 default tests and9 selected service cases. The actual
client decodes retained bodies, empties models after relock and returns the same
IDs after fresh actor restore. It changes no production source/ingress. That
selection killed the app on Stop, leaving Qt graceful destruction unqualified;
normal GUI/original
compositor and genuine authentication remain separate.

The latest [normal app Stop](../docs/native-app-stop.md) adds one absolute500ms
BOOTTIME TERM grace only for Running apps, capped within the original actor2s
budget; failure/cancel/Preparing remains forced. First-Stop reply/storage shutdown
is memoized, repeated Stop cannot renew grace, and actual Child/context ownership
persists through wait/drain.53 default tests and13 separately selected cases pass:
actual Qt owner wait0, TERM-ignore responsive fenced grace then wait9, forced
Terminate and an authored held Terminal Context with an actual Qt child. Strict
checks and ARM compilation are separate. No GUI/original SDK/phone proof follows.

The subsequent [normal full Qt GUI tracer](../docs/native-session-restoration-gui.md)
composes the actual display Listener/Supervisor and retained Messages in original
Hyprland/Omarchy on Arch ARM. A dev-only primitive facade bridges trait identity;
actual successful actor sends are observed through a bounded cfgtest witness,
never a second reader/model proxy. The same Rust test ELF reaches three real GUI
phases before a genuine Qt wait15 Stop RED and repaired normal main wait0 GREEN.
53 default host tests and13 selected host cases still pass; one ARM GUI case is
separately selected. Authentication/Source/SMS seed remain authored; direct Qt API,
secure presentation, production provisioning and hardware remain open.

## Connectivity control

The [registered app tracer](../docs/native-connectivity-actor.md) now joins actual
UID11050 launch, durable NetworkControl grants and the normal private Rust Main
to real packets. A genuine refusing-actor RED precedes the effect implementation.
A successful connection survives actual app exit; a fresh app receives newer
readbacks and explicitly Disconnects with independent removal and silence.

`run_with_connectivity_services` takes an explicitly trusted provider. Other
entrypoints select none. App requests contain only desired state and the original
absolute BOOTTIME deadline. A current app/policy/lifecycle/session/backend stamp
fences submission and publication; withdrawal closes the requester connection
without Disconnect, retry or Stop. Separate private wire IDs and transport/worker
incarnations prevent old intent reuse. Own Inspect occupancy permits reservation;
remote pending effects block mutation. Readonly status refreshes preserve actual
250ms timestamps and leases.

Six normal binaries and114 staged default tests pass, including eight new
controlled Authority/Provider plus real pidfd/socket groups, with formatting and
strict all-target Clippy. The27 ignored include the copied integration case that
runs separately. Registry identities remain unchanged. The normal Settings
engine/window, installed selection, genuine session/authentication, ARM and
hardware remain open; earlier private-Main evidence retains its own cohort.
