# Native ARM64 kernel Binder qualification

A later [cooperative bootstrap checkpoint](native-radio-bootstrap.md) adds a new
production-library parent-bound worker entry before Binder, qualified separately
by actual generic ARM Binder parent-death and live wrong-parent refusal controls.
It preserves durable uncertainty and the installed service-manager requirement;
no complete production worker, vendor cleanup, IMS or phone qualification follows.
The historical execution selections below retain their original scope.

The actual Rust radio transport, publication owner, owned registration/request
workers and process supervisor now execute through Binder under an ARM64 Linux
kernel in system-QEMU. Nine scenarios use an independently authored synthetic
native Rust peer. Three separate scenarios use a synthetic NDK peer executing
with genuine retained ARM64 Android `libbinder_ndk`, `libbinder` and Bionic.
These are distinct from ARM64 user-mode QEMU, the x86 Android reference emulator,
original vendor service execution and physical-device qualification.

The kernel is the unchanged retained Arch Linux ARM `7.1.6-1-aarch64-ARCH` image,
with its matching retained modules and initramfs. Embedded configuration and
`/proc/config.gz` agree on `CONFIG_ANDROID_BINDER_IPC=y`,
`CONFIG_ANDROID_BINDERFS=y`, and an empty legacy Binder device list. Binder is
built in; a Binder module or new kernel build is unnecessary. This test guest
is not the matched Google Pixel kernel. Google target kernel/modules/firmware
assets were neither replaced nor exercised here.

The initial driver provisioning red ran the actual AArch64 Rust executable as
UID 1000 under the guest kernel and failed with `ENOENT` opening the driver.
After guest-only binderfs mounting and `BINDER_CTL_ADD`, the unchanged executable
opened protocol version 8 successfully. The held-control capability red then
crossed real ARM Binder, passed callback transfer, and returned
`UnknownTransaction` before synthetic hold controls existed. Their later
implementation changes the fixture, not production radio policy. Exact failing
sources, binaries and logs are private under `artifacts/native-arm64-binder`.

The nine Rust/manual-peer scenarios qualify:

- Transferred remote Binder objects, nested callback interface-version query,
  raw response and indication callbacks, and kernel-reported caller PID/UID.
- Held registration cancellation and absolute expiry with timers withheld;
  repeated stop/snapshot checks remain responsive, and sixteen replacement
  attempts preserve occupied worker capacity.
- Late successful registration ID 42 and a late callback cannot restore failed
  state. Completion is joined before capacity clears; a second attempt remains
  refused, without automatic `clear` or uncertain registration retry.
- Peer death during registration, and retained death-watch invalidation after
  activation/publication.
- Owned request expiry before dispatch (`NotSent`), a complete 4096-byte two-part
  request/callback, and a held late raw echo retaining `OutcomeUnknown`.
- Permanent synchronous Binder registration in a production-supervised Rust
  child. Local stop/snapshot work remains responsive; the parent revokes output,
  sends SIGKILL, retains Child occupancy until actual reap, rejects the old tag,
  and refuses restart from its synced durable uncertainty marker.

Local thread completion or process reaping is not vendor cleanup. The permanent
peer branch has no release control. Fixture teardown explicitly terminates and
waits for that independent peer; it is separate from the client's local recovery.
Kernel transaction logs and counters include actual transfer, callback and death
notifications, rather than only process stdout markers.

The original Android-runtime evidence is narrower: registration, nested metadata,
response/indication delivery, death during registration and retained-watch death
after publication. The NDK compiler/API35 link stubs are separately hashed and
never used as runtime libraries. Fifteen genuine retained runtime files have an
exact dependency/hash manifest; its frozen metadata-only scope label describes
the inventory checkpoint before runtime qualification, while the separate runtime
results establish the three interop scenarios. Guest process maps confirm the original libraries
and exclude NDK stub paths. No Android or vendor library was patched/interposed.

The first NDK attempt used an unsupported loader argument and remains a harness
failure. The corrected actual runtime loaded the original library, then timed out
waiting for `servicemanager.ready`. Pinned public AOSP property serializer/parser
source and the genuine Bionic property APIs supply that one synthetic startup
property. Actual area initialization/add/readback succeed. A subsequent executed
red exposed the synthetic manager's old transaction numbering. Its corrected
Android 16 subset implements addService (5) and legacy checkService (3), verified
against exact AIDL/generated source. This manager and readiness property are test
infrastructure, not an installed native phone platform or a replacement Android
framework. The explicit library path runs without Android linker namespace
configuration; the loader warning remains recorded. Pixel SELinux/service policy
is not qualified.

Reproduce the completed gates using the prepared retained guest, cached Rust
builder/crates, cached genuine Android runtime and NDKr30 toolchain:

```sh
bash tools/native-arm64-binder/run.sh build
# Run once on a fresh checkout that has no dedicated clone:
bash tools/native-arm64-binder/run.sh clone
bash tools/native-arm64-binder/run.sh run
bash tools/native-arm64-binder/run.sh ndk-build
bash tools/native-arm64-binder/run.sh ndk-run
```

`prepare-public` fetches only small public AOSP inputs pinned by URL and SHA256;
with those present, builds and both VM runs are offline. Every VM uses a dedicated
cloned root disk with snapshot writes, its own lock, no NIC, a read-only project
share, and a separate results directory. The outer container drops capabilities,
runs as the task user, and exposes no host Binder, adb, USB or phone device.
Historical red logs/binaries are retained separately; the current working fixture
is the green implementation. Images, binaries, runtime libraries and raw logs are
ignored/private. [Provenance](evidence/native-arm64-binder-provenance.json) pins
source/build inputs and separates each executed evidence class.

Matched Pixel kernel/policy execution, installed service-manager/app boundaries,
vendor discovery/reset/cleanup, permanent request-driver pressure, native IMS
semantic publication, modem/carrier behavior and hardware remain unqualified.
No physical phone was accessed, and no production backend was enabled.
