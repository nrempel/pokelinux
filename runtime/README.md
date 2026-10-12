# Native phone runtime

Rust implementation for ARM64 Linux with one current local RPC format. It now
includes enrollment loading, peer authorization, service session fencing,
durable command coordination, pinned mutual TLS, receipt recovery, private
socket ownership and the `omarchy-phoned` executable. The native Linux process
startup/restart path is tested. Installed ARM64 service packaging, the CLI,
coarse phone-status validation, observation RPCs and native UI integration remain
unfinished. Do not ship this as the complete smartphone service yet.

The local command shape has `v`, `id`, `epoch`, `session`, and a nested `command`.
The command's `kind` is `dial` or `control`. Controls use named states, with valid
state/action pairs enforced before admission. All identities are 32 lowercase
hex digits. Numbers are 3–32 ASCII digits with an optional leading plus. These
are syntax rules; permission, emergency classification, freshness and command
ownership remain service/Android requirements.

Frames are single-line JSON objects ending with a newline, at most 16,384 bytes
including that newline. Duplicate and unknown fields are rejected. The Linux
socket reader caps an absolute read budget at two seconds and restores the
socket's previous timeout. The connection owner must close after one exchange;
this is not a persistent multiplexed protocol.

`Cargo.lock` pins dependencies. `build-support/Dockerfile.rust` pins the Rust
base image and provides an ARM64 cross compiler and QEMU user-mode runner.
`tools/rust-suite.sh` runs formatting, Clippy, native Linux tests and ARM64
executable tests with Cargo offline. Run it in that image with read-only source,
a populated Cargo cache and writable target/result mounts, networking disabled.

Host macOS can run the parser tests. Socket code and tests are explicitly Linux
only. Darwin rejects changing a socket timeout after the peer closes; this was
reproduced separately and is not a reason to add an unsupported platform layer.
Linux and ARM64 socket tests include that peer-close path.

QEMU user-mode execution verifies ARM64 machine code against Linux syscalls. It
is not an Arch installation, Android integration, Pixel boot or hardware proof.

## Durable admission journal

`Ledger::reserve` returns `New` only after SQLite commits the receipt. Only that
result permits one dispatch by the future service owner. An existing receipt,
an error or a client disconnect must never cause another dispatch. The journal
stores the command digest and outcome, not phone numbers or replayable payloads.
It persists its epoch, rejects changed-payload identity reuse, permits only one
unfinished operation and makes terminal outcomes immutable. Startup converts
pending operations to unknown. Explicit acknowledgement clears the admission
block but neither changes the outcome nor allows replay. There is no artificial
lifetime command-count cap; compact receipts remain retained for deduplication.

The directory and files require current-user ownership, private permissions and
no symlinks. A process lock gives one owner. SQLite uses EXTRA synchronization and
DELETE journaling. Unsupported schema versions fail; there is no old-format
reader or migration. An interrupted initial creation can leave an unusable new
journal; opening it fails rather than silently resetting potentially useful state.

Current Harbour evidence: 46 native Linux component tests pass, including actual SIGKILL
after committed admission and SQLite deferred-constraint failures at commit.
44 component tests pass as ARM64 executables under QEMU user-mode. The native crash helper
is intentionally ignored by normal test discovery and launched explicitly by the
SIGKILL test. Formatting and Clippy pass; source hashes match the local files.
The real daemon must still enforce peer permissions, session ownership and
freshness, and own the commit-to-dispatch handoff independently of client tasks.
These component tests do not yet prove that service integration.

## Android transport checkpoint

`TlsClient` uses TLS 1.3, an enrollment-specific root store, server-name validation,
client authentication and a SHA256 leaf-certificate pin. It completes the handshake
and checks the pin before application writes. It disables resumption and early
data, uses numeric socket addresses, and never retries. One length-prefixed
request and response use each connection. After the first write attempt, errors
are `Unknown`; `NotSent` is reserved for failures before application submission.
A successful exchange returns raw bounded response bytes, not a trusted execution
receipt. The control/dial protocol module validates raw replies directly; the final service
must use it and preserve unknown outcomes on any decode failure.

