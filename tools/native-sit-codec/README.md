# Original SIT codec comparison helpers

`original-setup-probe.c` is authored test infrastructure for one exact factory
library ABI. It calls the original APN factory, context constructor, inline
builder initialization and complete setup serializer, then the original packet
and context destructors. It contains no proprietary implementation or input.
The native Rust codec imports none of it.

The qualified run uses the retained NDK r30 API37 compiler, real matched Bionic
loader and libraries, and ARM user-QEMU inside a network-disabled unprivileged
Harbour container with no host devices. Original binaries, dependency closure,
loader traces and raw process evidence stay private. Never run a radio daemon
or attach a physical phone for this helper. The exact source and executed ELF
are pinned in [the codec provenance](../../docs/evidence/native-sit-codec-provenance.json).

Compile with the retained `aarch64-linux-android37-clang`, C11, `-Wall -Wextra
-Werror -O2 -fstack-protector-strong -fPIE -pie -Wl,-z,now,-z,relro` and `-ldl`.
Compilation alone is not an original-code result. The helper accepts:

```text
PROBE ABSOLUTE_STREAM_LIBRARY CID INTERFACE_INDEX PROTOCOL APN CARRIER FORMAT
```

The measured subset fixes carrier0, radio argument14, context type0, empty
credentials, APN bitmap1, MCC/MNC00101, all other scalar arguments0, no handover
address/traffic descriptor and false boolean arguments. Protocol is IP, IPV6,
IPV4V6 or NON-IP. The current helper admits format2 (983 bytes) and the separately
recorded original format1 control (638 bytes); Rust supports only format2.

After printing the entire encoded packet, it prints `held` and requires one
`G` byte followed by EOF before cleanup. The qualified controller used that
hold to capture actual credentials, argv, mapped file identity and bytes,
then owned every QEMU child through actual wait and stdout EOF under original
absolute deadlines. Those are separate observations; the helper's `closed`
record alone is not a parent wait receipt.

The original constructor can name different interfaces while the entire request
packet stays identical. This helper does not identify a modem packet channel,
invoke the service handler, configure a link, send a setup request or acquire
an APN. Normal properties/logging and Android linker namespaces remain separate:
the qualified isolated runtime records missing property/linker configuration
and a failed UNIX connection. See [the report](../../docs/native-sit-codec.md)
for the finite contract and remaining gates.

## Base response helper

`original-response-probe.c` is a separate authored parser-only helper. Its CLI is:

```text
PROBE ABSOLUTE_STREAM_LIBRARY COMPLETE_144_BYTE_HEX
```

It uses actual ModemData/adapter constructors, verifies original vtable/error and
non-deleting destructor identities, records scalar observations and separate
193-byte IPv4/IPv6 getter outputs, then follows the same held/release and owned
cleanup contract. C1 supplies the sole Init call. Original heap-copy and guarded
outer allocations are distinguished. No handler, PdpContext update, NetIf or
transport is invoked. The new run has one load and nine parser controls, with
ten direct child waits0/stdout EOF and outer CLI wait0; earlier encoder evidence
is unchanged. It uses CLOCK_MONOTONIC cutoffs, without suspend qualification.
[Response provenance](../../docs/evidence/native-sit-response-provenance.json)
records Root and independent qualification of this original reference. Proprietary libraries, raw traces and analysis remain private.

## Context application helper

`original-context-application-probe.c` adds direct original memory-context
application after the same complete base parser. Its CLI is:

```text
PROBE LIB FULL144HEX CONTEXT_CID INDEX USE_NULL
```

The authored wrapper requires exactly288 lowercase hex characters, context
CID1–30, index0–99 and USE_NULL0/1. Those ranges are test input bounds, not modem
or channel policy. Actual C1 supplies state0 with null APN/TD/controller pointers;
no fields are patched. It calls UpdateDataCallInfo twice, records the complete
352-byte before/source/first fixed prefixes, and reports a real full `memcmp`
for the second call without emitting a second raw array. The return bit is
reported without a copy-success/readiness interpretation.

After the held/release boundary, context D1/guard check/free runs first, then
adapter D1 and packet D1, their guard checks and outer frees. No handler, OnActivated, NetIf or transport is
invoked. Seven controls plus load have eight actual waits0/stdout EOF, separate
from the earlier parser/encoder runs. This adds original integration only,
without native behavior changes or a native test rerun. Independent review qualifies this bounded original run; see the [report](../../docs/native-sit-context-application.md) and
[context evidence](../../docs/evidence/native-sit-context-application-provenance.json).
Raw prefixes and proprietary artifacts remain private.
