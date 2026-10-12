# Native Linux Binder transport qualification

The target is a Rust radio client running in Arch userspace. Android's matched
kernel and vendor services may still be required; using their Binder interface
does not by itself require Android libc in the client.

## Selected dependency and first boundary

The first transport candidate is [rsbinder](https://github.com/hiking90/rsbinder),
version 0.12.0, pinned with Cargo.lock and default features disabled. Its Rust
implementation provides kernel Binder on Linux. The client does not enable its
socket RPC transport or link AOSP libbinder. Upstream source is Apache-2.0;
transitive dependencies retain their own licenses.

`native-radio` currently contains a read-only qualification binary. It opens
`/dev/binder`, checks the context manager descriptor, pings it, looks up the
existing `activity` service, checks its descriptor and pings that remote object.
It sends no radio commands, registers no services and changes no service state.

The initial run and the final locked offline rebuild both passed with SELinux
enforcing. [Source and binary provenance](evidence/kernel-binder-provenance.json)
records the verified identities and output. The binary was compiled
for `x86_64-unknown-linux-musl`; ELF inspection shows no interpreter segment.
Thus the client used Linux libc and the real Binder driver to reach separate
Android service processes. This is stronger evidence than an in-process Binder
capture or a Rust-only mock. The Android emulator remains the test peer; this
result does not mean all Android services have been replaced.

## Reproduce on temporary Harbour or another Linux KVM host

Prepare the Android framework environment documented elsewhere in this repository,
then run from the repository root:

```sh
bash tools/build-native-radio.sh prepare  # Fetch pinned dependencies and build tools.
bash tools/build-native-radio.sh build    # Locked build with network disabled.
bash tools/framework-suite.sh binder-kernel
```

The suite owns a disposable emulator container on an internal Docker network,
addresses only `emulator-5554`, checks SELinux enforcement and removes its owned
container/network afterward. It does not use host ADB or a physical device.
Results are written under ignored `artifacts/native-radio/`.

## Still required

- Generate or implement the recovered vendor radio interface and callbacks.
- Connect the native lossless request planner through this real transport.
- Verify cross-process callback objects, one-way requests, error propagation,
  Binder death, request correlation and bounded shutdown.
- Verify the Pixel vendor service's interface version, service discovery,
  permissions and accepted transaction semantics.
- Qualify ARM64 kernel IPC. The x86 emulator test is not ARM64 Binder evidence;
  the separate ARM64 codec/planner tests do not cover kernel Binder.
- Recover and verify native IMS policy and state machines, modem operation,
  carrier registration and physical audio.

The existing Android framework bridge remains reference test infrastructure.
This qualification does not alter the goal of native phone services or establish
a complete production radio transport.