Deadline checks use Linux CLOCK_BOOTTIME, which includes suspend. Checks after
I/O reject late acceptance, but cannot guarantee that a blocked thread wakes
while suspended. Simulated clock advancement is covered; real suspend remains
unverified. Android currently bounds requests to 4096 bytes and replies to 65536;
the transport's generic 65536-byte outgoing cap still needs alignment with the
final typed request encoder.

`artifacts/rust-android/` proves native Linux and ARM64 Rust interoperability with
the real Android emulator service and disposable enrollment. Each architecture
passes status, controls and dial-availability reads, normal JSON field reordering,
strict rejection cases, and rejection of valid commands while controls are disabled.
There is no executed-call, installed Rust daemon, or Pixel proof in this result.
The harness uses ADB forwarding inside the isolated emulator network namespace.

This integration exposed the Java endpoint's field-order-dependent regular
expressions. `WireRequest.java` now uses Android's streaming JSON reader, rejects
duplicates (including escaped-equivalent names), unknown fields, invalid types,
malformed UTF-8, trailing data and invalid command bounds. It accepts structural
JSON regardless of field order. All six existing Java call-policy suites pass.
The unsigned APK's executable payload matches the tested APK, with current source
hashes verified. No compatibility parser or fallback was added.

Next: coarse phone-status validation, a daemon-owned
commit-to-dispatch path, and installed Arch service/command integration. Keep local
`NotSent` distinct from Android's authenticated `not_invoked` receipt in the journal.

## Typed Android contract checkpoint

`android_protocol` now encodes validated local commands into Android requests
and decodes replies directly into strict types. It binds each command receipt to
its request ID, Android generation and operation. Unknown fields, duplicate keys
(including escaped-equivalent names), trailing JSON, invalid UTF-8, wrong types,
oversize frames and invalid outcome/reason pairs fail validation. A returned
invocation is not evidence that a call connected.

Control observations enforce revision and call bounds, unique call IDs, coherent
availability, valid state/action pairs and absence of actions when authorization
or single-call conditions do not hold. Dial observations require a valid offer
only when ready, and preserve pending/unavailable distinctions.

Five new contract test cases pass on Linux and ARM64. The real emulator test now
uses these exact control/dial decoders, command encoder and receipt decoder on
both architectures. Rejected commands are tested with controls disabled; actual
Rust call execution and the installed daemon remain unverified. The coarse
`status` test still inspects JSON as test data and is not a production validator.

Keep transport `NotSent` separate from Android `not_invoked` when implementing
journal completion. Ensure the final typed client covers validation within its
acceptance deadline and maps any invalid post-send reply to unknown. UI/client
cancellation must never cancel the daemon-owned dispatch or cause automatic replay.


## Enrollment files

`Enrollment::load` reads one current-user-owned directory with mode 0700.
It accepts only `endpoint.json`, `ca.pem`, `client.pem` and `client.key` as
its fixed input paths. The JSON fields are `endpoint` (numeric loopback address
and nonzero port), `server_name` and `server_sha256` (64 lowercase hex digits).
There is no reader for an earlier client configuration.

The loader opens the directory once and opens files relative to that descriptor.
It rejects symlinks, non-regular files, wrong owners, multiple hard links and
unsafe permissions. The key must have no group or other access; other files
must have no group or other write permission. Reads are bounded to 4096 bytes
for configuration and 65536 bytes per PEM file. Loading does not connect,
create credentials, change permissions or repair files. The raw key buffer is
zeroized on release. Rustls validates that the key matches the client certificate.

Focused loader tests and the complete native Linux / ARM64 component suites
pass. The real Android emulator test also passes with this exact loader on native
Linux and ARM64 QEMU user-mode. Both exercise authenticated observations and
disabled-command receipts. Disposable test staging is separate from production
loading. This does not prove installed daemon behavior or Pixel hardware.


