# Original SIT codec comparison helper

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
