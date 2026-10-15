# Bounded native SIT setup serialization

`native-sit-codec` now emits one complete initial token-zero,983-byte SIT setup
packet in the measured format-2 subset. Six original packet comparisons pass
on the host and ARM64 user-QEMU. All eight fixed tests pass per target; formatting
and host all-targets strict Clippy pass, and the ARM64 test executable compiles.
This is a pure codec checkpoint. No modem communication or APN acquisition is
qualified.

## Original reference and fixed comparisons

The corrected original reference uses the genuine Bionic loader and matched
vendor libraries under ARM64 **user-mode QEMU**. Eight owned children cover one
load check, one638-byte format-1 control and six983-byte format-2 outputs. Every
child has actual wait0 and stdout EOF before its original cutoff; the enclosing
run records actual CLI wait0, exact container registration/fresh absence,
same-daemon identity and unchanged selected inputs. This evidence is separate
from earlier [actual ARM kernel/Binder Data tests](native-cellular-data-owner.md).
The separate nine-case original constructor probe establishes only its declared
construction/destruction observations.

The original helper uses actual context construction, original APN factory and
ownership transfer, a caller-derived24-byte builder, and the direct extended
serializer. Callable and vtable symbol offsets are checked against the selected
loaded libraries. Complete packet bytes are copied before original packet
deletion and context destruction. It does not invoke the service handler,
context activation or modem transport. Raw proprietary libraries, decompilation,
private traces and audit records remain private.

| Fixed comparison | Context byte | Protocol | Complete bytes |
| --- | --- | --- | --- |
| `v3-ip` |1 | IPv4 |983 |
| `v3-ipv6` |9 | IPv6 |983 |
| `v3-dual-index0` |9 | IPv4v6 |983 |
| `v3-dual-index17` |9 | IPv4v6 |983 |
| `v3-nonip` |2 | Non-IP |983 |
| `v3-other-apn` |1 | IPv4, `x.invalid` |983 |

The other cases use the authored APN `pokelinux.invalid`. The two index cases
produce identical complete packets: index0 versus17 changes the constructor's
local name without establishing a modem channel, slot or subscription binding.
The earlier private format-0 run remains historical and supplies no native
fixture. Only the corrected format-2 packets are used here. The638-byte control
is an original observation; the Rust API implements only the983-byte subset.

## Native RED and repair

The frozen baseline validates inputs but returns `Unsupported` from `encode`.
After compilation and strict lint pass, the actual host run has two passing
controls and six missing-encoder failures, exit101. ARM64 compilation in that
baseline is not an ARM behavioral RED. Only `src/lib.rs` changes for repair;
the tests, six hex fixtures, manifest and lockfile remain identical. All eight
host tests and eight ARM64 user-QEMU tests then pass, including all six complete
comparisons per target. [Provenance](evidence/native-sit-codec-provenance.json)
pins both receipts, source files, fixtures and selected original outputs.

The API fixes initial token0, radio argument14, carrier0, original context type0,
MCC/MNC `00101`, APN bitmap1, empty credentials, equal protocol/roaming protocol,
null optional text and traffic descriptor, remaining scalar arguments0 and both
booleans false. Original properties are absent, retaining the observed default
branch without handover. Bytes241–244 preserve the factory-produced value1;
its policy meaning is unresolved. Format byte245 is2. Allowed protocol values
are IPv4, IPv6, IPv4v6 and Non-IP. Native APNs are1–99 ASCII lowercase letters,
digits, dots or hyphens. These restrictions and finite vectors do not establish
modem acceptance of all allowed values. `NonZeroU8` supplies a nonzero wire byte,
not a modem CID range or authority to use an existing context.

## Remaining backend gates

This crate is not a completed phone backend. Response framing, bounded parsing,
error/token correlation and uncertain dispatch outcomes remain open. So do
context-to-channel/slot/subscription binding, modem transport, provider identity,
APN lifecycle ownership, deactivation/reset and safe retry. Integration must then
qualify actual kernel/modem behavior, carrier policy and matched Pixel hardware.
The earlier public Data fixture and supervised service observations retain their
own scope; this serializer adds no readiness or link authority to them. No
physical phone was accessed.