## Durable dispatch boundary

`dispatch::execute` connects the journal, command encoder, TLS transport and
receipt validator. Only a newly committed admission permits a send. Existing
receipts return without a connection, including after restart. A malformed or
mismatched post-send receipt becomes unknown. Transport failures before an
application write produce a distinct `NotSent` receipt, never an Android
`DialNotInvoked` receipt. Receipt validation shares the transport deadline.
A terminal journal commit must succeed before the outcome is returned.

The target journal is schema 2. Earlier schemas are rejected without migration.
This change does not authorize deleting any previous journals.

The service must still authorize the peer and session before dispatch and own
execution independently of UI disconnection. This module is not yet a daemon.
Real TLS peer tests observe the committed SQLite admission before serving the
request, check malformed and mismatched receipts, and verify no connection on
replay after restart. Android integration of this exact path passes on native Linux and ARM64.
Both record the real Android disabled-command receipts, reopen SQLite, and
return the stored outcomes with a zero network budget. This is not proof of
executed calls, installed service authorization or Pixel hardware.


## Local authorization

`authorization::Session` owns one random service-incarnation ID. Both Phone and
shell clients receive the same ID after a Linux `SO_PEERCRED` same-UID check.
An app does not claim or replace the session. New service instances get a new
ID, so requests from an earlier service instance fail. The kernel peer check
runs before request parsing or session disclosure, with no cross-socket cache.

This is a trusted desktop-user boundary, not isolation between same-UID apps.
Linux peer credentials describe connection creation; they do not prove which
process currently holds a passed socket. The service must run under a fixed UID,
protect its socket directory, and serialize command admission. These component
APIs do not yet provide the complete installed service or lock-screen policy.

Native Linux and ARM64 tests cover two app surfaces and restart fencing.
`tools/rust-peer-suite.sh` additionally runs an isolated native Linux test with
UID 65534 connecting to the root test service, using only SETUID/SETGID container
capabilities. Both session disclosure and request reading return permission
denied. Production services do not need those capabilities.


## Recovery lookup

`Ledger::lookup(epoch, id)` reads the stored outcome and acknowledgement flag
without admitting, cancelling or dispatching a command. An absent result is
only absence at lookup time; a delayed complete request can still arrive.
`acknowledge` requires the exact journal epoch and operation ID. It accepts only
unknown outcomes, preserves that outcome and the replay guard, and is idempotent.
A late acknowledgement for one operation cannot clear another operation.

Tests use real SQLite files, verify unchanged bytes on absent lookup, reopen
the journal, and confirm that acknowledged unknown receipts remain unknown and
cannot dispatch again. The service RPC and native UI must use this read-only
lookup after a lost response; they must not refresh and resubmit old intent.

The Android emulator integration passes on both architectures with the current
source: local peer authorization, UI socket close, durable dispatch, actual
Android disabled-command receipt, journal reopen, read-only lookup, and duplicate
suppression. These are component boundaries exercised together, not an installed
Rust daemon or successful physical call.


## Service-owned command coordinator

`Coordinator` combines same-UID/session authorization, exclusive ledger ownership,
durable admission and a service-owned worker. It checks the full local frame
before admission. Only a new committed reservation starts a worker; duplicates
return the existing result and other actions encounter the pending/unknown gate.
The journal mutex is not held across TLS I/O, so lookup and busy responses remain
available. A competing action is rejected rather than queued for later execution.

The worker owns the immutable TLS configuration and parsed request. It does not
borrow the local socket. Dropping the coordinator joins its worker before the
coordinator releases its ledger ownership. A worker/commit failure preserves
pending state for recovery, and thread-start failure records NotSent when that
receipt can be committed. No shutdown or UI close path erases a reservation.

A real TLS peer test pauses the reply while duplicate and competing commands
arrive, verifies the second command has no journal entry or network connection,
and reopens the journal after coordinator shutdown to recover the first receipt.
The full executable, socket-path lifecycle, local RPC/CLI and installed service
remain to be connected and verified. Coordinator construction is not yet proof
of systemd or complete service deployment.

