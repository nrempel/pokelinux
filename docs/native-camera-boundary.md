# Native camera Binder boundary checkpoint

The synthetic camera boundary now runs original Android 17 generated NDK
proxy/dispatcher code over the real Binder driver in the owned SDK 37 emulator.
An independent static Linux Rust client uses generated rsbinder bindings against
the same NDK dispatcher. This is executed interface and FD evidence, separate
from the earlier AIDL compiler generation and static factory audit. It is not a
camera service, capture implementation or Pixel qualification.

The public frozen `hardware.common` V2 `NativeHandle` is fetched unchanged into
ignored artifacts with its Apache notice and checked against a tracked pin manifest. `ICameraBoundaryProbe` is original test infrastructure, with
a synthetic descriptor/version/hash. No factory ABI equality is claimed. Java
instrumentation only starts the disposable JNI endpoint and passes its Binder
object to the clients. Java is not part of the intended production runtime.

## What ran

The NDK peer duplicates and retains the incoming pipe FD, sets its local int
return to 1, then either returns success, returns service-specific error 17 or
writes a receipt byte and kills its own process before replying. Both clients
must hold a remote Binder object; a local direct-call shortcut cannot pass.

The generated NDK dispatcher omits the int on error. Its remote proxy reports
error 17 and leaves the caller's sentinel `-77` untouched. The generated Rust
proxy returns an error without a count. The Rust adapter consequently reports
`SubmissionReceipt::Uncertain`; neither client derives an accepted subset or
capture completion from that failed call. The peer really retained an FD before
returning the error, so uncertainty is required even though the return count is
not observable.

Both clients close their sender FD before asking the peer to return a duplicate
and release its retained FD. The peer's `R` pipe byte proves independent receiver
lifetime. A client then writes `C` through the returned FD after receiver
release, proving independent reverse-transfer lifetime. Closing the final copy
produces bounded EOF. Each client completes 64 alternating success/error
submit/release cycles with exact client and peer FD counts unchanged.

For death before reply, the incoming FD carries `D` before the peer kills
itself. The clients observe `DEAD_OBJECT`, no valid acceptance result, bounded
EOF after closing their source copies, a death-recipient notification and a
subsequent transaction failure. The harness verifies that the owned peer
process is absent. The NDK death case is followed by a fresh Rust-client/peer
bootstrap. This qualifies the disposable fixture restart only.

Both clients run as emulator `uid=2000(shell)` / `u:r:shell:s0`; SELinux remains
Enforcing. The outer suite uses its existing workspace lock, dedicated internal
network and owned-container cleanup. The completed emulator container and
network were removed. No phone, carrier, sensor or vendor binary was accessed.

## Red before green

Two live runtime failures preceded the fixes:

- NDK: remote bootstrap, independent FD lifetime and untouched error output
  passed, then the adapter failed `service error has no valid acceptance
  result` because it trusted a count despite failed status (exit 1).
- Rust: remote bootstrap and independent FD lifetime passed, then the adapter
  returned `Accepted(1)` instead of
  `Uncertain { service_error: 17, transport_error: Ok }` (exit 101).

The NDK fix requires successful status before exposing the count. The Rust fix
preserves service/transport errors as an uncertain receipt without a count.
The complete combined suite passed twice with both fixes; the final run also
recorded pinned input and runtime-library provenance. Formatting, Clippy with
warnings denied, static Rust ELF checks, shell syntax and Python compilation
passed. Provisioning/bootstrap failures were diagnosed separately and are not
counted as behavioral red evidence.

## Reproduce on temporary Harbour capacity

The existing framework SDK/image and native Rust build image must be prepared;
these are the same prerequisites as the repository's Binder suites.

```sh
python3 tools/prepare-camera-boundary.py
bash tools/framework-suite.sh camera-binder
```

Preparation downloads official NDK r30 `30.0.16248370` into ignored
`artifacts/camera-boundary`, verifies its size, published SHA1 and recorded
SHA256, and pins matching public Android 17 Binder C++ helper headers. The
prepared SDK 37 AIDL compiler and frozen NativeHandle input are also checked by
SHA256 before generation. The build leaves generated code, binaries, APK,
compiler/runtime/source identities and all red/green outputs in that ignored
artifact directory. Generated NDK code and toolchain bytes are not committed.

The standalone NDK client resolves public platform service-discovery/thread-pool
symbols from the emulator's `libbinder_ndk` for its disposable framework
bootstrap; these APIs are absent from NDK app link stubs. This is an explicit
test-infrastructure dependency, not a retained production service dependency.
The native Rust client has no ELF interpreter and uses the Binder driver
through rsbinder 0.12.0.

The compact [public evidence](evidence/native-camera-boundary.json) contains
only synthetic/public identities and results. The private camera REA dossier,
proprietary target bytes, credentials and raw decompilation remain ignored.

## Remaining camera gates

Pipe FDs are neither sync_file fences nor gralloc/dma-buf camera buffers. These
checks do not prove mapper import, layout/stride/usage, fence signalling,
allocator release, cache IDs, completed DMA, provider flush/close behavior,
callback reconciliation, cancellation, generation revocation or safe successor
reuse. A locally reaped synthetic peer does not establish hardware cleanup.

The later [owner continuation](native-camera-owner.md) executes frozen camera
device V4 stream-buffer, capture request/result and callback types over Binder,
including cached handles, error returns, delayed pipe signals and quarantine.
Full session/FMQ schemas and matching factory hashes still need qualification. Matched vendor processing/Bionic/configuration/calibration/firmware,
kernel permissions, sensor capabilities, privacy, storage, preview/photo/video
and later authorized physical capture tests remain open. No ARM64 camera
execution or physical camera evidence is claimed here.
