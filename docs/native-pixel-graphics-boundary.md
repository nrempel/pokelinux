# Native Pixel graphics boundary

Retained-evidence review, 17 October 2026. No new build, target execution or
physical-phone access occurred. Native Arch/glibc applications and a native
compositor remain the goal; the selected Bionic PowerVR userspace is an unresolved
backend dependency. Kernel/modules/firmware do not supply that userspace ABI.

The first unresolved rendering boundary is successful original PowerVR device
initialization. Existing virtual graphics and the retained original EGL failure
qualify separate paths; neither qualifies Pixel rendering or scanout.

## Existing original execution

The retained C `runtime-probe` executed the factory EGL through the patched,
pinned libhybris bridge under ARM64 **user-mode QEMU**. It returned client
extensions and a display, then reported
`INITIALIZE=0 VERSION=0.0 ERROR=0x3003` (`EGL_BAD_ALLOC`), exit77. Its syscall
trace opens the factory `powervr.ini` and finds the attempted DRM render nodes
absent. This reaches an actual original API failure, not successful rendering.
An EGL error and absent nodes do not prove that hardware is the only remaining
initialization dependency. This review does not rerun or strengthen the old
harness's process-closure evidence.

The [hash-only provenance](evidence/native-pixel-graphics-boundary-provenance.json)
binds the retained stdout, syscall trace, result and authored probe/validator.
It also records the current retained runtime binaries; this recheck does not
reconstruct a new historical build-to-execution chain. The existing
[virtual graphics](native-virtual-graphics.md) runs use Mesa/virtio/virgl on an
Arch guest kernel, with their own geometry and evidence. Booting the Google
kernel in a generic guest does not instantiate the Pixel GPU or display.

## Selected inputs and limits

The CP3A.261005.005 runtime manifest contains86 selected files. All86 current
files match its existing hashes. The provenance preserves that manifest digest
and selected EGL/GLES, PVR Services, Bionic, mapper/allocator and firmware pins;
it publishes no original payload or configuration body. `libIMGegl` identifies
25.3@6908880; retained `libsrv_um` contains71.2.2448.1212 and DRM node discovery.
These static identifiers do not establish a successful driver connection.

A retained `pvrsrvkm.ko` reports DDK25.3, firmware ABI33,
`gc67b777b0fa8`, `google,lga-gpu`/`platform:rgx` binding and
`6.6.127-android15-8-gee206ffe8752-ab16254410-4k` vermagic. Its symbols include a
PVR DRM ioctl table and PVRSRV bridges; its dependencies include Google
DVFS/voting/SLC/IIF and Trusty components. Its digest matches a retained module
metadata list. **This review has no verified extraction chain from that member
to the authenticated vendor_dlkm image**, so these are retained static metadata,
not authenticated member provenance, actual module loading or KMI qualification.
The separately bounded [factory metadata](../native-boot-inputs/evidence/factory-metadata.json)
records the authenticated archive/image hashes and explicitly leaves KMI
unqualified. GKI and vendor release strings differ; string agreement alone would
not establish KMI either.

A generic DRM render node does not implement the selected PVR Services contract.
DRM includes driver-specific ioctls, and render nodes cannot modeset.
[Linux DRM interfaces](https://docs.kernel.org/gpu/drm-uapi.html).
The upstream kernel and Mesa PowerVR support tables inspected for this review
do not list71.2.2448.1212; matching a product name or supplying an upstream DRM
shim would not qualify this selected device.
[Kernel supported cores](https://docs.kernel.org/gpu/imagination/index.html),
[Mesa PowerVR](https://docs.mesa3d.org/drivers/powervr.html).

## Remaining bringup gates

- **Loader and runtime:** establish the native glibc-to-Bionic loader, properties,
  namespace, TLS and dependency closure for the selected production path. The
  existing selected-library load result and source patches are narrower than
  complete Android17 compatibility or installed-service initialization.
- **Real PowerVR buffers:** qualify genuine device initialization, allocation and
  import with exact format/modifier, plane/offset/stride and handle ownership;
  then cache synchronization, native fences and bounded teardown. An exported
  FD or successful submission does not establish completion or safe reuse.
- **Display ownership:** qualify the separate `vs_drm` scanout path, supported
  display formats, atomic presentation and sole compositor ownership. The
  earlier static PRIME/atomic/allocation observations remain probe targets.
  Render-node access and offscreen rendering do not establish presentation.
- **Matched platform:** qualify actual module loading/KMI, device description,
  firmware and SoC dependencies, permissions and recovery with the unchanged
  selected assets. Generic virtio rendering cannot substitute for this gate.
  [Android KMI monitoring](https://source.android.com/docs/core/architecture/kernel/abi-monitor).

The mapper5 stable-C loader and buffer-free supported-metadata table are an
optional future ABI check after an independent actual original-call oracle.
They are not implemented or executed here, and enumeration would not close
allocation/import or graphics bringup.
[Public IMapper contract](https://android.googlesource.com/platform/hardware/interfaces/+/refs/heads/main/graphics/mapper/stable-c/include/android/hardware/graphics/mapper/IMapper.h),
inspected header blob `23de7b9b71a889954a5bf361cfc186cfc8076ce4`.
