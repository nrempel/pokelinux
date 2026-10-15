# Bounded native SIT setup codec

`pokelinux-sit-codec` encodes one initial token-zero,983-byte Samsung SIT setup
request and parses the measured144-byte base response subset. Six request
comparisons and six response/getter comparisons match separately executed
original Bionic code under ARM64 user-QEMU. All22 native tests pass on the host
and ARM64 user-QEMU after separate missing-encoder and missing-decoder host REDs.
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
preserves the original numeric1/2 translation. Optional IPv4/IPv6 parameters
contain an address, two DNS and three P-CSCF values. Family presence follows the
PDP selector; separate DNS/P-CSCF selectors determine copied or zero fields.
The two observed tail fields stay opaque. Token equality grants no dispatch,
provider or APN authority. Broader response formats are refused.

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
