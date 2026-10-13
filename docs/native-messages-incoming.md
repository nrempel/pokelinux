# Synthetic incoming Messages composition

The separately built incoming fixture connects actual frozen messaging V3 callbacks
on an ARM64 Linux Binder kernel to the existing supervised Messages storage child.
That child commits each raw occurrence through `ReceiptOwner` and immediately
reconciles `Inbox` while no app window exists. A later installed native client
reads the exact committed IDs through the existing current lifecycle authority.
The normal installed `messages-service` still opens local history only: it has no
Binder dependency, backend selector, registration, incoming ACK or send operation.
This is authored synthetic integration using genuine retained Android runtime
libraries, not a vendor modem or physical-device qualification.

## Source and storage ownership

The fixture peer is independently generated public V3 NDK code with two slots,
an immutable explicit generation and one registration attempt per slot. It
retains generated response/indication callbacks. The Rust storage child verifies
remote V3 version/hash, exact synthetic generation and kernel callback UID1000;
it retains the probe, remote endpoints, callback objects and actual death
recipient. A synthetic registration barrier confirms the fixture's state.
One-way transaction return alone establishes IPC progress, not vendor attachment,
delivery, acknowledgement or producer drain.

Before any Binder initialization, the sole child exclusively creates a private
`incoming-attempt-<generation>` marker, writes its exact bytes, fsyncs the file
and fsyncs the raw directory. Existing valid evidence means history-only startup;
partial or invalid evidence means unavailable. At most eight attempt markers may
exist; uncertainty is never evicted or refunded. Child recovery discards incoming
arguments. Whole-service and actual systemd restart with the same generation also
open history without another registration attempt. A local UUID, a stopped child
or its successful reap does not authorize another remote attachment.

Each attachment gets a typed `ReceiptOwner` source incarnation and slot. Every
callback receives a checked monotonic occurrence and its original absolute
BOOTTIME deadline. Raw admission gives equal PDUs distinct receipt IDs. The child
owns both SQLite databases; the lifecycle actor owns the process, pidfd and
publication authority and never opens either database. ACK permission remains
denied. No fixture registration, commit or completion creates an ACK/replay promise.

## Bounded receiving and current reads

Callbacks retain at most512 bytes per payload and enqueue at most eight arrivals
plus one active occurrence. They copy bounded memory and send nonblocking start
metadata; they perform no SQLite or modem command. The512-byte limit bounds
callback copies and retained data. Generated Binder decoding may allocate before
the callback; this increment does not qualify a decoder allocation bound. The
sequence mutex spans only bounded copy and nonblocking IPC work, but waiting for
a descheduled owner is not bounded. Callback completion itself is therefore not
qualified as bounded. The parent knows an occurrence only after Start metadata
is emitted; an admission blocked before that emission is not yet supervised as
an active occurrence. Any late Start still carries the original callback ingress
deadline and is refused after expiry. These limits are separate from the tested
responsive lifecycle actor and actual child reaping.

The actor polls source packets independently of pending UI reads. Start, raw
commit and completion bind the exact worker incarnation, remote generation,
slot/source incarnation, occurrence, original one-second deadline and committed
receipt ID. Late start never renews the deadline. The parent checks every active
deadline while no window exists. A sticky failure latch is independent of both
arrival queue and metadata socket capacity; shutdown of the private IPC boundary
makes failure visible even if its failure packet cannot be sent. Overflow,
unsupported callback obligations, oversized payload, source death, invalid/stale
association, IPC loss, database failure and expired deadlines invalidate receiving.
The supervisor retains occupied ownership until actual child reaping.

Clean app close leaves the source and child active. Policy lock, suspension and
background denial fence data release while source projection continues. The
unchanged read stamp still binds current kernel principal identity, launched
pidfd generation, policy/lifecycle revisions, storage incarnation and deadline
at every nonblocking send attempt. Closing an app during a held read currently
stops the shared provider and therefore explicitly fails the source. Its later
local recovery opens history only; this increment does not promise continuous
receiving across held-read cancellation.

