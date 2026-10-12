# Current ringing observation protocol

Implementation checkpoint: 2026-10-12. Rust request/response types and the typed
TLS client path exist. Android routing, daemon observation and Linux presentation
are not connected. This is not proof of working background incoming calls.

## Request

One existing length-prefixed, mutually authenticated TLS exchange:

```json
{"v":1,"op":"watch","id":"11111111111111111111111111111111","wait_ms":25000}
```

A subsequent request may include both `after_generation` (32 lowercase hex
characters) and `after_revision` (integer). The cursor comes from a validated
snapshot. `wait_ms` is an integer from 0 through 25000. No call action, dial offer,
command admission or command session is present.

## Response

```json
{"v":1,"id":"11111111111111111111111111111111","generation":"22222222222222222222222222222222","revision":1,"source":"android.telecom.InCallService","source_state":"observed","ringing":[{"id":7,"episode":"33333333333333333333333333333333"}]}
```

The request identity must match. Revision is an integer from 1 through
9007199254740991. There are at most 64 ringing entries. Call IDs are distinct
positive signed-32-bit integers; episode identities are distinct 32-character
lowercase hex values. Unknown and duplicate fields are rejected. The response
is at most 65536 bytes; requests remain limited to 4096 bytes.

Source states are `observed`, `detached`, and `unavailable`. Only `observed` may
carry ringing entries. An empty registry or detached InCallService is not proof
that the modem is idle. Episode identities identify a ringing episode, not
permission to answer it. Actions still require fresh command controls.

## Transport deadlines

`TlsClient::watch` accepts only a `WatchQuery`, constructs the message internally,
and validates the complete reply before the acceptance deadline. It cannot be
used to send arbitrary command bytes under the longer watch deadline.

Queries with waits of at most 1000 milliseconds get a five-second total budget;
longer waits get 30 seconds. Ordinary exchanges retain their five-second maximum,
even when their caller supplies a longer budget. Deadlines use CLOCK_BOOTTIME
and are checked around I/O and after decoding. There is no retry in the transport.
Observation errors return no snapshot; they do not create command receipts.

The future observer must confirm ringing received from a long watch through a
short query before presentation. A successful reply alone does not establish a
presentation lease. Lease time must start before the short query, and an
independent expiry mechanism must withdraw stale presentation. See
`pro-background-observation-review.md` for the full lifecycle contract.

## Local verification scope

`runtime/tests/watch.rs` checks the schema and identity bounds. The TLS fixture
uses real local mutual TLS with disposable certificates. It checks typed reply
validation and concurrent long-watch, short-watch and command deadlines against
delayed replies. Existing TLS tests check framing, trust, client authentication,
request/response bounds and absolute deadlines. These are Linux container tests,
not Android emulator or Pixel hardware results.

## Android source checkpoint

`RingingRegistry` now maintains immutable current snapshots on the main looper.
Unchanged state keeps its revision. A call that leaves and then re-enters the
observed ringing set gets a fresh episode identity. A generation change also
starts new episodes. Source-state changes advance the revision, and invalid
input cannot replace the current snapshot. Host tests cover these properties
and compile the registry with Java 8 source/target settings.

CallService source now samples this registry before publishing its wake signal,
registers callbacks before the initial sample, and rejects callbacks whose call
or observation session is no longer current. These Android changes have not
been compiled or exercised on Android: the local SDK is absent and Harbour
SSH still requires Tailscale authentication. Host registry tests do not verify
Android callback order, binding, role changes or lifecycle. The registry describes
the bound call set; the future watch endpoint must separately enforce role and
enrollment prerequisites before returning a snapshot. No endpoint was added in
this checkpoint, and long waits still must not run in GatewayService workers.

## Local Android compilation and parser verification

The previous SDK compile blocker is now resolved locally. The pinned public
platform `platform-37.0_r02.zip` was downloaded and its existing repository SHA-1
checked before extraction; SHA-256 values are recorded in
`artifacts/rust-native-local/android-local-compile-result.json`. All production
bridge Java files compile against this platform's `android.jar` with Java 8
source/target settings. This supersedes the earlier uncompiled-source note,
not the outstanding emulator/lifecycle checks.

