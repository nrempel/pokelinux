# Lifecycle-owned media routing

The lifecycle actor now mediates logical Media selections through the existing
Rust audio owner. Sounds connects to the actor rather than receiving direct
native-audio mutation authority. The audio service remains the sole owner of
PipeWire links; the actor owns current app permission and its Media connection.
This is a shared service boundary, without Pixel node names in app requests.

## Current admission and background continuation

`media_route` accepts a logical route and a1–1000ms lifetime. Admission requires
the exact registered live PID/UID/app generation, an independent `media_route`
grant, foreground/unlocked/awake state and the selected backend incarnation.
When a session Provider is installed, the existing current Source/deadline checks
also apply. The original absolute CLOCK_BOOTTIME deadline survives queueing,
provider backpressure and positive-reply publication. Generic historical
`prepare` permits cannot authorize this effect.

Successful publication promotes the current selection to an approved Media
lease. Ordinary focus, lock and suspend transitions may retain that approved
lease for background playback. Matching grant revocation, app-generation/death,
connection retirement, audio-provider connection loss and actor Stop withdraw it. An unrelated
grant change does not revoke an approved lease. A matching revocation is fenced
in memory before its owned durable policy write; regrant cannot revive an old
lease. No durable receipt is treated as media-effect authority.

One bounded unsent replacement may be retried only after current-stamp checks.
Cancelling it preserves an earlier approved lease. Once bytes have crossed the
audio boundary, cancellation closes the connection: possible effects remain
uncertain until independently observed. Provider `poll()` never dispatches an
effect or reconnects. Read-only inspection shares the single-in-flight client.
Explicit reconnect creates a fresh connection/incarnation and never replays an
old route.

The exact app socket is checked for EOF or extra input before dispatch and every
positive-publication attempt, including after EAGAIN. A request-half close must
retire the provider even though the kernel still permits sends to the client's
read half. This is an observed-input fence, without an atomicity claim against a
close occurring after the check. Stop and snapshots perform no blocking audio
transaction.

## Matching Sounds client

The separate [fixed desktop launch](native-sounds-desktop.md) now reaches the
existing actor-owned app path without depending on a Messages provider. Its host
proof uses an actual UID11042 sleep child, with a later actual public native-client
invocation. The [normal permission-aware ARM UI](native-audio-arm-lifecycle.md)
separately passes all four touch/graph/PCM groups with native Pixel10 captures.
Its root-authored lifecycle controls do not qualify installed Source/audio.

The single target Sounds client uses nonblocking credentialed SEQPACKET to the
root actor's `app.sock`. Its fifth argument now selects that actor socket; the
appearance argument and QML remain unchanged. Every packet must match the
connected actor's kernel PID/UID/GID, with no unexpected descriptors or truncated
control/data. UID0 authenticates the trusted root deployment, not a unique actor.

`media_status` exposes logical routes and admitted/applied observations, plus
current choose/reconnect availability. At most32 routes are paged eight at a
time under one original one-second deadline. Status is read-only. The UI keeps
controls enabled during a read-only refresh so polling cannot cancel a normal
press. One explicit unsent selection may wait for the completed fresh catalog;
permission, availability, route membership and its original deadline are checked
before dispatch. Further taps are ignored while queued. Loss, expiry or refused
fresh state cancels without replay; prior applied state remains until actual
submission. A queued wire TTL uses only remaining original milliseconds.

## Evidence and limits

Seven current-authority component controls use real pidfds and synced policy
files with injected app/lifecycle facts. Genuine missing-admission RED and a
separate historical-permit failure precede the repair. Four additional real
Linux SEQPACKET controls reproduce three publication failures before the
source-only pending-phase fix, then pass alongside the full64-pass lifecycle
library suite;14 environment-specific integration tests remain ignored there.
Authority/provider state in these socket controls is modeled and explicitly
separate from the actual host composition.

The actual host fixture launches its app bridge through the lifecycle actor as
UID11042, with real PID/UID/SCM checks. Native audio runs as11040, privileged call
fixture11041 and its configured app/root principal0. Direct11042 mutation is
denied. Four groups qualify no-grant quietness; current granted selection with
independent graph/440Hz samples; actual filled-socket EAGAIN followed by revoke,
public reconnect/regrant and no replay; and media revocation during an880Hz
call followed by call release without restored media. The final half-close
source passes all four in a fresh build/run. A root-owned fixture alias and held
inspection admission expose real backpressure; no send outcomes or owner events
are fabricated. Lock/focus/grant ingress is authored, without production
Source/authentication/PAM qualification. Immediate status after revocation may
still show the previous observation; actual graph and paired silence establish
the tested withdrawal.

The first integration controller used invalid5000ms typed requests. Both failed
runs remain preserved and their missing-dispatch classification is superseded.
The corrected1000ms controller is identical across baseline and final runs.
Its baseline replay occurred after initial implementation, so it is not the
chronological test-first proof. The component and publication-socket REDs retain
their own original chronology.

The initial lifecycle Qt cohort passes25 behavioral socket cases plus setup and
cleanup after missing-protocol and held-response REDs. A fixture-only negative-
receive correction preserves functional assertions and its earlier failed run.
Five further reconnect controls bring the current total to30 behaviors plus
setup/cleanup,32 passes. Read-only status cannot cancel a reconnect press: one
explicit action waits for fresh reconnect permission and retains its original
deadline, with no replay after expiry, refusal or disconnection.

The first reconnect fixture mistook initial disconnected `canReconnect` for an
already consumed actor status. Corrected controls wait for the first complete
trusted catalog, then hold a later poll. The corrected baseline reproduces five
failures with27 passes, followed by32 repaired passes. This baseline replay came
after the initial implementation; no chronological test-first claim is made for
those corrected five cases. Earlier invalid-premise/setup runs remain preserved,
and the prior25 controls retain their separate test-first chronology. These Qt
tests use an authored server, not the actual actor/audio graph. The completed
[normal ARM tracer](native-audio-arm-ui.md) uses a historical direct-stream cohort;
it must not be presented as this lifecycle Qt composition.

Strict Rust formatting and all-target Clippy pass. The final source also passes
64 host library tests with14 environment-specific tests ignored, host binary
builds, ARM64 binary/test cross-compilation and three existing composed-caller
builds. Fifteen lock graphs add only the audio/power path dependencies, retaining
all existing package versions and unrelated edges. Final publication differs from
the actual host runtime selection only by formatting and one scoped Clippy
allowance. The first aggregate caller build lacked the required public camera
schemas; that setup failure remains retained. A schema-only follow-up compiles
both remaining callers without changing publication source or repeating passed
lifecycle tests.

[Hash-only provenance](evidence/native-audio-lifecycle-provenance.json) records
the exact selections and separate scopes. Production embedding must explicitly
supply `audio::Owner` through `run_with_services`; the existing headless daemon
and callers with `None` provide no audio service. Installed service/desktop
provisioning and normal Sounds→current Source/permissions→audio composition
remain open. The later normal ARM join above qualifies root-authored authority
only; it does not install a production Source. Original CLOCK_BOOTTIME tests do
not qualify real suspend behavior.
Physical routes/AoC, app PCM brokerage, carrier audio, power and device gates
remain open. No physical phone was accessed.
