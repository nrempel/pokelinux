# Cold startup callback diagnostic

A bounded diagnostic now passes its native component consumer and a complete
matching Hyprland/SDK build. It has not yet executed real compositor callbacks
or identified the cold-start Source gap. The unchanged100ms freshness rule and
full installed shell/Sounds consumer remain the acceptance gates.

The [temporary upstream patch](../tools/native-callback-spans/callback-span-producer.patch)
brackets existing Aquamarine, selected event-loop timer and deferred callbacks.
It copies identities and records BOOTTIME plus the compositor main thread's CPU
time in a fixed1024-record ring. It adds no worker, callback scheduling, logging,
new resource ownership or readiness predicate. Normal-return records occur only
after normal callback return. Trace unavailability does not suppress callbacks.
The [tool contract](../tools/native-callback-spans/README.md) describes the fixed
file and post-exit reading requirements.

A frozen current-source compiler control passes before the genuine missing
producer-type structural RED. The exact extracted implementation then passes
three native groups: real normal/nested/unmatched callback records with errno
preservation; fixed-file/unavailable/sticky refusal; and rolling overflow. The
normal file retains7 records and3 complete spans; the overflow case reserves1200
records and retains1024, with512 complete spans and176 overwritten reservations.
All16 actual direct waits and16 merged PIPE EOFs close under the original bounds,
with both exact containers absent on the same daemon. This used GCC14.2 and
executes the writer component, not Hyprland callback dispatch.

The frozen reader independently rejects crossing completed spans after an actual
v1 format RED. All10 authored format controls pass against v2; its actual FIFO
CLI refuses without blocking. These three process waits/merged EOFs close too.
Authored partial-commit and clock-error bytes do not constitute actual producer
interruption or a forced clock syscall failure.

A separate fresh clean build uses the genuine selected GCC16.2.1/GNU++26,
448 compile commands and the normal default feature set. Its actual event-loop
object compilation and final compositor link pass. The full498-file SDK is
read back from the exported archive; only generated version.h differs from the
prior normal SDK. The selected source changes are the earlier one-entry logical
output allowlist and this diagnostic CPP. The build completes in159.311 seconds,
with31 direct waits/31 merged PIPE EOFs, separate QMP EOF, actual QEMU/CLI0,
exact container absence, unchanged inputs and the original resource/deadline
limits. It executes no compositor ELF, Source, app or hardware.

The next runtime must retain the exact tmpfs file descriptor/device/inode and
compositor main pidfd, require PID=TID=actual main, and read only after exact
writer death and source retirement. At most16 Source stamps may come from the
existing authenticated observer records; no second Source reader is added.
Temporal overlap, elapsed-vs-thread-CPU time and unmatched entries each have
limited meaning. Raw Wayland timer dispatch and other callbacks remain outside
coverage; no internal operation or exclusive CPU/driver cause is established.

[Curated hashes and evidence scopes](evidence/native-callback-spans-provenance.json)
separate format controls, native component execution and the matching full build.
Cold startup, the full Omarchy shell/menu, current ARM composition and all Pixel
hardware gates remain open. The production phone-service and app-core target
continues to be Rust; this C++ probe measures a retained upstream dependency.
