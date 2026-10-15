# Native historical registration-wire owner

The native TransportSession now publishes a complete, original-qualified raw
registration indication as bounded historical wire fields. A real generic ARM
kernel Binder test first receives the exact37-byte indication and fails at the
existing `Unsupported` result. The repaired owner passes the same frozen test,
peer ELF and outer oracle, including history replacement, refusals and cancellation.
All52 host library tests and strict checks pass; six ARM binaries compile.
This checkpoint connects the decoder to its owner. The later
[installed slot1 tracer](native-radio-registration-installed.md) separately
qualifies typed IPC/status and packaged slot selection with59 host tests.
Semantic IMS registration remains unfinished.

## Production change

`RadioOwner` and `TransportSession` require an explicit immutable selected slot.
The owner accepts only `IndicationKind::Raw`, recovered message code3012 and that
exact slot. It decodes the entire supplied array with the existing
[native-ims decoder](native-ims-registration.md), copies at most1000 URI bytes
and256 reason bytes, then reacquires the existing shared gate. Same owner
allocation, current SessionKey, active phase, absolute CLOCK_BOOTTIME deadline,
base revision and private preparation token must still match after decoding.
Only then does one revision bump atomically replace the observation.

The latest observation is one enum: raw response or registration wire fields,
with its own commit revision. No older raw response takes precedence after a
registration commit. Requests retain a separate uncertain receipt and cannot
relabel a callback with a newer revision. Raw response metadata retains its
existing rules; registration slot/complete-array rules do not apply to responses.

Unknown numeric values and exact NUL/non-UTF8 URI/reason bytes remain data.
Absent extensions remain `None`. No registration/RAT/features, URI identity,
carrier acceptance or call state is inferred. Invalid selected payloads return
`InvalidIndication` without partial fields or revision/history changes. Other
slots, IDs and every fragment return `Unsupported`. Invalidating the owner
withdraws its observation; independently retained Arc snapshots remain history.
PreparedEvent Drop releases its preparation slot on every refusal.

The [original-reference qualification](native-ims-registration-routing.md) remains
separate: original adapter dispatch, captured listener, class ID association and
null-channel parser observations, without full helper/channel semantics. The
ARM peer and all payloads in this checkpoint are authored public fixtures.

## Actual red-green evidence

The immutable API baseline already retains existing raw responses in the latest
observation enum and requires an explicit slot; it still returns `Unsupported`
for every indication. Existing45 host tests pass before the new publication
requirement runs. The ordinary ARM client performs one real Binder registration,
joins that worker, publishes the existing raw07 callback and records its revision.
A separate public peer then submits raw3012/slot0/full37 bytes. The client checks
the actual PreparedEvent kind, route and every byte before calling publication.

The baseline records `Err(Unsupported)` with unchanged phase, revision and Arc
history. It then performs normal Stop, joins any owned workers and drops its
retained watch before returning the dedicated requirement failure. Actual client
wait1 and bounded pipe EOF both precede the original Rust2s cleanup cutoff.
The peer stays alive through those observations, then its owner signals the exact
pidfd and waits SIGKILL9. Full guest/outer/container/input closure qualifies the
behavioral RED; a submission acknowledgement alone never proves owner receipt.

After repair, the identical test runs:

- First complete typed publication commits revision4.
- An authored raw request remains outcome-unknown; its receipt alone cannot
  republish history. Its actual raw response replaces typed history at revision5.
- A second complete typed indication replaces that raw history at revision6.
- Wrong slot, adjacent ID, fragment, truncated header and trailing byte refuse
  publication while preserving current phase, history and revision.
- A third exact consumed valid indication is stopped before publication. Stop
  clears history and advances to revision7; `Failed(Cancelled)` cannot advance it.
- Normal client wait0/EOF, peer wait9 and all outer closure checks pass. One
  registration and zero clears remain; no retry or vendor cleanup is inferred.

Seven focused owner gate tests additionally cover selected slot1 with opaque
bytes, unsupported/malformed retention, expiry without timer delivery,
cancellation/service death, wrong owner/replacement and queue loss. Against the
unchanged raw-only baseline,47 tests pass and5 assertion groups fail at
`Unsupported`; repair passes all52. Cargo's failed-test exit is101. The private
component wrapper incorrectly expected1 and remains recorded as failed; Root
independently verifies the exact assertion failures and complete owned closure.
No test output or receipt is rewritten to conceal that classifier mistake.

The first repaired ARM client passes its requirements but the guest validator
incorrectly expects a caller PID in a diagnostic emitted while processing a
one-way request. Actual caller PID is0, matching the
[AOSP Binder contract](https://android.googlesource.com/platform/frameworks/native/+/refs/tags/aml_ads_331814200/libs/binder/include/binder/IPCThreadState.h).
That failed qualification is preserved. A fresh baseline and repair replay with
one corrected exact-zero expectation, identical ELF bytes and the same corrected
oracle qualify RED and GREEN. The peer's diagnostic label `target_pid` reports
current incoming caller metadata; it is not destination identity.

[Bounded provenance](evidence/native-radio-registration-owner-provenance.json)
pins the source snapshots, unchanged fixture/peer, actual compiler-selected ELFs,
runtime results and closure receipts. Raw logs and private original inputs remain
outside Git. This is actual generic ARM kernel/Binder evidence, without new
original vendor execution, actual suspend, visual or physical-phone qualification.

## Installed composition and remaining gates

Required profile `SLOT_ID` now reaches the service, worker `--slot` and immutable
TransportSession selector. The current observation codec carries a bounded typed
Frame, and status exposes one tagged `last_observation`. The separately executed
[installed slot1 tracer](native-radio-registration-installed.md) receives and
accepts registration R4, demonstrates stale RAW R3 history as a genuine RED, then
passes after one service history assignment. Typed R4 survives unavailable R5;
native worker wait9, Main0, cgroup closure and durable refusal78 also pass.

Terminal phase selection precedes history projection, so retained typed history
cannot suppress a Failed frame. The worker now explicitly skips
`InvalidIndication` while retaining transport/history. This is source policy;
the installed malformed-indication continuation scenario remains unexecuted.
The earlier owner refusal controls above do not qualify that worker path.

Full original helper/channel dispatch, producer identity, reset/cleanup
acknowledgement, safe successor registration, semantic IMS/calls/SMS/media,
installed pressure/principal negatives and matched Pixel hardware remain open.
No physical phone was accessed.
