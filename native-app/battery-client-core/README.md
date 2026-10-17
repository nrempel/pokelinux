# Read-only battery client

Rust owns the fixed Unix stream endpoint `/run/pokelinux-battery/snapshot.sock`,
root:root peer validation, bounded nonblocking connect/read, strict schema1 JSON,
and all publication/expiry decisions. It sends no requests or commands. A complete
single JSON line followed by EOF finishes a transaction; partial EOF, unavailable
state, malformed data, missing endpoint, transport failure, original request
deadline or publication expiry clears capacity, status and revision.

Refresh is250ms; one original500ms CLOCK_BOOTTIME request deadline cannot be
renewed by partial bytes. Replies are at most4096bytes and may expire at most2s
in the future. Unavailable replies require the deadline sentinel"0" and explicit
null capacity/status. Production resamples CLOCK_BOOTTIME after bounded I/O and
checks the original request deadline before accepting a reply. Every getter also
samples CLOCK_BOOTTIME, including after suspend or a late Qt timer. The Qt adapter supplies a100ms timer and scalar presentation only.
Its normal constructor has no endpoint argument. No Source, grant, permission,
Android bridge or command authority enters this public coarse-state interface.

The four-operation versioned C ABI copies fields and retains no caller pointer.
Rust owns and closes pending sockets. Calls and free must not overlap; callers
must supply live handles and complete aligned disjoint output slots. Null and
misaligned pointers are refused; valid-looking dangling pointers remain caller
misuse. The header defines status numbers and unavailable capacity(-1).

Seven frozen real Unix boundary groups first compiled and failed against the
nonconnecting stub, then passed unchanged against this implementation. An eighth
C ABI pointer/lifetime group, strict all-target Clippy and the optimized staticlib
build also passed in a pinned offline Rust1.98.1 container. Three later protocol
controls produced two actual unavailable-sentinel assertion failures while the
required-nullable-field control passed; the parser was then corrected. The final
formatted source passes all11 groups, formatting check, strict Clippy and the
release archive build. A prior lock setup failure is separate. Pinned rustfmt
changed formatting only.

The boundary tests cover valid root peers, all five statuses, absence, malformed
and duplicate JSON, invalid fields/ranges/horizons, unavailable/null state, partial
EOF, delayed partial reply, the original request deadline, getter expiry and zero
request bytes. Their controlled time uses the same policy with a Rust-private
endpoint constructor. They do not prove physical suspend, rejection of an actual
nonroot server, native Qt/app composition or service integration. No phone,
physical battery, VM, screenshot or host power-supply driver execution is claimed.

A separate locked/offline ARM64 release staticlib crossbuild passes with432
AArch64 ELF objects and the same four strong C exports. This executes no ARM
code or tests; the host component results above remain separate.

Later native Qt composition and prepared kernel-to-visible battery results are
recorded [separately](../../docs/native-battery-ui.md). They use the real Rust
client in one normal app/window; physical suspend and battery hardware remain
unqualified.
