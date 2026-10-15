# Bounded native SIT setup codec

`pokelinux-sit-codec` encodes one initial token-zero,983-byte Samsung SIT setup
request and parses the measured144-byte base response subset. Six request
comparisons and six response/getter comparisons match separately executed
original Bionic code under ARM64 user-QEMU. The earlier40-test codec checkpoint is followed by owned receive copying: all50
native tests pass on the host and ARM64 user-QEMU after separate encoder/decoder,
independent-server, modern-report and missing-copy REDs.
The crate has no dependencies and forbids unsafe Rust.

`InitialLteSetupV3::new` accepts a `NonZeroU8` context identifier, an APN of1–99
ASCII lowercase letters, digits, dots or hyphens, and one of `Ipv4`, `Ipv6`,
`Ipv4v6` or `NonIp`. `encode` returns the complete983-byte array. These are native
input restrictions; the finite comparisons do not qualify every accepted
combination. The nonzero byte establishes wire width, not a valid modem CID,
subscription, context owner or interface assignment.

The packet fixes radio argument14, carrier0, original context type0, initial
token0, MCC/MNC `00101`, APN bitmap1, empty credentials, null traffic descriptor,
all remaining scalar arguments0 and both booleans false. The original property
area is absent, so its missing-property/default branch is part of the reference
premise. The factory-produced scalar at bytes241–244 is preserved as1; its
meaning remains unresolved.

`response::BaseSetupResponse::parse(packet, expected_token)` returns owned typed
observations only after exact length, header, message, declared length, token,
error/status and field-policy checks. A nonzero CID is a wire byte; activity
preserves the original numeric1/2 translation. Local address options follow the
PDP protocol. DNS and P-CSCF each have independent optional IPv4/IPv6 arrays;
selector presence remains distinct from all-zero addresses. Explicit
`address_info_v4()`/`address_info_v6()` derive the original getter projection,
including its local-family omission and zero fill, without losing servers from
the canonical observation. The uint32 tail becomes `reported_mtu_v4` through the
selected original modern-report/logger mapping; the final byte remains opaque.
No MTU configuration policy is implied. Token equality grants no dispatch,
provider or APN authority. Broader response formats are refused.

`modern_report::ModernSetupReport22::project(&response, presentation_index)`
returns the modern-only owned scalar/text/vector observation for the fixed base
subset. Fourteen captured original reports match; inactive/null text and Non-IP
empty text remain distinct. Local and server zero filtering differ, independent
server families survive, and compatible IPv6 spelling matches the measured
original. Explicit index0–99 supplies presentation only. Invalid public input
consistency is refused. Fixed empty arrays do not implement unmeasured dynamic
ABI fields. See the [report projection](../docs/native-sit-report-projection.md)
and [provenance](../docs/evidence/native-sit-report-projection-provenance.json).

`received_chunk::ReceivedChunk::copy_from` owns a nonempty raw slice up to65,568
bytes through fallible allocation. Its readonly bytes grant no header, token,
length or record validity. Seven full original C1/D1 comparisons include short,
coalesced, opaque and maximum-bound inputs; source overwrite/drop preserves the
native copy. Null/signed-count/boolean C++ ABI behavior remains reference-only.
The bound is native ingress policy, not a protocol or kernel maximum. See
[receive ownership](../docs/native-sit-received-chunk.md) for strict RED/GREEN,
separate original-code evidence and supervised process integration.

From the repository root:

```sh
cargo test --locked --offline --manifest-path native-sit-codec/Cargo.toml
```

These are pure serialization and parsing observations. There is no request
submission, provider or kernel-link association, APN-readiness grant, reset/deactivation,
automatic retry or physical-phone qualification. The constructor's interface
index does not select a modem channel. See the [report](../docs/native-sit-codec.md)
and the [encoder](../docs/evidence/native-sit-codec-provenance.json) and
[response](../docs/evidence/native-sit-response-provenance.json) provenance for the exact
reference, RED/GREEN and remaining gates.

The [independent-family repair](../docs/native-sit-codec.md#independent-server-families)
records the pre-RED API/getter test adaptation, unchanged original getter fixture
bytes and fixed-test23/23 repair. Earlier provenance records its historical source.
