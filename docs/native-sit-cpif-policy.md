# Native CPIF local read policy

`native-sit-transport::cpif::CpifReadPolicy` adds actual Rust poll/timer/read
behavior for the selected CPIF contract. It preserves the generic transport's
terminal zero-read policy; this explicit backend instead treats a timely zero
read as idle. Output remains an owned local chunk plus the existing strict
parser's syntax result. Device acquisition, source qualification and production
attempt/deployment admission remain unavailable.

The [exact retained-module findings](native-cpif-receive-boundary.md) motivate
this policy but are separate static evidence. Unix descriptors exercise its
actual OS operations; no CPIF module or modem is executed by these tests.

## Owned policy and deadline

Construction consumes one descriptor, one original absolute BOOTTIME cutoff and
a nonzero finite work limit. It creates one CLOEXEC/NONBLOCK CLOCK_BOOTTIME
`timerfd`, armed once with ABSTIME at that cutoff. The same cutoff enters the
existing one-shot transport. There is no fd accessor, replacement, reopen,
request retry, source-proof flag or backend device-path acquisition API.

Each admitted observe spends one work unit before one poll and at most one raw
read into the65,568-byte ingress buffer. Predispatch, terminal and expired calls
issue no poll/read. The timer joins the poll set so an empty descriptor can wake
at the original deadline. Deadline checks follow poll, read, copy and parsing;
late bytes cannot escape after those calls return. Work exhaustion terminates
on the next observation attempt; zero reads spend work while retaining the same
generation and dispatch uncertainty.

Readability is limited to IN/RDNORM. Other masks, including HUP/ERR/NVAL alongside
readability, terminate the generation. A ready indication reserves neither a
packet nor its start. Read/poll errors are terminal, with no hidden EINTR/EAGAIN
or idle loops. The original generic dispatch still writes once, retaining
OutcomeUnknown across incomplete/error outcomes.

Poll/read/close can still block in the kernel. Timer expiry cannot interrupt an
already blocked read; an owned process supervisor is required for responsive
Main snapshots/Stop and real reaping. BOOTTIME accounts for suspend but this
non-alarm timer does not establish a hardware wake source. Actual suspend,
blocked driver cleanup and Pixel wake behavior remain hardware gates.

## Local syntax is not source admission

Positive bytes are copied into immutable `ReceivedChunk`, then passed through
`BaseSetupResponse::parse` with the supplied expected token. The component keeps
the original complete144-byte subset and returns supported syntax or the parser
refusal alongside unchanged bytes. It never joins short chunks, splits a larger
chunk or interprets zero as EOF/cleanup. Unsupported syntax spends work but
creates no current data observation.

Even a perfect144-byte response can be old queued data or a retained suffix.
This API mints no source/process identity and calls no attempt or deployment
gate. No direct CPIF device acquisition or CPIF-to-authoritative-attempt path is
implemented. The remaining production question is how the selected channel's
queue binds to this worker/request generation, including competing opens/readers
and queued-before-open data. A path, OwnedFd, inner length or token0 cannot supply
that authority. Provider/APN ownership, native IMS and carrier state remain open.

## Compiled behavioral RED and component GREEN

The compiling baseline includes the API, real absolute timer construction and
fixed tests, while observe policy returns Unsupported after its preflight.
Strict formatting and Clippy pass. Two controls pass and nine actual descriptor
cases fail at missing policy; test exit101 is a behavioral RED. Only `cpif.rs`
changes for GREEN; tests, fixtures, locks and dependencies remain identical.

Final host and ARM64 user-QEMU each pass11 policy tests plus unchanged10 raw
transport,50 codec and15 attempt tests:86 per architecture. These include
actual readiness, empty-poll timer expiry, actual ready zero then a positive
read on the same generation, finite repeated-idle work, real peer HUP,
pre-dispatch and late-queued refusal, prequeued supported syntax without a
freshness claim, short/coalesced/malformed/token refusals, no short-chunk
assembly and full65,568-byte retention after another read/drop.

