# Native scope parent/supervisor prompt forwarding

The isolated Rust parent and supervisor now implement hidden-prompt/response
forwarding in source. Forty host components pass after two separately qualified
REDs, with formatting, strict default/all-target/all-feature Clippy and separate
ARM64 release compilation. The full ScopeChild/Attempt process path is not run
here. [Bounded provenance](evidence/native-auth-parent-forwarding-provenance.json)
keeps that limit separate from the actual real-pipe component execution.

The production record decoder first accepts a synthetic healthy state through a
real pipe, then refuses the authored credential-free prompt event. Nineteen
components pass and this one fails; actual pipe EOF precedes the assertion.
No child scope, PAM or cgroup executes. A separate standalone extraction keeps
the fixture's legacy per-byte C/S/? selection behind new codec API scaffolding:
1 test passes and11 fail. The repaired standalone codec passes12. This is a
runtime command-selection RED, not execution of the original complete fixture.
Its cache/noexec preparation failures remain separate with zero runtime tests.

PKSCOPE3 adds a48-byte credential-free prompt record containing exact worker PID
and sequence1..32. Reserved fields remain zero; PKSCOPE2 is refused. The supervisor
queues current state before its first prompt, then emits each sequence once.
The parent requires a healthy ready/start state, exact worker and next sequence
before retaining a prompt. Responses carry only the matching sequence and
1..512 non-NUL bytes on the private command pipe. They never enter observation
records. Candidate buffering still supplies no result or unlock authority.

The command decoder bounds its public header and one Zeroizing response body,
supports arbitrary fragmentation and multiple commands, and permanently refuses
malformed frames or callback failures. Control characters inside a declared
body remain body bytes. In-band C/S cannot interrupt an incomplete body; local
owner retirement is independent of that stream.

Each owner retains its ingress decoder and clears partial response bytes before
its retirement work. The fixture decodes to bounded local intent first, drops
its zeroizing read buffer, then applies intent to the target. A retiring batch
retains no response, preserves the first C/S cause and clears duplicate responses.
This also prevents read-buffer tails from surviving into a cgroup call. These
are source/ownership properties, not allocator-scrubbing measurements.

Parent admission and release use the original BOOTTIME deadline. The parent
queue retains a complete zeroizing frame under actual EAGAIN; cancellation clears
it before further IPC. An unexpected partial write or other write error disarms
the input while retaining faulted occupied Child/pidfd ownership. A later control
byte cannot be appended to an uncertain response frame. This defensive error
branch is reviewed source, without partial-write injection in the selection.
Local snapshots and retirement retain their existing nonblocking ownership role.

The final frozen host suite passes40 tests, with one helper entry ignored in the
parent suite and actually invoked by its owning setup regression. Exact parent
queue responses cross real pipes into the command decoder for two successive
prompts. Backpressure/cancellation, calls begun after expiry, wrong worker/order,
duplicates, terminal states, response bounds and malformed record fields pass.
Codec controls include every split of a mixed command stream, maximum body,
NUL positions, malformed headers, sticky failure and real pipe fragmentation/EOF.
The earlier worker conversation and actual subprocess setup controls also pass.
The first full GREEN stops at Clippy's byte-char-slices warning before tests;
that failed selection is retained. The fresh selection changes only that spelling.

Root independently checks all20 frozen bindings and the source review's final
hashes, retains host test and ARM64 fixture ELFs, observes actual container wait0
and confirms fresh exact-container absence on the same Docker daemon. The two
delegated Python drivers parse as syntax only and select PKSCOPE3; they retain
their existing synthetic cancel/no-workload scopes and refuse prompt events.
The new AArch64 fixture is compiled, not executed.

The tests exercise the exact PromptQueue, Decoder/Batch and worker Conversation
components. They do not construct ScopeChild/Attempt, run either full forwarding
loop, exercise actual worker/adopted waits or establish empty cgroup closure.
Calls begun after expiry are refused; no atomic deadline cutoff across syscalls
or real suspend is measured. Older ARM scope/containment evidence keeps its exact
earlier source and format. Actual owner forwarding, original PAM with effective
retained faillock, broker/actor one-shot receipts, exact-object unlock, synced
durable clear and normal Messages reopening remain gates. The crate stays
isolated-feature-only. Matched Google policy, secure frames and physical hardware
remain unqualified. No phone was accessed.
