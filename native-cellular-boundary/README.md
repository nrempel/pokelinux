# Frozen public cellular DataV3 bindings

`pokelinux-cellular-boundary` generates synchronous Rust Binder types from the
original41 public AIDL files:31 DataV3 schemas and10 imported RadioV3 schemas.
[SOURCE.json](SOURCE.json) pins upstream commit, module hashes and each source;
original headers and [NOTICE](NOTICE) are retained. rsbinder and rsbinder-aidl
are pinned to0.12.0. The crate forbids unsafe Rust.

The default feature set generates only public ABI types. Opt-in `test-fixture`
adds the authored control schemas under `tools/native-cellular-data/fixture-aidl`
to the same generated module. It is reference test infrastructure and does not
change the public DataV3 schema. The public endpoint is version3/hash
`cd8913a3f9d39f1cc0a5fcf9e90257be94ec38df`; the authored fixture interface has its
own version1/hash `b74bae7e2fba9b8fbb574174a24e7431dbb6281b`.

Actual generic ARM Binder execution qualifies the Rust/generated-NDK transport
for one complete setup payload, held and two serial-bearing response callbacks,
populated/empty list copies, and peer deathwatch. The
[report](../docs/native-cellular-data-abi.md) and
[provenance](../docs/evidence/native-cellular-data-abi-provenance.json) distinguish
that run from compile prerequisites. See the
[fixture tools](../tools/native-cellular-data/README.md) for the narrow client,
manager and peer.

The crate provides wire types, not a native Data owner or acquisition API.
No APN, slot/subscription, registration, modem/provider identity, kernel link
association, route, carrier, ACK_EXP or installed permission/service completion
is implied. No physical phone was accessed.

From the repository root, a locked offline library compile can use:

```sh
cargo build --locked --offline --manifest-path native-cellular-boundary/Cargo.toml --release
```

Compilation generates bindings; it is no Binder execution or behavioral test.
The `test-fixture` feature additionally requires the adjacent authored schemas.