The coordinator also passes the real Android emulator test on native Linux and
ARM64: it completes a disabled-dial rejection after UI disconnect, persists that
receipt, changes session on restart, recovers the receipt and rejects the old
command session. No real call invocation or Pixel hardware is proved by this.


## Local socket and RPC boundary

`ServiceSocket` binds `bridge.sock` in an existing current-user-owned private
0700 directory. It retains a 0600 file lock, rejects a second owner, and does not
unlink active sockets or unrelated files. A nonblocking probe permits cleanup
only after connection refusal establishes a stale socket. Drop removes the socket
only when device/inode identity still matches. It leaves the lock inode intact.

Linux tests include actual SIGKILL of a separate socket owner and recovery by a
new owner, plus preservation of active listeners, regular files, symlinks and
replacement files. ARM64 tests cover the same non-process socket paths. This is
not yet the installed service's signal/shutdown integration.

`local_protocol::decode` defines one tagged RPC schema: hello, submit, lookup and
acknowledge. Submit carries the validated command envelope; lookup and acknowledge
carry epoch and target operation ID. Frames require one bounded newline-terminated
JSON object. Duplicate tags and fields, unknown fields, invalid versions/types,
and invalid command state/action pairs are rejected. No previous-format fallback
is part of the RPC decoder. The listener and decoder still need to be connected
to coordinator handling and the service executable before deployment.


## Service executable checkpoint

`omarchy-phoned --enrollment /absolute/enrollment --state /absolute/state
--runtime /absolute/runtime` loads enrollment, acquires the persistent journal
owner, and publishes the private socket. Runtime and enrollment directories must
already exist with the required private permissions. The state directory is
created if absent and must be outside the runtime directory.

The executable waits for socket activity or a signal using poll and a signal
self-pipe, with no periodic idle wake. It accepts at most 16 concurrent local
connections. Excess connections close before parsing/admission. SIGTERM/SIGINT
stop acceptance, join bounded local handlers, remove the owned socket, and join
admitted command work before releasing the ledger. Persistent storage stalls
can still delay shutdown; no physical power-loss guarantee is claimed.

The real native Linux executable test performs hello over its filesystem socket,
uses SIGTERM, checks socket cleanup, restarts, and verifies preserved journal
epoch plus a new process session. ARM64 builds the executable but does not yet
run this process test. Installed ARM64 service, systemd packaging and CLI work
remain. The current RPC handler implements hello, submit, lookup and acknowledge;
observation methods and native client response validation are still needed.

Signal API reference: https://docs.rs/signal-hook/latest/signal_hook/low_level/pipe/fn.register.html
Poll API reference: https://docs.rs/rustix/latest/rustix/event/fn.poll.html

The latest component run reports success after connecting the RPC handler and
service executable. Its evidence download is pending renewed Tailscale SSH
sign-in; the local rust-core inventory still describes the earlier checkpoint.
Do not claim current source-hash verification until those files are retrieved.


## Phone-status parser: local checkpoint

`decode_status` now validates the coarse Android status reply directly from
bounded bytes. It requires matching request identity, a valid generation,
bounded Android elapsed-realtime timestamp, unique bounded call IDs/states,
coherent Telecom availability and dialer role, and the exact default-subscription
phone-state source/scope. Explicit unavailable/null phone state is accepted;
a missing nullable state is rejected. Telecom call data and the TelephonyCallback
signal remain independent observations. Neither grants command authority.

The three new status tests and twelve existing parser tests pass on macOS, with
Clippy for those targets. `artifacts/rust-local/` records this limited evidence.
The emulator test now calls this decoder, but Linux/ARM64 and Android runs for
this new change await Harbour sign-in approval. Do not infer target verification
from local parser success or older emulator artifacts.


## Native-client reply validator: local checkpoint

