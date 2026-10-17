# Immutable CPU texture resources

A private CPU implementation of the existing `Render::ITexture` retains a
separately owned Rust SHM snapshot. Two public functions in `Texture.hpp` create
the immutable resource and borrow its real snapshot on its creator thread.
The [source patch and frozen consumers](../tools/native-render-owner/texture/README.md)
contain no replacement renderer or graphics identity.

The factory accepts actual AR24/XR24 four-byte snapshots with valid retained
layout. Size and format come from that descriptor; RGBA/RGBX type and explicit
opacity remain independent. The existing normal transform, linear sampling and
absent explicit colour description are preserved. GPU ID stays zero. A new
factory success publishes a new object, leaving previously admitted Rust clones
unchanged. No C++ pixel-vector copy is added.

The output must be empty or a genuine CPU object owned by the calling thread
lifetime. This check precedes cloning, allocation and assignment, so rejecting a
native texture cannot invoke its graphics destructor. Creator identity uses a
private monotonic atomic counter and a cached thread-local token; it saturates
rather than wrapping. Recycled C++/OS thread identities cannot reactivate an
old object. Destruction releases only Rust custody, including after creator exit.
A synchronous borrow must be cloned before it becomes a queued frame resource.
Public texture metadata is still serialized by its caller, not generally thread-safe.

Nearest and linear sampler setters update real material metadata. Foreign-owner
setters, bind/unbind, native allocation and every legacy update explicitly throw
before mutation. Empty damage is also refused: replacing a whole image would
change the existing damage-clipped update contract. Capture new content before
factory replacement. Unsupported sampler parameters throw without changing
metadata. DMABUF/external images and LUTs have no accepted factory overload.
Legacy nonvirtual `dataCopy()` remains an empty C++ vector for this resource;
its cursor consumer still needs migration to the typed snapshot borrow/clone.

## Actual structural RED

In one offline native x86 KVM guest, the unchanged existing-type control compiles
with strict flags and zero diagnostics against the authenticated original SDK.
The frozen target fails only because both `createCPUTexture` and
`borrowCPUTextureSnapshot` are absent, followed by their eight precise dependent
alias/type errors. Both dependency maps select the same17 private headers from
the original535-regular/two-link SDK. This is a missing-interface compilation
RED, not a runtime pixel failure. No future declarations were added to the tests.

All30 direct waits and37 PIPE EOFs close within their original bounds. Same-peer
QMP observes KVM and normal guest shutdown; QEMU and foreground Docker CLI wait0,
the exact container is absent on the same daemon, and all locked inputs remain
unchanged. No Rust archive, link, texture instance or method body executes in
this structural epoch.

## Functional qualification and remaining gates

All five actual native x86 C++/Rust groups pass after implementation:

- Real5x3/stride32 memfd capture retains all96 pixel/padding bytes after producer
  overwrite, unmap and descriptor close.
- AR24/XR24 type and explicit opacity remain independent.
- Real sampler changes and refusal checks preserve pixels and metadata. Two
  joined threads have distinct Linux TIDs; the successor actually reuses the
  retired creator's C++ identity, yet borrowing, mutation and replacement refuse.
- New-object replacement preserves the old object and separately admitted clone.
- All legacy native/update calls, empty input and actual RGB565 input refuse
  without changing the current resource.

Both unchanged structural TUs compile cleanly in the same guest. The real link
selects eight objects: the consumer, new CPUTexture.cpp, original Texture.cpp and
SHMSnapshot.cpp, and four original logging/colour support files. Original SDK535+2
is authenticated before a single Texture.hpp overlay; this is not a rebuilt main
SDK. Only old Texture.cpp and the four support files retain their exact original
warning options. Consumer, new adapter and SHMSnapshot remain strict. The sole
Rust246 archive, actual loader relocations and ELF/link-map closure qualify.
No native graphics entry-point or renderer implementation symbol is admitted;
two exact original empty unique-pointer destructor wrappers remain recorded.

The consumer finishes in5.764ms inside its original5s bound. All47 direct waits
and54 PIPE EOFs close within original limits, followed by actual normal QMP
guest shutdown, QEMU0, CLI0, same-daemon exact-container absence and unchanged
locked inputs. [Hash-only evidence](evidence/native-render-cpu-texture-provenance.json)
binds the distinct structural RED and postimplementation functional GREEN.
The producer is in the consumer process; no separate producer-child exit is claimed.

Two earlier functional attempts remain setup failures: an unused test helper,
then an overbroad symbol-name filter. Exactly five dead helper lines were removed;
all preselected functional assertions/groups and both real thread checks remain.
The symbol repair admits only the two original generic empty unique-pointer
wrappers. It retains every other native/renderer exclusion and exports ELF/map
before admission. No production bytes, warning flags or cutoffs changed after
implementation. This is not an original semantic runtime RED. Recoverable C++
allocation exceptions return PR_ALLOCATION; inherited Rust metadata and Hyprutils
noexcept shared-control-block allocation may terminate on exhaustion. Complete
allocation-failure recovery is not qualified.

The core `updateSynchronousTexture` still calls the existing GL renderer after
Rust capture. No fresh full compositor/SDK build, real Wayland client frame,
CPU scene extraction, asset/cursor admission, worker presentation, native
readiness or retirement is qualified here. Normal CPU bootstrap and the genuine
client-to-worker-output tracer remain next. Source7, Sounds/PCM and Messages
composition, current ARM and matched Google kernel qualification, all Pixel
hardware and carrier gates remain open. No physical phone was accessed.
