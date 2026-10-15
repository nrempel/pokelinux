# Bounded native SIT setup codec

`pokelinux-sit-codec` encodes one initial token-zero, 983-byte Samsung SIT setup
request in the measured format-2 subset. Six complete outputs match original
Bionic code executed under ARM64 user-QEMU. The fixed Rust tests pass eight cases
on the host and eight under ARM64 user-QEMU after a genuine missing-encoder RED.
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

From the repository root:

```sh
cargo test --locked --offline --manifest-path native-sit-codec/Cargo.toml --lib
```

This is serialization evidence. There is no request submission, response parser,
provider or kernel-link association, APN-readiness grant, reset/deactivation,
automatic retry or physical-phone qualification. The constructor's interface
index does not select a modem channel. See the [report](../docs/native-sit-codec.md)
and [provenance](../docs/evidence/native-sit-codec-provenance.json) for the exact
reference, RED/GREEN and remaining gates.