`local_reply::decode` binds each response directly to the expected request ID
and operation. Recovery replies must match the exact journal epoch and target.
Submission outcomes must match dial versus control commands; no outcome means
that a call connected. A receipt must explicitly contain null or a typed record,
and acknowledged records must retain an unknown outcome. Unknown error codes,
duplicate/unknown fields, wrong types and invalid framing are rejected.

The server reply integration test now uses this validator before inspecting
JSON as test data. That Linux test change has not run yet. Twenty portable parser
tests and Clippy pass locally; target and Android verification remain separate.


## Native ARM64 Linux local verification

An isolated local Docker run now passes 54 component tests on native ARM64
Linux using the same pinned official Rust base. Source hashes are verified in
`artifacts/rust-native-local/`. This includes the new phone-status decoder,
client reply validator and real server-reply/client-decoder boundary. It does
not include Android or the x86-only child-process tests. Harbour sign-in remains
pending, but local Linux development and verification can continue without it.

## Native client transport and ARM64 daemon checkpoint

`local_client::exchange` now performs one validated request/reply over a Unix
socket. It checks the service's kernel peer UID before writing, uses nonblocking
connection admission, caps the total acceptance deadline at two seconds using
CLOCK_BOOTTIME, and never reconnects or replays. Invalid requests and connection
failures are `NotSent`; failures after entering the write phase are conservatively
`Unknown`. A caller must retain the original command ID for receipt recovery.
The command-line executable uses this transport; native app integration remains
unfinished.

The local native ARM64 Linux suite passes 58 tests. A separately selected process
test starts the actual daemon, uses this production client for hello, stops it
with SIGTERM, and verifies restart preserves the journal epoch while changing
the service session. ARM64 child execution is explicitly selected on a native
host so the Harbour QEMU runner does not attempt unsupported child execution.
Evidence is in `artifacts/rust-native-local/{tests.log,daemon.log,result.json}`
and `sources.sha256`. The native validation image extends the pinned base with Clippy and rustfmt.
Both checks now pass with warnings denied.
These checks do not exercise Android, Pixel hardware, or installed app packaging.


## Native command-line interface

`omarchy-phone --socket /absolute/runtime/bridge.sock rpc` reads one bounded,
newline-terminated current-protocol request from standard input through EOF.
It prints only the validated reply as JSON. The caller supplies the request ID,
epoch and session where required; the CLI does not create replacement intent,
refresh stale sessions, reconnect or retry. Keep command contents out of process
arguments. This interface is for native app adapters and diagnostic tools.

Exit codes:

- `0`: a validated non-error reply was printed. Read its outcome; this does not
  mean a call connected or even that a submitted command finished.
- `2`: invalid invocation/input or failure before command submission.
- `3`: uncertain transport result or failure delivering the reply to the caller.
  Recover using the original command ID; do not create and send a new command.
- `4`: a validated service error was printed; inspect its typed code.

The latest local run passes 58 component tests and three native process tests:
service restart recovery, CLI hello/receipt lookup/service rejection/input
redaction, and CLI lost-reply handling without reconnect. Formatting and Linux
Clippy with warnings denied also pass. These are local ARM64 Linux results;
Harbour Android reruns, observation RPCs, installed packaging, and native app
integration remain pending.

## Status observation RPC

The current local protocol accepts `{"v":1,"id":"<32 lowercase hex>","op":"status"}`
followed by a newline. The service authenticates the local peer and fetches Android
status over its enrolled TLS connection with a one-second total exchange budget.
It returns `result.kind=status` and a typed `snapshot`. The snapshot preserves
Android's generation and elapsed-realtime observation time, Telecom calls, and
independent default-subscription phone state. It is an observation, not authority
to execute a call command. Failure returns the typed `unavailable` service error;
there is no stale cached success, automatic retry, or command journal admission.

