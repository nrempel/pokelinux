# Sounds queued intent in Rust

A route or explicit reconnect gesture made during a read-only status refresh now
belongs to a small app-local Rust core. Rust owns the copied logical route ID,
original one-second CLOCK_BOOTTIME deadline, remaining route TTL, fresh-state
revalidation, cancellation and one-shot send token. It preserves the first gesture
and never renews its budget while waiting, paginating or retrying an unsent packet.

Qt retains its QObject/QML surface, credentialed SEQPACKET transport, current
socket generation, bounded JSON/page validation and direct request state. It
supplies the same accepted status/catalog used by the UI. Rust cancels a queued
choice if current permission/availability/membership refuses it; the lifecycle
actor still independently owns permission and provider authority. The provider's
different STREAM/lease client is not reused at the app edge.

Preparation returns a copied command without consuming it. Every actual queued
send attempt recomputes remaining TTL against the original deadline. EAGAIN/EINTR
preserve that token and deadline; only a complete native send consumes it. Close,
reconnect, denial and malformed fresh state cannot replay a previous choice.
Applied output stays visible until an actual complete route send, as before.
Direct requests retain the existing1000ms first-packet shape.

The safe queue and layout modules forbid unsafe code. The narrow C ABI borrows
fixed bounded input/output objects on the one Qt event thread; it retains no
caller pointers, socket, thread, media lease or backend target. The native caller
owns handle/buffer validity and exclusive calls/free. This is the first portable
client-core extraction; transport, replies, other app clients and upstream graphics
still include C++.

## Actual checks

The genuine current C++ header first compiles cleanly under strict GCC14.2/C++17.
The next target adds only the absent Rust queue type/create symbol and fails with
exactly those missing names before production implementation. This is an actual
missing-API RED, without a behavioral bug claim.

The independent six-group C++→Rust consumer then passes copied first-wins intent,
final-page waiting, immutable deadlines/nonincreasing TTL, fresh refusal/removal,
explicit reconnect/no replay, real BOOTTIME expiry without timer delivery, and
sent/cancel token fences. Its declaration-only compiler setup correction and
production-header filename normalization retain every assertion and original5s
work/8s close bound. The failed setup aggregates are preserved.

The unchanged Qt peer suite passes all30 behavior cases plus setup/cleanup with
real Linux credentials and SEQPACKET sockets. Rust formatting, strict all-target
Clippy and Release pass; the Rust library has zero redundant unit tests. The
permanent CMake/CTest target executes the same six-group boundary. Default offline
Cargo/CMake builds and links the full Sounds application; its Rust archive is
byte-identical to the explicitly selected archive used by the Qt suite.

Nine separately closed phases retain54 actual outer direct waits and54 merged
PIPE EOFs, exact container absence on the same daemon, unchanged selected inputs
and original limits. An absent pkg-config tool, a C++ type-deduction setup error
and a wrapper option typo remain failed setup epochs. Their later corrected
selections are separate results. [Hash-only provenance](evidence/native-sounds-rust-queue-provenance.json)
binds the source, archive, full app and actual test records; private logs/binaries
are not published.

Prepared real GUI/PCM requalification, installed startup, coherent ARM selection
and every physical phone gate remain open for this new client. Earlier app/PCM
results retain their older client selection. No physical phone is accessed.
