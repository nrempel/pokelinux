# Media-only Rust audio client

The native Rust audio library now provides an owned, nonblocking client for the
existing audio service. It exposes logical Media choices and separates admitted
intent from independently observed application. The audio service remains the
sole routing authority. This milestone qualifies the transport component;
lifecycle permission and normal Sounds integration are separate work.

## Ownership and deadlines

`Client::from_stream(stream, expected_owner_uid)` takes ownership of an already
connected Unix stream, verifies its type and kernel peer UID, and assigns a fresh
process-local incarnation. UID0 verification trusts a root deployment; it does
not identify a unique actor or grant current app permission.

`inspect(until_ns)` and `select(route, until_ns)` attempt one explicit nonblocking
write. There is one operation in flight and no unsent command queue. Actual
EAGAIN preserves the earlier admitted/applied route. A complete send means a
possible effect, while admission and observed application remain distinct.
`poll()` only reads: it cannot dispatch, reconcile or reconnect.

The original absolute CLOCK_BOOTTIME deadline is checked before send and around
receive and publication, including a real buffered reply consumed after expiry
without timer delivery. Replies are bounded to64KiB. Catalogs contain bounded
logical IDs and labels; backend node names do not enter the shared route API.
Snapshots describe the last explicitly consumed reply. Read-only status may
withdraw or restore applied state without issuing another mutation. Call
preemption preserves admission.

`revoke()` shuts down and closes the connection and clears local lease state.
It provides no Release, Reconcile, Stop or automatic reconnect/replay operation.
Connection closure is not proof of remote lease or graph cleanup. A successor
must create a new stream/client and receives no inherited intent.

## Validation

The fixed new-feature scaffold initially lacked receive deadline fences. Its two
actual deadline controls fail before the source-only repair and pass afterward.
An earlier test-fixture correction explicitly accepts exact ECONNRESET when
expired unread bytes are present; empty-queue revocation still requires EOF.
Compilation/setup failures remain separate from behavioral RED.

Strict host formatting, lint and compilation pass, plus ARM64 cross-compilation.
The final default host run as UID1000 passes24 tests with one ignored:12 new
unprivileged client controls and12 existing authority/profile controls. The
isolated root run with `--include-ignored` passes25. Its additional test launches
the unchanged authored Rust audio Main under a distinct UID and exercises actual
catalog/acquire/route IPC, then asserts a successful owned child wait after an
explicit privileged fixture Stop. Its backend is unavailable; it does not test a
running PipeWire graph or proprietary vendor code.

Five owned build/runtime containers have actual waits, CLI pipe EOF and exact
same-daemon container absence. Test stdout/stderr are closed owned regular files;
the Main wait is asserted inside the test, not a separate external PID receipt.
The root-only ignore attribute is a packaging correction, without a new
behavioral RED claim. [Hash-only provenance](evidence/native-audio-client-provenance.json).

From `native-audio`, default component tests use `cargo test --locked --offline`.
The ignored case additionally needs a disposable root environment that can launch
a child as UID1000; it must be selected explicitly with `--include-ignored`.

## Remaining integration

The later [lifecycle integration](native-audio-lifecycle.md) composes this client
through an actor-owned provider in actual host permission/graph/PCM tests. Its
matching Qt client remains a separately selected component; normal target ARM
and production deployment remain open.

This client milestone has no ARM runtime, normal Qt, current lifecycle permission
or graph/PCM composition claim. Actual partial-write coverage is also open.
Continue the separate normal ARM Sounds tracer and current-authority actor
integration. Physical routes/AoC, app PCM brokerage, carrier audio, DND, volume,
haptics, power and device qualification remain open. No phone was accessed.
