# Native graphics resource operations

The matching normal Aquamarine library and SDK build now pass after a genuine
missing-interface RED. All ten existing CPU groups, two retained API checks and
two exact-operation API checks pass. Real allocation/import behavior is the next
execution gate; no displayed app frame or phone behavior is qualified here.

Portable format decisions and lifecycle belong in Rust; the native adapter
still has C++ policy awaiting replacement. This small C++ dependency patch
performs native GBM/DRM operations and retains their objects. It uses the
existing swapchain and framebuffer ownership rather than adding another manager.
The [Rust format selector](native-render-format-policy.md) remains separate and
currently refuses implicit selection; this fixture cannot grant it eligibility.

The [public source and frozen tests](../tools/native-render-owner/exact-buffer/README.md)
cover one exact allocation path, raw exported metadata, matching framebuffer
import, explicit refusal and release. Device equality comes from actual
`drmGetDevice2` and `drmDevicesEqual` observations. An implicit operation explicitly
uses `gbm_bo_create` and AddFB2 flags0; an explicit operation uses exactly the
supplied modifier and requires capability1. The returned modifier never changes
the request mode. There is no format substitution or automatic fallback.

Holders retain native acquisitions before subsequent effects. Failed swapchain
reconfiguration preserves the old committed buffers/options. Reuse checks the
complete request, live output identity and options. A fresh exact buffer cannot
enter the older framebuffer import path. Exact close records one CLOSEFB result,
with no hidden retry or RMFB fallback. CLOSEFB success does not prove retirement
of active scanout; this consumer only plans a never-attached registration case.

The actual structural RED used genuine GNU16/C++23 and the existing normal SDK:
the control exited0 with no diagnostics, and the target exited1 on exactly six
missing interfaces. Its failed aggregate remains preserved. After implementation,
all three changed CPP files were compiled into the new normal AQ15 library. The
complete matching SDK was read back, including24 regular files and two links.
All57 distinct owned waits and57 PIPE EOFs close, with QMP EOF recorded separately,
QEMU/CLI exit0 and the exact container absent on the same daemon. This is source,
loader and CPU-boundary evidence, not kernel operation or presentation evidence.

The next unchanged five-group consumer uses a real seatd broker, Rust-owned
worker and virtual DRM driver. It checks allocation/export, actual cap0 explicit
refusal, request mismatches without import, AddFB2 plus independent GETFB2, and
never-attached CLOSEFB/export-FD release. Its actual execution remains pending.
Partial PRIME/GEM-close failures, explicit positive allocation and all active
scanout retirement remain unqualified by those five groups.

Connect qualified operations to the Rust selection/admission and production
prepare/execute path, then run the same normal app session through real client
content, presentation, Sounds/PCM, permissions, live Omarchy themes, Source and
clean shutdown. ARM evidence and the matched Pixel kernel/GPU/firmware hardware
gates remain separate. No physical phone has been accessed.
