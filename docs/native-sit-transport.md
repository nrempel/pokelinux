# Factory SIT transport findings

The later [native owned descriptor primitive](native-sit-owned-transport.md)
implements one-shot writes and owned raw reads using these findings while
keeping conservative terminal uncertainty. Its real descriptor tests and
process integration are separate from the static/original evidence below.

The [native CPIF local policy](native-sit-cpif-policy.md) now implements bounded
readiness/idle handling with local syntax observations; source/record and device
admission remain unavailable.

These are author-reasoned static contracts from selected matched factory methods,
with independent method/assembly/ELF review. The later
[retained CPIF and construction checkpoint](native-cpif-receive-boundary.md) adds
exact-module read/poll, ordinary static owner creation and default path storage.
These static sessions execute no vendor methods. A separate memory-only original
comparison is recorded in [receive ownership](native-sit-received-chunk.md). They guide
the Rust backend; the separate [owned attempt fixture](native-sit-attempt.md)
qualifies actual host/generic ARM process composition through Unix records.
Neither evidence class establishes a live modem transport.

## Descriptor and ordinary caller

`IoChannel::Open` consumes an already stored path, opens with O_RDWR|O_CLOEXEC,
and reports a lower32-bit local status0 or−1. No O_NONBLOCK flag is selected.
`Read` returns the first nonnegative count, including short/zero. It retries
EINTR/EAGAIN indefinitely with10ms sleeps. It adds no packet framing and does
not establish validation of a negative supplied count.

`Write` accumulates positive short writes. Full local completion returns status0,
not the byte count. Terminal−1 loses any earlier progress; zero-write progress
has no guard. EINTR/EAGAIN retry is unbounded. Descriptor fields are refreshed
between attempts; no lock/source-generation contract appears here. `Close`
ignores the imported close result, stores−1 and returns0. These status words do
not prove a recovered C++ return declaration, remote acceptance or cleanup.

The actual `RadioControlMessageMgr::Run` caller waits in select with no timeout,
checks modem status, clears a retained65,568-byte buffer and performs one raw
read. Each positive chunk passes the same pointer/count synchronously through
owner vtable slot+0xa0. Run establishes no local framing, reassembly or owned
queue copy. A later static vtable binding identifies a receiver and copy contract below;
additional ordinary construction analysis now binds the supplied wrapper owner
([scope](native-cpif-receive-boundary.md#ordinary-factory-path)). Actual runtime
initialization, factory selection and channel custody remain unobserved. The next
iteration can overwrite the buffer.

The caller **does** reopen after zero or terminal-negative read: close,20ms sleep,
then open with the result ignored. Negative modem status sleeps3s. `Start` can
proceed to thread creation/start after an initial open failure, so status0 is not
usable I/O readiness. Public manager Read delegates; Write calls the opaque
DumpData helper and delegates despite its nonzero result. DumpData is not proven
side-effect-free, and unchanged callback bytes cannot be inferred here.

These bodies supply no proven select wakeup, thread join, source fence,
auto-reopen cleanup acknowledgement or safe native retry. The matched library's
existing policy is a reference to understand, not a policy to reproduce blindly.

## Candidate receive owner and copying

The manager constructor stores its supplied owner pointer without establishing
its dynamic type. The `RilContextWrapper` vtable's slot+0xa0 binds to the static
`ProcessModemData` candidate. That body requires a nonnull source and nonzero
unsigned count, allocates a24-byte `ModemData`, invokes the imported constructor
with the supplied signed count and false flag, and calls `SendMessage`. A negative
result deletes the object; the callback itself returns0. This static binding
alone establishes neither the ordinary creation chain nor delivery. The later
construction checkpoint closes the static chain, with runtime/channel gates retained.

The separately bound protocol provider's C1/C2 copies exactly the positive supplied
count when the source is nonnull, including when the boolean is false. It does
not validate the inner header, token or declared length. Null/zero/negative input
leaves empty payload state. The named nondeleting D1/D2 releases the original
payload through sized scalar delete, clears pointer/count/flag and retains the
receiver. These allocator/layout facts guide original test cleanup; Rust imports
no C++ ABI. Original memory-only tests and the native owned copy are recorded
[separately](native-sit-received-chunk.md).

The candidate `SendMessage` performs lookup/removal and routing before enqueue.
The AArch64 hidden aggregate result and adjacent copied route words require
assembly interpretation; the decompiler loses that branch. A queue error can
still leave return0. Neither callback0 nor local memory copying proves queue
admission, delivery, source identity or APN authority. The manager destructor
closes its channel before opaque thread Stop/delete; join and concurrent ordering
remain unproven. `DumpDataStatic` remains opaque.

## Scalar classification is not complete-record admission

Four `ModemData` getters delegate through a stack adapter. Four corresponding
`ProtocolBaseAdapter` bodies establish pointer guards, then byte0 as the type;
solicited/unsolicited compare1/2 through the bound virtual GetType slot. GetToken
reads little-endian bytes6–9 when type is0 or1; absent pointers or other types
return `0xffffffff`. None of these selected bodies checks the stored payload
length, inner declared length or complete-record size before those reads.

These are static facts, not getter execution on short buffers. Native code keeps
its stricter complete144-byte response admission and does not reproduce unchecked
reads. Copying short, opaque or coalesced bytes grants no validity. The next
unexamined length/message-ID getters and the transport's complete-record premise
remain explicit; no receive assembler is inferred from these bodies.

## Public driver evidence stays separate

The earlier pinned [public s5400 IPC driver](https://gitlab.com/grapheneos/kernel_pixel_6.6/-/blob/66529eed85e8f3ccbda7f74911b125e7d6df7efc/private/google-modules/radio/samsung/s5400/ipc_io_device.c)
returns zero for idle, can preserve unread packet tails, and can fail a write
after earlier fragments. Its shared queues and12-byte outer framing are distinct
from the inner RCM header/token. This public revision is not authenticated to the
factory KMI. Its idle-zero behavior cannot simply be combined with the factory
caller's reopen policy as one qualified runtime contract.

A separate [official public link-device revision](https://android.googlesource.com/kernel/google-modules/radio/samsung/s5400/+/42c515afbe75b28ee10a756e69b30ff89783b9ff/link_device.c#2014)
returns immediately for SIT initialization and sets `terminate_comm` to NULL.
A generic release hook therefore establishes neither SIT remote cleanup nor a
required modem INIT_END exchange. Keep Google's matched kernel/modules/firmware;
these public sources do not license replacing their matched set.

The ordinary static owner creation chain is now qualified separately. Next
qualify where complete records are established before classification, including
the unresolved length/message-ID getters and channel custody. Selected exact
queue producers are now reviewed, but supply no complete inner-record premise.
The owned-copy integration proves only the
selected Unix-record path. Broader transport rules, queue limits, original
deadlines and known-source loss still need their own qualification. Keep one
descriptor generation and terminal uncertainty. Reaping, local status0 and
factory reopen grant no retry, provider/APN ownership, channel/subscription
mapping or physical modem/carrier readiness.

[Provenance](evidence/native-sit-transport-provenance.json) contains hashes only.
Raw snapshots, decompilation, method inventories and proprietary binaries stay
private. An earlier filename-collision session is preserved and excluded from
the selected IoChannel response artifacts; the manager repeat additionally
captures exact live JSON-RPC wire records.