Independent source review led to a precise budget comment and an unconditional
post-copy deadline check before allocation-error selection. Both prior GREEN
and final GREEN remain separately frozen; fixed tests still match the RED.
Root independently verifies selections, logs, ELF machines, actual container
waits/EOF/removal and fresh exact CID absence. ARM user-QEMU is userspace
execution on the host kernel, separate from native ARM kernel proof below.

Unexecuted policy branches remain explicit: readable+HUP/ERR/NVAL combinations,
poll EINTR/error, read races/errors or a late actual read return inside this
policy, copy/parser expiry and allocation failure. Existing generic transport
late-read controls retain their narrower evidence. No physical suspend occurred.

## Natural process exits

A separate source-bound Unix-record fixture now handles two natural terminal
cases through the existing Attempt/Supervisor. Before-commit worker0 makes the
prepared candidate SourceLost with no history. After a live commit, the peer's
actual write-half close causes generic raw ZeroRead and worker70; Main observes
that wait before cancellation, clears current and preserves the exact prior
Arc/source/144-byte record/full report as history.

The RED explicitly adds fixture schedules and diagnostics while retaining
Main's status-discard defect. Both cases already pass withdrawal/history checks,
then Main waits again for its reaped worker, reaches its2s cleanup cutoff and
exits70. This is a discarded-wait RED, not a stale-current or missing-library
implementation RED. The Main-only repair retains the one observed status and
timestamp; cleanup explicitly reuses that receipt rather than claiming a second
actual wait. Tests/controller and semantic oracles stay fixed.

Both final host cases pass Main0,100 post-loss snapshots below200ms, Stop after
reap below200ms, unchanged journal/successor AdmissionUncertain, actual parent
wait/PIPE EOF and ECHILD. The original completion/expiry/cancel controller stays
byte-identical and passes all three separately, requiring worker signal9 and
its original deadlines. Stop after reap is distinct from blocked-worker Stop.

The final combined build contains the exact policy sources, matching shared
dependencies and natural Main/controller sources. It passes strict host/ARM
compilation and all five host schedules. The fixture deliberately still uses
generic WorkerTransport; its synthetic Attempt authority is not conferred on
CPIF observations. These are separate proofs, even when run in one ARM guest.

## Native ARM kernel and remaining gates

A fresh actual system-QEMU epoch under `7.1.6-1-aarch64-ARCH` passes all11
policy tests and, separately, all three original plus two natural process cases.
The policy ELF and generic-record tracer keep separate exact source closures;
the fixture still does not call CpifReadPolicy. Root and independent review
qualify177 selected files,187 external inputs and23 actual waits: seven host,
two boot, four direct guest, five Main and five workers. Natural cleanup reuses
the observed wait rather than adding two fictional waits. Required EOFs,
original cutoffs, five distinct source incarnations, private journal copies,
QEMU0/driver0, same daemon and fresh exact CID absence qualify.

An earlier epoch's individual tests passed but its overall witness failed at
leading whitespace in `/proc` State parsing. That epoch, its conservative
unresolved-handle field and separate fresh exact-CID absence check remain
preserved; it is not a qualified GREEN or behavioral RED. Only the two witness
normalizations change for the fresh successful epoch; Rust, controllers,
artifacts and all test oracles stay identical.

This is a generic ARM kernel, not the matched Google CPIF driver or Pixel.
Preserve user-QEMU, host, static/module and physical evidence separately. The finite
policy slice ends after its local I/O and separate lifecycle qualification;
record/source custody remains unavailable while other smartphone matrix
verticals continue. The next planned slice is an installed native media-output
selector using the existing Rust audio owner and actual PipeWire/WirePlumber,
with normal Qt input, virtual graph/sample-flow evidence and Omarchy theme
switching. Physical audio routes remain separate. No additional static campaign
to assume one skb equals one RCM is needed.

[Hash-only provenance](evidence/native-cpif-policy-provenance.json) binds code,
fixed RED/GREEN and lifecycle evidence. No proprietary inputs, raw decompilation or
private tool logs are published. Zero phone models are hardware-qualified.