Both service and client validate the snapshot schema and request identity. The
native Linux integration test exercises local RPC plus an actual mutually
authenticated TLS test peer, including mismatched IDs, duplicate JSON keys and
oversized replies. The client separately rejects invalid nested calls and
identity mismatches. Latest native evidence: 60 component tests plus three
process tests, formatting and Clippy pass. This new RPC has not yet run against
the Android emulator; the TLS test peer is synthetic. Controls and dial-status
RPCs and installed native app wiring remain to be completed.

## Controls and dial availability RPCs

The local protocol now also accepts `controls` and `dial_status`, with the same
version and request-ID fields as `status`. Each returns a same-named result kind
and a strictly validated `snapshot`. Controls retain call IDs/states/actions,
Android generation and revision. Dial availability retains generation, the
current offer when available, pending state and the typed reason. Disabled
availability is a valid observation; transport or malformed-data failure is an
`unavailable` service error. The service performs no command admission for these
queries. Android must revalidate each later command against its current state.

Native transport coverage exercises all three operations through local RPC and
mutually authenticated TLS, including wrong IDs, duplicate fields and oversized
responses. Client checks reject incoherent actions/offers and preserve disabled
states. Latest local evidence: 61 component tests, three real daemon/CLI process
tests, formatting and Clippy pass. The new ignored emulator test
`local_rpc_observes_real_android_framework` is compiled and ready for the isolated
Harbour suite, but has not run. Native app and installed package integration
remain unfinished; synthetic transport evidence does not establish Android or
Pixel hardware behavior.

## App restart recovery discovery

The `recovery` RPC takes `v`, request `id`, and the journal `epoch` obtained from
hello. It returns the same epoch and `items`, containing the unfinished command's
ID and `pending` or `unknown` outcome. An empty list is explicit. The journal
admission rule permits at most one unresolved command; both service and client
reject a response that violates this bound. A stale journal epoch is refused.
The query takes one journal lock for the complete snapshot, requires the local
peer UID, and performs no Android connection, command admission, or replay.

This lets a restarted app find uncertainty without retaining a command ID in its
own process. It does not cancel commands or declare absence permanent. Explicit
acknowledgement removes an unknown item from the unresolved view while preserving
its durable receipt and duplicate-command protection. Pending work cannot be
acknowledged away.

Current native evidence: 63 component tests, three process integration tests,
formatting and Clippy pass. The new recovery test reserves a command, reopens the
journal to recover it as unknown, discovers it through RPC, rejects a stale epoch,
and verifies explicit acknowledgement clears the unresolved view without opening
an Android connection. Native UI and package replacement remain unfinished.

## Native ARM64 runtime package

`tools/build-client-package.py` now packages the target Rust runtime only. Supply
`--binaries <release directory> --arch aarch64 --output <package path>`.
The builder rejects script inputs and mismatched ELF architectures. The archive
contains `omarchy-phoned`, the `omarchy-phone` RPC CLI, and the user unit. It does
not contain Python runtime modules, the earlier UI launcher, enrollment keys,
activation hooks, or an enabled service link. The user unit uses the current
fixed enrollment directory and private systemd runtime/state directories.

The release build produced `artifacts/client/omarchy-phone-0.1.0-1-aarch64.pkg.tar`.
Three package tests pass. The exact extracted binaries ran on local native ARM64
Linux: CLI hello succeeded, SIGTERM removed the socket, and restart preserved
the journal epoch while renewing the service session. Dynamic dependencies
observed with readelf are libgcc_s, libm and libc. Evidence and the package hash
are in `artifacts/rust-native-local/package-result.json` and `package-elf.log`.
This is a runtime package, not the completed smartphone app bundle. Pacman
installation, systemd activation, Android enrollment and native UI wiring still
need installed integration tests.

Fixture failures were investigated before accepting the smoke result: rustls
rejected keys produced by the system LibreSSL test commands; a fresh OpenSSL 3
P-256 identity validated. An initial runtime/state placement on the macOS shared
mount returned EINVAL. The successful test used container-local Linux runtime
and state directories. These findings do not justify weakening key checks or
claiming support for state on a shared host filesystem.
