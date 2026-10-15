# Public DataV3 ABI fixture tools

These tools are reference test infrastructure for generated Rust/NDK Binder
interoperability. They provide no production Data owner, modem backend, APN
acquisition, slot mapping, kernel association, route or carrier authority.

- `peer.cpp` implements the generated public DataV3 endpoint. Authored fixture
  controls capture one setup, submit normal setup/list callbacks and exit the
  peer. Other Data operations are refused.
- `fixture-aidl` and `fixture-interface-hash.txt` define the authored controlV1
  boundary. They are separate from unchanged frozen public DataV3 AIDL.
- `interop` is the Rust client. It uses the bindings' `test-fixture` feature,
  validates metadata, compares all10 setup arguments/21 profile/18 result
  fields for the selected payload, holds callbacks for100ms, copies serial714
  then713, copies populated/empty lists and observes actual Binder death.
- `manager` transfers one named fixture binder. It is a narrow authored context
  manager, not a general Android service manager or installed discovery policy.
- `build-ndk.sh` generates and compiles the peer/property helpers. Its caller
  must supply absolute selected workspace and nonexistent output paths and own
  the offline container, immutable input closure, child waits and final receipts.

The actual NDK compile records43 generated C++ files,129 headers,52 translation
units/waits and1066 dependency paths, followed by three links. NDK `.so` files
are link stubs; they must never replace genuine libraries in a runtime selection.
SDK AIDL also needs its fully pinned adjacent host `lib64` directory. This build
is a prerequisite and executes no generated ARM binary.

The separately qualified actual ARM run uses a private Binder device, genuine
Bionic loader/libraries for the peer and a Rust client/manager on a generic
Linux7.1.6 VM kernel. All three run at UID/GID1000 with no supplementary groups
or effective capabilities. Process and mapping evidence, eight actual child
waits/EOFs, fixed original deadlines and owned container/input closure are
summarized in the [report](../../docs/native-cellular-data-abi.md) and
[provenance](../../docs/evidence/native-cellular-data-abi-provenance.json).

Chosen result coverage uses `Qos::noinit` and empty QoS sessions. Bounds are
checked after generated decoding; hostile decoder memory, other QoS variants,
ACK_EXP, native operation fencing/recovery and Binder pool/owned-worker joins
remain unqualified. Missing Android linker-config warnings are retained;
Android namespace/config and SELinux are unqualified. The two copied serials are transport evidence, not an owner
acceptance/rejection policy. The dedicated manager's exit−9 is owned cleanup,
separate from the peer's exit0 and deathwatch.

Locked offline release compilation from the repository root:

```sh
cargo build --locked --offline --manifest-path tools/native-cellular-data/interop/Cargo.toml --release --target aarch64-unknown-linux-gnu
cargo build --locked --offline --manifest-path tools/native-cellular-data/manager/Cargo.toml --release --target aarch64-unknown-linux-gnu
```

These are compile commands only. Actual execution requires a fresh isolated ARM
Binder/property fixture and complete owned custody; a build or ordinary IPC
return cannot substitute for a copied callback receipt. No host behavioral test,
vendor endpoint, normal installed app, authentication or phone pass is claimed.
