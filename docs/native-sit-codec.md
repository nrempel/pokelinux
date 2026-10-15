# Bounded native SIT setup codecs

`native-sit-codec` now emits one complete initial token-zero,983-byte SIT setup
packet in the measured format-2 subset. Six original packet comparisons pass
on the host and ARM64 user-QEMU. All eight fixed tests pass per target; formatting
and host all-targets strict Clippy pass, and the ARM64 test executable compiles.
That encoder checkpoint remains unchanged. The later base-response decoder
below initially adds14 tests, for22 per target. The later independent-family
repair brings the current total to23 per target. These are pure codec observations; modem
communication and APN acquisition remain unqualified.

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

## Base144-byte response decoder

A separate original Bionic ARM64 user-QEMU run now covers one load control and
nine complete144-byte parser inputs. Six supported cases cover IPv4, IPv6,
dual-family, inactive, crossed address selectors and Non-IP observations. Three
further original controls record header error2, unsigned PDP status65535 and a
wrong message ID. The wrong-ID original leaves default fields; native code
refuses it explicitly. These are parser observations, not modem outcomes.
The earlier constructor9 and encoder8 child counts remain separate.

The authored helper uses guarded original24-byte ModemData and10912-byte adapter
storage, actual constructors and two separate193-byte original address getters.
The adapter constructor naturally calls Init once. Verified non-deleting
original destructors release nested allocations before the outer guarded
allocations are freed. It invokes no service handler, PdpContext update,
context activation, NetIf or transport. All ten owned QEMU children have actual
wait0 and stdout EOF before their original cutoffs. The enclosing CLI has
actual wait0, exact container/fresh absence, same-daemon and unchanged-input
receipts; no separate outer-CLI EOF field is claimed. Those original bounds use
host CLOCK_MONOTONIC and do not qualify suspend behavior.

After strict compilation/lint, the new native baseline has16 host tests pass
(the eight prior encoder tests and eight new refusal groups) and six supported
comparisons fail with `Unsupported`, exit101. ARM64 is compile-only in that RED.
Only `src/response.rs` changes for repair; fixed tests, six response fixtures,
prior encoder and other source remain identical. All22 host tests and22 ARM64
user-QEMU tests then pass, including all six complete input/original-getter
comparisons per target. Root has qualified the native RED/GREEN receipts. A separate independent audit
qualifies the original parser reference within its declared input/lifecycle scope.
[Response evidence](evidence/native-sit-response-provenance.json).

`BaseSetupResponse::parse(packet, expected_token)` accepts exactly144 bytes:
packet type1, header bytes1/11 zero, message0x0600, declared length144 and the
expected32-bit token. Nonzero raw header errors return `HeaderError(u8)`;
nonzero unsigned PDP status returns `PdpStatus(u16)`, without success fields.
CID must be nonzero, activity0–2, protocol1/2/3/5, and both address selectors0–3.
The decoder refuses every other length, including extensions and truncations.
These are explicit native policy checks, not broader original acceptance or
failure-remap equivalence.

Returned activity maps wire1→2 and2→1, leaving0 unchanged; no connected/readiness
meaning is assigned. The initial getter-shaped representation was subsequently
replaced by the complete typed base observation described below. Non-IP has no
local IP address. The final body byte at131 remains opaque. Token equality is local wire correlation, not an owned
operation, provider, subscription or APN authorization. No partial result is
returned on refusal. Original binaries, decompilation and raw runtime/audit
records remain private.

## Independent server families

The original complete parser→fresh-context-copy→modern version22 report pipeline
exposed a real data-loss case: local protocol IPv4 with DNS/P-CSCF selector2 still
reports IPv6 servers, although its IPv6 address getter is absent. All nineteen
emitted report fields in eleven original cases have independent post-run
qualification; this is not complete264-byte ABI equivalence. Root rehashes the
reference and explicitly rechecks those209 fields. The bounded original run is
ARM64 user-QEMU, with no handler, activation, modem or physical phone.

Before the new regression run, the API was changed to separate local address
options from `ServerFamilies<2>` DNS and `ServerFamilies<3>` P-CSCF arrays, and
the old test harness was adapted to explicit getter projections. That baseline
retained the existing lossy protocol gating. The six original getter fixture
files remain byte-identical. After compilation, formatting and strict Clippy,
all22 prior tests pass while the new cross-selector test fails with IPv6 DNS
`None` versus the captured original servers, actual host exit101. ARM64 is
compile-only in that RED. Only `src/response.rs` changes afterward: server
selection follows each server selector, independently of local protocol. Fixed
tests/fixtures pass23 host and23 ARM64 user-QEMU tests. Local setup/controller
failures are retained privately and are not behavioral RED evidence.

Canonical server `Some` preserves selector presence even when all selected octets
are zero. `address_info_v4()` and `address_info_v6()` reproduce the narrower
original getter view, with zeros for disabled servers and `None` for an absent
local family. There is one packet parser; projections do not independently
reinterpret raw bytes. No compatibility alias is retained.

The body uint32LE at127 is now `reported_mtu_v4`: selected original parser copy
and modern-report conversion reach the field consumed by the original logger's
MTUV4 label. This identifies a reported scalar, without a range check, route
configuration, IPv6 MTU inference or permission to configure a link. The byte
at131 remains opaque. [Repair provenance](evidence/native-sit-server-families-provenance.json)
pins the new RED/GREEN, original reference and exact source inventory. Historical
encoder/initial-decoder receipts retain their original source and test scope.

## Remaining backend gates

This crate is not a completed phone backend. Extended responses, broader
error/remap semantics, correlation to an owned dispatched operation and uncertain
outcomes remain open. So do
context-to-channel/slot/subscription binding, modem transport, provider identity,
APN lifecycle ownership, deactivation/reset and safe retry. Integration must then
qualify actual kernel/modem behavior, carrier policy and matched Pixel hardware.
The earlier public Data fixture and supervised service observations retain their
own scope; this serializer adds no readiness or link authority to them. No
physical phone was accessed.