A raw commit followed by projection loss or completion loss survives as the exact
existing receipt ID. Restart reconciles those IDs without recreating an arrival.
The frozen `f733d3b` source exposed latest32 only: its33rd raw occurrence committed
before terminal projection refusal. That evidence remains historical. The subsequent
[complete enumeration](native-messages-enumeration.md) removes this33rd-occurrence
limit with owner-bound fixed-upper pages and eight-import turns; its separately
frozen actor/source selection qualifies all committed IDs over64/67-key fixtures.
Raw4096/inbox capacity, retention and cold-backlog deadlines remain production gates.

## Authenticated shutdown completion

The installed `messages-stop` now authenticates the root socket peer and exact
reply PID, obtains that socket's kernel-origin pidfd before sending the existing
Stop operation, then waits after ACK for actual service exit within one original
three-second BOOTTIME deadline. A small [kernel identity boundary](../native-kernel-identity/README.md)
contains the fixed `SO_PEERPIDFD` ABI call because pinned rustix1.1.5 lacks it;
`native-messages` remains unsafe-forbidden. The wrapper adopts only a descriptor
transferred on successful syscall return, rejects wrong length/negative fd/missing
atomic CLOEXEC and drops owned descriptors on successful-result rejection.
Syscall failure may expose an uninstalled prospective fd number; it never adopts
or closes that number. Unsupported platforms fail explicitly without numeric-PID
fallback. ACK and process exit establish no remote cleanup or healthy policy seal.

An executed old-helper RED used the actual root socket's pidfd and a real held
policy storage child: the helper returned0 while the service remained alive.
A second actual systemd RED recorded Main `EXIT_CODE=killed`, `EXIT_STATUS=TERM`
after the old helper ACK, despite `SERVICE_RESULT=success`. The green checks
actual helper waiting and Main `exited/0`, then separately verifies a matching
private policy seal epoch/hash and policy-validating fresh history startup.
The local service's stop/snapshot actor remains responsive. Kernel origin/peer
exit and ENOTSOCK tests are actual kernel execution; malformed reply-shape cleanup
uses injected shape with a real owned descriptor and is recorded separately.

## Executed evidence

The strict intermediate raw-only RED used real generated Binder callbacks and
committed two equal payloads as distinct raw receipts while no window existed.
It found zero projected records before any read. An actual UID1000 fixed desktop
launch and UID11010 native client then demonstrated default denial and, after
fresh trusted authorization, read both exact receipt IDs. That executed failure
is retained separately from the arrival-triggered implementation.

The current green composes the same actual boundaries and covers clean close,
closed-window arrivals, two slots, status/malformed exclusions, lock/suspend/
background denial, held-read failure, child and whole-service history recovery,
an independently fresh synthetic successor, uncertain held registration, actual
peer death,513-byte refusal, unsupported ACK obligation, actual external SQLite
locks, callback overflow, SIGSTOP ingress deadline, loss after projection, actual
start-send EAGAIN and terminal33rd-occurrence refusal. An actual systemd restart
checks durable source uncertainty and exact history without re-registration.

Fresh successors have their own actual peer and manager processes, generation
and private Binderfs context. The harness waits/reaps the old storage child,
peer and manager before explicitly removing the old test device. That is fixture
reset evidence, not vendor producer cleanup. Early lookup, contention and fixture
runtime-directory failures remain retained unselected diagnostics. The earlier
callback-only scaffold and `ac57dd6` installed desktop/UI freeze remain historical.

[Clean provenance](evidence/native-messages-incoming-provenance.json) records exact
selected source, generated code, ELF and proof hashes without publishing retained
Android runtime bytes or raw private logs. The VM has no NIC, phone access, host
Binder/adb/USB exposure or vendor execution. Existing component and installed
service/lifecycle regression results are recorded separately. Trusted production
session ingress, notifications, wake behavior, complete raw enumeration, retention,
production modem attachment and physical-device/carrier receive remain open.
