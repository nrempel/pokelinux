# Retained CPIF receive boundary

The exact retained4K CPIF module now supplies static read/readiness evidence,
separate from public source comparisons and the native Unix fixtures. A fresh
read-only single-file extraction binds it to the authenticated selected
`vendor_dlkm` image. No module, modem or phone was executed. The
[owned Rust transport](native-sit-owned-transport.md) remains a raw byte primitive;
complete-record and selected-channel admission are still open.

## Input identity and evidence

The603,640-byte module hashes to
`95fffae6b7e3530cb6bc0622283bd6a360e8912ebc48aa9a529ca8fea8d634fe`.
A pinned debugfs1.47.0 extracts exactly `/lib/modules/cpif.ko`, inode82, from
the43,810,816-byte selected ext4 image. Version/stat/dump have three actual direct
waits0 and both EOFs before their original cutoffs. All13 existing inputs remain
unchanged. The retained compressed extent independently reproduces the selected
image's size, hash and ZIP CRC with deflate EOF. Whole factory-stream
authentication is inherited from the earlier authenticated selection; no fresh
whole-archive download or AVB verification is claimed.

This is the4K vendor_dlkm variant, not the distinct16K vendor_boot module.
The module records `scmversion=g06da1f8c219a`, DDK construction and build16254410.
The producing public project/full revision/build manifest remains unresolved.
Public s5400 source similarity does not establish that identity or matched KMI.
Keep the selected Google kernel/modules/firmware together.

## Exact read and readiness bodies

The offline REA session selects only `ipc_read` and `ipc_poll`. Original ELF
symbols/relocations bind their676/408-byte spans; ET_REL section mapping is
qualified independently. Raw and normalized responses match the closed
snapshot and actual wire records. Six RPC responses, actual container wait0,
both EOFs, same daemon, exact CID absence and all21 source/tool pins qualify.

Read and poll use the same queue on the object obtained from the file's private
pointer. Read waits when that queue is empty, with an initial25 timeout ticks,
then dequeues one skb. Still empty returns zero. A successful copy returns the
lesser of the requested count and that skb's remaining length. An unread suffix
is placed back at the queue head; the call does not join a second skb. A failed
copy discards that skb and returns−14. The selected body provides no O_NONBLOCK
branch. These are compiled static paths, not observed device timings or EOF,
cleanup, exclusive custody, packet-start or complete-RCM guarantees.

Ghidra includes572 of the676 read bytes. Independent original-word/relocation
review covers the omitted104-byte jump-label trace path, including its
alternative-instruction reference. That path preserves the copied-count/error
result and adds no queue copy/dequeue/requeue. No live kernel patching or opaque
trace helper behavior was executed. The poll body covers all408 bytes.

Poll can register the supplied wait callback when empty. Its selected numeric
state3/4 branch returns mask0x41 only for a nonempty queue; state1/2/5/9 and an
owner-field condition can return0x10; state10 has a separate owner-bit condition
for0x10. Null private state returns8; other selected paths return0. Public enum
names are not imported into this exact-module result. Poll reserves neither a
packet nor its start. Cross-open object sharing and exclusive channel custody
require their own open/registration and deployment evidence.

## Exact queue producer

A second bounded static session selects `rx_fmt_ipc`880 bytes and
`queue_skb_to_iod`472 bytes. Five actual RPC responses, wait0/both EOFs,
exact CID absence, closed-snapshot agreement and27 unchanged source/tool pins
qualify. Independent review rehashes15 inputs and manually accounts for the
queue helper's164-byte trace/alternative tail omitted from its308-byte Ghidra
body. All1,352 selected original bytes are accounted for; callbacks and live
patching remain unexecuted.

The header-present branch invokes two unresolved protocol callbacks. Numeric
selector1 strips12 outer bytes; when the callback's low control bit is set,
it groups payloads by a six-bit outer index and concatenates on outer bit14.
Selector0 strips the callback-provided length and groups by a seven-bit index;
its flush path splices separate skbs without concatenation. Header-free and
single-frame paths use the ordinary queue helper. Public enum names and actual
selected protocol/channel configuration are not established by these bodies.

Both producer paths target object+0x138 and wake object+0x38, matching consumer
layout. The producer's skb-attached object and consumer's file-private object
are not yet bound to the same runtime pointer. Neither path validates complete
inner RCM records, fragment sequencing/expected total or a65,568-byte maximum.
The concatenation uses32-bit length accumulation without an explicit overflow
bound in the selected body. This is a static contract limit, not an executed
memory-error claim.

The ordinary helper can drop a new skb under a nonempty-queue500-tick condition,
yet still wake and return its original length. Its watermark is not a hard
queue cap. Multipart completion bypasses that helper. Local return length,
outer gathering and a shared layout therefore grant neither delivery nor
complete/fresh inner-record authority.

## Ordinary factory path

Additional matched-library static construction evidence connects `RIL_Init`,
application initialization, constructed wrappers, `OnCreate` and `OnStart` to
the same wrapper owner/channel passed into the manager constructor. This closes
the earlier static candidate-owner creation gap. It does not observe normal
runtime initialization or actual channel use.

Initialization builds `/dev/umts_ipc0` and `/dev/umts_ipc1` parameter records.
OnCreate calls the selected supplied/default factory with the wrapper and stored
path, then stores the returned channel. The additional default-factory body
returns null for a null context; otherwise it allocates40 bytes and forwards
the supplied path to the channel constructor. That constructor starts with fd−1,
a zeroed32-byte name and a separate zero terminator, then copies at most32 path
bytes. Both unique bodies cover their complete68/44-byte ELF spans. Five actual
RPC responses, wait0, both EOFs, exact CID absence, closed-snapshot agreement and
all72 selected inputs qualify; no constructor/vendor code was executed.

The application's factory selector starts null, but an opaque configuration
loader intervenes before initialization. The supplied/default dispatch remains
conditional. These path facts do not establish matched factory DT selection,
subscription mapping, exclusive custody, readiness or provider/APN authority.

## Next native boundary

Implement one bounded CPIF-specific readiness/idle-read policy while keeping
authority-bearing source admission and device acquisition unavailable. The
remaining custody question is what binds the chosen queue to this worker/request
generation, including queued-before-open data and competing readers. A short
read can end a retained suffix; an inner length field,
opaque byte copy or Unix SEQPACKET fixture cannot supply missing record-start
and boundary evidence. Preserve one descriptor generation, the original
BOOTTIME deadline and terminal uncertainty. Never copy the original unbounded
retry/reopen policy into native recovery.

The later [local I/O policy](native-sit-cpif-policy.md) implements readiness/idle
behavior and separately tests natural worker termination. A production receive
vertical must observe natural worker termination before
cancellation, withdraw current state, preserve only previously committed history
and retain durable successor refusal. These are additional integration checks,
not a claim that the current attempt library already loses committed history.
Source/KMI/DT and custody, provider/APN/IMS, installed policy and physical
modem/carrier testing remain explicit gates. Zero phone models are qualified.

[Hash-only provenance](evidence/native-cpif-receive-boundary-provenance.json)
contains authored metadata. Raw decompilation, module bytes, extracted images
and private tool logs remain outside the repository.
