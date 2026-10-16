# Owned Rust render bootstrap

The original Rust worker now receives an owned initial session/descriptor pack,
executes bootstrap after retaining its deferred native holder, and returns copied
output observations. Actual Linux x86 tests use kernel descriptor operations,
threads, eventfds, timerfd and poll with authored `NativeOwner` adapters. This is
CPU integration; real Aquamarine/GL/DRM and the original Source runtime remain
unqualified. [Hash-only evidence](evidence/native-render-bootstrap-provenance.json)
records the exact source, tests and release archive.

## Input and discovery custody

`OwnedBootstrap` copies source2's408-byte session metadata records and immediately
owns CLOEXEC duplicates of the separate primary/render FDs. The grammar is INIT,
zero to32 ADD records, then ENUMERATION_DONE, with consecutive sequences and the
same nonzero session generation. At most34 records/64 duplicate FDs are admitted.
Primary descriptors must be character devices matching their recorded rdev; a
render descriptor can have a different rdev. Original descriptors/flags remain
untouched. Copied values and shape checks authenticate no real DRM device.

The worker retains the holder before calling `bootstrap`, and the adapter takes
the pack into owner-held state before fallible native work. A bounded identity
copy made before transfer binds returned discovery to the input's session and
device lease/generation tuples. Discovery admits at most32 outputs and rejects
unknown devices, duplicate output IDs/connectors, invalid modes/geometry/format/
flags and nonzero unused records. Width/height use Frame's1..16384 bound; bit0 of
output flags records preferred mode. IDs are local registry keys, not kernel
object authority. An empty input can report zero outputs but admits no frames.

Frame refusal precedes issuing authority and preserves the caller's frame,
credits and earlier unused ticket. Accepted frames must match the discovered
session, output ID/generation and native dimensions. The selected device tuple
also travels to the final native attempt and must match before sealing. These
checks still require a trusted backend to resolve the complete actual request,
old/new routing and native objects. Mode/format changes must invalidate the real
output generation; later registry/session updates are not implemented yet.

## Startup and terminal state

Construction requires an original absolute CLOCK_BOOTTIME cutoff. A NONBLOCK,
CLOEXEC timerfd is armed before spawning. Register both its borrowed FD and the
completion FD before waiting. On completion readiness, inspect discovery and
frame completions; draining an empty completion queue alone can consume the
candidate wake. A deadline callback calls a normal status/discovery/admission/join method to enforce expiry; it must not simply drain the counter.

The worker publishes a copied Candidate, never Ready. Control accepts it with a
post-disarm BOOTTIME eligibility check before exposing availability. A worker or
control turn delayed past the original cutoff cannot activate stale startup.
Accepted readiness disarms the startup timer; stale queued events cannot stop an
already accepted owner. Stop/expiry is terminal, invalidates authority and rejects
late successful results. BOOTTIME includes suspended time but this ordinary timer
does not wake the machine. Suspend/resume and phone wake remain hardware gates.

A held bootstrap or cleanup call stays on its retained worker. Snapshots, Stop
and Busy join remain responsive. Malformed results and ordinary bootstrap errors
stop availability while native shutdown still needs actual verified drain.
Errors notify control before held cleanup returns, even after the startup timer
is disarmed. Pending cleanup is not retirement. Both borrowed watches must be
unregistered before release; fallback Rust Drop still joins and can block.
No worker is detached or replaced, and no operation is implicitly retried.

## Actual tests and chronology

Five frozen semantic RED/GREEN pairs cover missing bootstrap dispatch/discovery;
unchecked discovery/frame admission; wrong-device final grants; missing original
startup expiry/watch; and a terminal error that failed to wake control while
cleanup stayed blocked. Each pair executes real tests, then passes with the
entire test module byte-identical within that pair. The initial fixture Busy
unwrap failure is preserved; corrected tests were refrozen against the old
scheduler before the first qualified pair. A separate Clippy fixture shorthand
failure remains recorded before final QA.

The descriptor module's12 tests were authored first, but its missing-module
baseline was a structural compile failure and supplies no semantic RED. Actual
cases include partial duplicate cleanup after a later memfd refusal, CLOEXEC,
metadata overwrite, original-handle preservation and transfer to a joined worker.
There is no32-device/64-FD positive fixture or injected allocator/dup exhaustion.

Final QA passes36 library cases (8 permit/28 worker),14 Frame and12 descriptor
cases:62 active passes, plus two ignored owned-child modes invoked by parents.
Formatting, strict Clippy and x86 release compilation pass. Two additional
existing-behavior controls preserve an earlier unsealed ticket on frame refusal
and exercise actual empty-input/zero-output worker startup. Seventeen finite
outer epochs retain102 direct waits/102 merged PIPE EOFs, unchanged readonly
inputs, original bounds and exact same-daemon container absence. The owned child
fixtures' closure is separate from those outer receipts. Reaping a quarantine
fixture establishes neither vendor cleanup nor production blocked-call recovery.

## Remaining native and phone gates

A separate [actual GL baseline](native-render-gl-baseline.md) now executes
CLEAR/RECT and retained Frame byte controls against the earlier native Frame
cohort, with complete owned test-channel closure. It selects no new Rust worker
and supplies no SHM upload/native-fence behavior or Source7 GREEN.

The later [native owner resource checkpoint](native-render-owner-resources.md)
qualifies two actual native KVM RED/GREEN pairs: missing fence export, then a
nondefault sampler override producing eight wrong bytes. Final eight-case GREEN
checks padded AR24/XR24, owner/context/allocation refusals and six real sync-file
facts before exact readbacks, followed by EOF/unload/join. Its four native sources
compile into a test plugin against unchanged Frame507/9184/Rust3cf inputs. The
production Rust executor, AQ output/presentation, Source7 and phone remain open.

The bootstrap checkpoint archive is1dcce61e. The later
[current worker C ABI](native-render-executor-ffi.md) now passes actual C++
lifecycle and five authored-owner groups, plus all62 Rust cases and strict QA,
using release archive246a6680. The earlier native Frame ELF91848cff/507-file SDK
still selected3cf0a142; neither newer worker archive is linked into that cohort.
Its historical FFI/codec/native GL results are not relabelled current. Genuine
AQ/GL initialization, later session ingress and OOB revocation, real discovery updates,
complete native routing scope and a separate StopRetire ledger remain required.
The production worker GL/output/presentation/retirement path, unchanged Source7 rendering
GREEN, cold normal app/PCM composition, ARM/matched Google kernel and every
physical Pixel/carrier/camera/power/suspend gate remain open. No phone was accessed.