`WireRequest` now parses `watch` and its bounded wait and paired cursor. The
regression test runs against unchanged, hash-checked Android 17 AOSP JsonReader
and four dependencies on the host JVM. Only the build-tool annotation has a
compile-only stand-in; no JSON parser is mocked. Source URLs and SHA-256 values
are in `artifacts/rust-native-local/aosp-json-provenance.json`. Download those
files into a separate directory and run:

```sh
sh tools/test-wire-request.sh /path/to/aosp-json
sh tools/test-control-policy.sh
```

Both passed. The parser test first failed on the new `wait_ms` field before
implementation. It checks bounds, numeric types, cursor pairing, duplicate and
trailing fields, and rejection of command fields in a watch. GatewayService
still rejects the new operation after parsing: dedicated watch execution and
response routing are the next integration step. No APK packaging, emulator
execution, or physical-phone access took place in this checkpoint.

## Watch endpoint source integration

GatewayService now routes authenticated, parsed watch requests to two dedicated
workers with a SynchronousQueue (no waiting task queue). Authentication and
request reading retain the existing bounded pool and initial deadline. Rejected
watch submissions close their socket. Command admission remains on its existing
path. A connection owns its scheduled closure across the handoff; only a watch
may extend the deadline, and the extension is measured from accept. Short watches
retain five seconds; long watches get at most 30 seconds.

CallWatch captures the wake cursor before each fresh main-looper sample. Matching
wire cursors wait; publication or timeout causes another main-looper sample.
Each sample has a two-second FutureTask bound. Missing dialer role, expiry,
shutdown, closed sockets or failed samples return no snapshot. Enrollment and
peer-certificate expiry are checked before accepting an observation; service
shutdown closes sockets and interrupts watch workers. Timer closure wakes watch
waiters. Network serialization and writes run outside the main looper.

All production Java sources compile against the pinned Android 17 SDK. Host
CallWatch tests cover a wake during sampling, timeout resampling, initial state,
and guard failure before and after sampling. Parser and existing policy tests
pass. See `watch-endpoint-result.json`, `watch-endpoint-compile.log` and
`watch-loop-{red,green}.log` under `artifacts/rust-native-local`.

This is source integration, not Android runtime proof. Still required: actual
mutual TLS watch replies decoded by Rust, command traffic under saturated watch
capacity, timer/handoff races, stalled main looper, role loss, service stop,
enrollment expiry/revocation and real call lifecycle callbacks. The Rust daemon
observer and Linux presentation are not connected. No new packaged APK or
physical-device execution is claimed.

## Shutdown review update

The original SSLServerSocket/SSLSocket-close ownership described above has been
replaced by raw TCP acceptance plus server-mode mutual-TLS wrapping. Cancellation
closes the independently owned raw socket outside the connection monitor; worker
cleanup closes TLS afterward. See `pro-watch-endpoint-review.md` for the tested
change, independent review and remaining capacity/deadline/callback work.

## Confirmation reservation and final-sample allowance

Watch capacity is now split into one positive-wait worker and one zero-wait
confirmation worker, with no queues. The hold is limited by the remaining
connection budget with 2500 milliseconds reserved for final sampling and output;
a main-looper sample can wait no longer than min(2000 milliseconds, remaining
connection budget). Source/tests and limits are recorded in
`pro-watch-endpoint-review.md`. This supersedes the earlier two interchangeable
watch-worker description. It does not establish Android runtime or suspend proof.

## Credential cutoff

Authenticated exchanges now anchor the earlier enrollment/certificate expiry to
elapsed time and cap both socket deadlines and command admission with it. Watch
extension cannot move that anchored cutoff. Wall time is still checked for
forward changes. See the credential lifetime section of
`pro-watch-endpoint-review.md` for tests and limits, especially suspend behavior.
