# Native scope conversation primitives

This report retains the13-component selection published at `df73965`. The later
[conversation I/O selection](native-auth-conversation-io.md) exercises the exact
extracted transport methods and passes19 host components. It adds separate
ARM64 compilation, without actual Attempt, PAM, cgroup or ARM runtime evidence.

The existing Rust PAM wire emits Ready plus a hidden prompt through real private
Linux pipes, while the scope parser rejects that valid prompt. This actual host
RED has8 passes and1 runtime failure; the terminal-candidate control and seven
prior components pass. The parser was extracted without changing its behavior
before this selection. No PAM, helper, actual scope or ARM guest executes here.

The parser now retains one complete sequential hidden prompt in1..32, handles
fragmentation and rejects duplicate/out-of-order/pipelined records and a terminal
candidate while a prompt is outstanding. A prompt sends no terminal ACK. The
isolated Attempt API observes that prompt and queues one matching1..512-byte,
nonempty, non-NUL response under the original absolute deadline. No broker or
parent/actor transport invokes those APIs yet.

The private response frame owns a Zeroizing Vec for the entire frame, at most517
bytes. Nonblocking writes advance an offset rather than draining secret prefixes.
EAGAIN leaves ownership intact. Completion drops the frame; retirement drops it
before potentially blocked cgroup work, including rejected-call input before
retirement. Admission rechecks the original deadline after frame construction.
These are source/ownership properties; no allocator-scrubbing experiment runs.

The fresh host GREEN passes13 components, formatting and strict all-target /
all-feature Clippy. The existing wire receives the standalone frame through real
pipes in a joined thread. A separately filled nonblocking pipe forces EAGAIN at
offset0, then accepts the whole maximum frame after draining; another completion
call adds no duplicate bytes. Bounds, fragmented/malformed ordering, retired
parser state and the old terminal behavior pass. The first GREEN stops at a
Clippy collapsible-if error before any tests and remains failed; a let-chain-only
correction precedes this fresh selection. Actual test/container waits, same-daemon
exact container absence and all11 frozen source bindings close. The test ELF is
retained. See [bounded proof](evidence/native-auth-conversation-provenance.json).

The13 tests in this selection call the parser, Barrier::retire and standalone ResponseFrame directly.
They do not construct an actual Attempt or call pending_hidden_prompt,
respond_hidden_prompt, write_pending's response branch, or queued-response
Attempt cancellation/deadline cleanup. They do not run a second prompt after a
response flush, an actual partial write, EINTR or write-error injection. Those
paths require their own actual composition controls. This is host component
progress, not original PAM, full native scope or ARM integration evidence.

The crate remains isolated-feature-only; default builds expose no authentication
implementation. The supervisor/parent command and observation channels stay
unchanged, so the fixture cannot yet forward this prompt/response from its
outside caller. Candidate buffering remains separate from result authority.
Original PAM with owned scope, preserved effective faillock, the fixed broker,
actor one-shot exact-unlock restoration, synced durable clear and a normal
Messages read remain open. Older ARM scope/containment results keep their exact
preceding source selections; they do not qualify this new conversation code.
Matched Google, secure frames and every physical-device gate also remain open.
