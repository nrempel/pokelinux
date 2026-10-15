# Native owned SIT descriptor I/O

`native-sit-transport` now performs native Rust descriptor I/O beneath the
existing attempt and process owner. The authored process fixture moves its
peer-checked handle into this primitive, sends the exact983-byte request,
receives owned raw bytes, then applies its existing strict144-byte admission
and14-field report oracle. This advances the native backend; it is not a
production CPIF adapter or complete cellular/IMS service.

The [exact retained CPIF checkpoint](native-cpif-receive-boundary.md) separately
qualifies static read/poll and path construction; it does not upgrade these Unix
fixture runs into matched-driver execution.

## Local contract

A transport consumes one `OwnedFd` and the original absolute BOOTTIME deadline.
Its API exposes no descriptor borrow, cloning, replacement, reopen, reset or
retry. Rust ownership of this handle cannot prove absence of other OS aliases,
source identity, channel/subscription mapping, readiness or provider authority.
The constructor deliberately establishes none of those properties.

Dispatch checks the deadline, records `OutcomeUnknown` immediately before one
actual `rustix::io::write`, then rechecks the same deadline. Only the complete983
count succeeds. Zero, short or failed writes stop the local generation; there
is no continuation, EINTR/EAGAIN loop or implicit uncertain-request retry.

Each read requires prior dispatch, performs one actual raw read into65,568
reusable bytes, rechecks the deadline, copies the positive slice into the
existing immutable `ReceivedChunk`, and checks the deadline again before
exposing it. The bound is native ingress policy, not a protocol maximum.
There is no parser, packet assembly or record splitting here. A short16-byte
chunk and coalesced288-byte chunk remain opaque. A local zero read stops this
generation conservatively; it does not establish CPIF EOF or modem cleanup.
Read/copy errors are terminal. The first terminal reason and dispatch uncertainty
remain recorded after the local handle is dropped.

Methods and descriptor close may block. They run inside the owned worker;
Main's attempt snapshots and Stop use the separate process supervisor.
A transport snapshot is only copied historical state, and cannot interrupt its
own blocked read or attest that an outside process is still alive. Main retains
source admission, cancellation and the durable uncertainty journal.

## Compiled RED and component GREEN

The initial attempt to compile stopped at a Clippy test-helper API warning; a
second stopped at test formatting. Both are preserved setup failures, excluded
from RED. After those prerequisites were corrected, the fixed ten tests compiled
and passed strict linting: two ownership/expired-before-dispatch controls passed
and eight failed specifically at `Unsupported` dispatch. A dispatch-only source
repair then passed three and exposed seven missing-read failures. Only
`native-sit-transport/src/lib.rs` changed for either repair; tests, fixtures,
locks and all dependencies stayed identical.

The final source passes all ten real Linux `CLOEXEC|NONBLOCK` Unix SEQPACKET
tests on host and ARM64 user-QEMU. They cover exact one-shot dispatch, refusal
of a second dispatch, all65,568 copied bytes surviving another read and transport
drop, short/coalesced/opaque input, pre-dispatch refusal preserving queued bytes,
actual peer-close zero read and actual empty-descriptor EAGAIN terminal refusal.
Queued-byte preservation explicitly proves no response freshness.

Unchanged codec50 and attempt15 tests also pass on both architectures:75
component tests per architecture. ARM user-QEMU executes userspace components
against the host kernel; it is separate from native ARM kernel integration and
from the earlier original Bionic memory-only comparisons. No new original code
or proprietary library was executed for this checkpoint.

## Actual process integration

The source-bound V6 fixture keeps the independent controller and full
request/reply/report/lifecycle assertions unchanged. Its worker now uses the
owned transport for the first request/read, emits only the owned bytes, and
makes one separate second raw read as its deliberate post-response hold.
The peer sends no second record. This is neither a dispatch retry nor response
accumulation. All three schedules require actual worker signal9; natural70 is
not accepted interchangeably. Independent successor-refusal evidence is now
recorded even if the actual worker wait violates that expectation.

Actual host and generic ARM kernel completion/expiry/cancel cases pass. Main preserves the original
BOOTTIME cutoff; expiry delays commit of a timely candidate, while cancellation
rejects it. Each case records100 Main snapshots below200ms, then the controller
independently captures the worker blocked in the real Unix receive. That ordering
does not prove the worker was already blocked during every snapshot. Stop runs
after the blocked-read capture and actually waits signal9 within2s. The
unchanged journal refuses a successor with `AdmissionUncertain`. Native parent
wait0, both PIPE EOFs, later peer EOF, final ECHILD and unchanged input pins pass.
This is additional integration after the component RED/GREEN, not a new
missing-completion RED.

The ARM system-QEMU run executes the reviewed ARM ELF natively under
`7.1.6-1-aarch64-ARCH`, separately from user-QEMU components and Pixel's matched
kernel. Its17 owned waits comprise seven host, two boot, two guest, three native
parents and three native workers; each applicable PIPE EOF and original cutoff
is qualified. Root rehashes125 selected and308 external inputs. Actual QEMU
wait0, guest closure and exact CID absence after the foreground container wait
pass. This generic kernel evidence qualifies no Pixel driver/module KMI.

## Additional real syscall controls

A separate Rust example and fixed Python controller now exercise three
actual OS boundaries against the already implemented source. Host and ARM64
user-QEMU each pass readonly-file EBADF, child-only RLIMIT_FSIZE16 partial write,
and a positive read returned after the original BOOTTIME deadline. File bytes
must equal exactly the request's first16 bytes. Every case retains uncertainty
and terminal refusal, with actual child wait0 and bounded stdout/stderr EOF.

The late-read control observes the owned child's actual blocked read syscall,
matching its descriptor and65,568-byte count before expiry and again before
releasing144 bytes after expiry. It requires ErrorDeadline and no chunk, then
checks peer EOF while the helper and transport value remain alive; a separate
stdin acknowledgement permits exit. No mock result, request retry or exit-time
fd-drop substitute is used. These are direct helper controls, separate from
Main's worker-exit/journal integration and from native ARM kernel evidence.

The initial ARM user-QEMU partial case exits with SIGXFSZ before the target
request write, leaving the target file empty. A bounded guest trace stops in
first-use clock/runtime probing after the file limit. That unqualified helper
prerequisite failure and the original host passes are preserved. The helper now
samples its one original5s deadline before setting the child-only file limit;
all write/prefix/terminal oracles and production source remain unchanged. The
revised helper passes all three controls on host and ARM user-QEMU. No detailed
QEMU cause or new pre-implementation RED is claimed.

## Remaining boundaries

The new direct syscall controls do not yet qualify native ARM kernel or CPIF
partial/error/late-return behavior. Expiry during copying, allocation failure,
Main's handling of natural terminal worker exits and physical suspend/wake
remain open. Main's existing expiry integration holds a timely candidate; the
separate helper now qualifies a late raw-read return on the host kernel.

Complete-record framing and ordinary runtime identity, CPIF/channel/slot binding,
provider/APN acquisition, deactivate/reset and independently acknowledged vendor
cleanup remain open. No process wait or local close clears the uncertainty
journal. Production deployment, modem/IMS/carrier behavior and all physical
Pixel gates remain unqualified. Keep Google's matched kernel/modules/firmware
unchanged. No phone was accessed and zero physical models are qualified.

[Hash-only provenance](evidence/native-sit-owned-transport-provenance.json) keeps
component, host process and ARM kernel evidence separate. Raw private logs
and original inputs are excluded from the public repository.
