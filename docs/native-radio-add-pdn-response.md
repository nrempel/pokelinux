# One-shot add-PDN response observation

The native radio owner can retain one immutable raw response observation for an
already-admitted add-PDN request 105. `RequestReceipt::response` holds copied
payload bytes, the original opaque owner/session/request identities, exact
serial/code/slot, uninterpreted vendor error, real receiver-admission sequence and
BOOTTIME, and the actual publication revision. This history never resolves
`OutcomeUnknown`, releases the occupied request slot, authorizes retry, or grants
APN, IMS or network readiness.

The receiver mints its sequence/time under the existing owner gate when a
validated callback is admitted to the actual queue. The first dispatch permission
captures that gate's current sequence. A queued callback admitted before that
permission cannot become the request response when the owner later drains it.
This is local admission ordering; it is not a kernel arrival timestamp or proof
that a remote producer caused the callback in response to this request.

Publication requires the same owner allocation, session and request, active
session, unexpired original request/session deadlines, exact 105 route and selected
slot, complete raw payload of at most 4096 bytes with matching nonnegative length,
and no previous attribution or intervening cancellation, Stop, death or
replacement. Parsing and bounded owned copying occur outside the mutex. The
existing final commit gate and raw diagnostic history fencing remain in force.
A rejected attribution may still be an ordinary historical raw diagnostic. A
later duplicate cannot replace the first attributed `Arc` or its bytes/revision;
subsequent cancellation/death retain that earlier history without live authority.

## Evidence boundaries

Original-code execution previously showed that its pending-request dispatcher
matches and removes by serial, including controlled mismatched metadata. See
[original response dispatch](shannon-native-recovery.md) and its
[bounded provenance](evidence/response-dispatch-provenance.json). Exact route,
length, local ingress ordering and one-shot attribution here are deliberately
stricter authored policy, rather than recovered vendor protocol semantics. Vendor
error meanings, sender flags, producer/reset identity and modem acceptance remain
unresolved.

The independent unit consumer was frozen before implementation. A genuine
missing-field compiler RED was followed by a safe-empty API runtime RED: seven
required positive observations failed and two negative controls passed. The same
nine tests then pass through real parcel decoding, callback queue, dispatch
permission and owner commit. They cover original keys/owned bytes, prepermission
queueing, wrong serial/route/length/size, cancellation/expiry/Stop/death, duplicates,
foreign owner/old session, segmented/other operations and zero-error/empty payload
as history only. The final source-complete host suite passes 79 active tests (68 library plus 11
existing integration), with 9 explicitly Binder-driver-required tests ignored.
Formatting, strict Clippy and Release pass separately, as do fixture formatting
and strict Clippy. These source additions do not relabel older suite counts.

The same new ARM consumer and independent peer then run in two fresh generic
ARM64 Arch Linux 7.1.6 guests using actual kernel Binder, UID 1000, a private Binderfs
device and no NIC. This is an additional post-implementation differential check:
a controlled safe-empty attribution baseline exits 101; current production exits 0.
It is not relabeled as a preimplementation native failure. Both first qualify
one registration, the full original 663-byte direct preflight, fixture-only
capture reset, and exactly one real owned code 105/slot 1 submission. Seven authored
synchronous callbacks acknowledge actual receiver completion: a response queued
before dispatch permission, four wrong serial/slot/code/length controls, one fresh
matching response and one duplicate. The matching payload/error are copied
history only; the slot stays occupied and another prepare is refused.

The independent peer and manager binaries, frozen test and owned
framework are identical between the two differential runs; only request, owner
and receiver production bodies differ. Actual peer death and subsequent Stop
retain uncertainty. Each guest has four direct child waits and four merged PIPE
EOFs; test wait/EOF precede the original 2s Rust cleanup cutoff. Original 10s
setup / 20s work / 30s session / 45s guest test and 540s / 600s outer bounds are retained.
Each actual foreground container CLI exits 0, its exact CID is absent afterward,
the daemon is unchanged, and locked source/backing inputs are unchanged. The
outer console is a closed ordinary output file, not an invented PIPE EOF.
See the [bounded provenance](evidence/native-radio-add-pdn-response-provenance.json).

The ordinary installed radio service/status schema is unchanged; attributed
request history is currently a library snapshot, separate from its latest raw or
registration-wire diagnostic. This adds no durable command permission/ledger,
request retirement, subsequent operation, APN ownership/acquisition, semantic
registration, vendor cleanup/reset, carrier, physical-device or UI qualification.
The backend still uses the Samsung radioExternal provider; replacement of that
modem-facing vendor userspace remains a separate final-architecture gate. No
physical phone was accessed.
