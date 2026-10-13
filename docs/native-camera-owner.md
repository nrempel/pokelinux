# Native camera ownership checkpoint

The device-neutral Rust owner now consumes exact frozen Android 17 camera device
V4 request, result, stream-buffer and notification types through an independent
original-generated NDK peer over real kernel Binder. Original NDK callback
proxies invoke generated Rust `ICameraDeviceCallback` dispatch. The installed
fixture runs in the owned SDK 37 emulator with SELinux Enforcing. This extends
the [NativeHandle boundary](native-camera-boundary.md); it is still component
and synthetic transport evidence, not an installed production camera service.

## Ownership and observed behavior

`native-camera` owns bounded capture records, opaque buffer leases and owned
release descriptors. It has no vendor frame numbers, stream IDs, camera IDs,
Binder transaction numbers or Android parcel types. The test adapter privately
maps frozen wire identities to domain capture/lease IDs. A session/configuration
generation binds every receipt, callback and fence observation.

The executable tests dispatch real `CaptureRequest`, `CaptureResult`,
`StreamBuffer`, `BufferCache`, `NotifyMsg`, physical-camera metadata and shutter
stream-group fields. The NDK peer queries the Rust callback's public frozen V4
version/hash over Binder. These are public reference identities; equality with
the Pixel factory libraries remains unverified. The synthetic control interface
is not `ICameraDeviceSession` and cannot establish full session ABI compatibility.

A first request carries a real NativeHandle FD. Subsequent requests use its
cached buffer ID and an empty handle. Removing the cache entry causes a later
cached submission to return service error 23. Error-buffer and error-request
callbacks still return the owned buffer, but cannot publish successful media.
Duplicate, unknown-buffer and stale-frame callbacks cannot release the current
lease or settle a different capture.

The owner reports four separate facts: observed submission receipt, returned
buffer, capture completion and fence readiness. Shutter plus final metadata and
all expected returns settle a successful capture; they do not imply fence
readiness. A release FD received in a callback stays owned after the Binder
parcel dies. Before the peer's explicit `F` byte, a zero-time poll proves the
pipe is pending and the lease cannot be reused. After the byte, the fixture's
policy allows cached reuse. This is a pipe protocol, not sync_file validation,
gralloc import or permission to reuse physical DMA memory.

Capture completion is recorded independently of the current buffer owner.
Metadata arriving after safe cached-buffer reuse settles the older capture
without completing or releasing the new capture, or authorizing publication
from a buffer already reused. Each receipt batch is validated before mutation;
already-resolved receipts and duplicate capture IDs are refused. Accepted,
rejected, invalid-count and uncertain outcomes cannot be reclassified by a
second adapter delivery of a real Binder result. Negative or oversized accepted
counts quarantine their leases. A successful zero count remains rejected even
if the synthetic peer later emits callbacks or an unrelated count of one.

The held-submit fixture retains its incoming FD before waiting at a bounded
three-second barrier. One owned Rust worker makes the Binder call without
holding the owner mutex. Revocation and snapshots stay responsive while that
call is held. Configuration replacement fences publication immediately and
refuses admission while leases remain quarantined. Late success, metadata,
returns and release signals preserve accounting without restoring publication
or reuse. Session replacement also refuses unresolved leases.

After peer death, a death recipient revokes publication and quarantines the
owned buffer. The pending release pipe reaches actual EOF, which does not mark
it fence-ready. The owner retains its lease and refuses a successor session.
The fixture process and emulator are reaped; this does not establish HAL or DMA
cleanup. Quarantine has no reset shortcut based on local process death.

## Evidence and reproduction

Three live runtime reds preceded the fixes: buffer return incorrectly enabled
reuse while a release FD was pending; late metadata lost completion after
cached reuse; and a duplicate adapter delivery reclassified an already-resolved
receipt. Each failed with exit 101 after original generated Binder dispatch.
The complete corrected suite then passed all nine checks.

The live red/green record and final source identities are in the compact
[public evidence](evidence/native-camera-owner.json). Red outputs, source and
binary snapshots, generated original NDK/Rust code, toolchain, schemas and full
runtime logs stay under ignored `artifacts/camera-boundary`.

```sh
python3 tools/prepare-camera-boundary.py
bash tools/framework-suite.sh camera-owner
```

The preparation script fetches the exact frozen callback import closure to an
ignored directory, checking every SHA256 in the tracked
`framework/camerapeer/frozen-schema-manifest.json`. Both generators refuse
missing, changed or extra schemas. The SDK 37 AIDL compiler, NDK r30 and public
Binder helper pins from the previous boundary remain enforced. Upstream schema
sources, generated source and proprietary artifacts are not published. Java
instrumentation only bootstraps the disposable native endpoint.

The suite owns its container, internal network, emulator, fixture process and
worker lifetime. It records the emulator identity and Binder-library hashes and
removes owned resources on success or failure. Rust formatting, Clippy with
warnings denied, static ELF checks, shell syntax and Python compilation pass.
There is no physical-phone access, vendor execution or shared ARM64 VM change.

## Native actor/process integration update

The separate [native camera service checkpoint](native-camera-service.md) now
composes this owner with a typed current Capture route and retained backend child.
Two actual integration REDs precede17 passing Arch ARM7.1.6 kernel groups using
public generated Rust/Rust Binder, real app PID/UID, held calls, actual Supervisor
channel EAGAIN, permission/lifecycle fencing, original deadlines and actual reaping.
It uses synthetic Root controls and no Source, vendor runtime or original NDK rerun.
The ordinary installed production backend, media/image publication, containment,
qualified reset/reopen and device work remain open. The original checkpoint below
retains its evidence class; the new ARM result does not establish captured images.

## Remaining gates

The owner exposes policy eligibility, not a media publication or storage API.
Production permission enforcement, exclusive admission across processes,
atomic media publication, durable quarantine across daemon restart, bounded
capture-record retirement and qualified reset/reopen remain open. The current
owner backpressures at its configured capture/lease budgets rather than evicting
unresolved records. Revocation is deliberately conservative: existing leases
remain quarantined until a separately qualified cleanup path exists.

The executable scenario uses one output stream/buffer per request, inline
synthetic metadata, no acquire fence and no FMQ transfer. Actual multi-stream
and input buffers, partial metadata accumulation, callback queue saturation,
HAL buffer-management requests (`requestStreamBuffers` is explicitly rejected),
flush/close/drain, hung vendor-call supervision, cancellation and provider
restart require further integration. Metadata test bytes are not image or
calibration data.

Actual sync_file acquire/release semantics, gralloc/dma-buf layout/import,
Wayland/PipeWire/media integration, mapper validation, completed DMA, matching
factory interface/dependency closure, sensor capabilities, preview/photo/video,
privacy indicators, storage durability and later authorized device tests remain
unqualified. The original NDK checkpoint remains separate from the newer ARM
synthetic service runtime above; neither claims a captured image.
