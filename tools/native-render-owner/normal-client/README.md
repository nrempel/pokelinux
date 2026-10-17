# Real normal Wayland client boundary

The exact [client](real-two-frame-client.cpp) authenticates the compositor's
actual socket peer PID/UID/GID and performs real registry and bound-global syncs.
It requires wl_compositor version4 or later, wl_shm, xdg_wm_base and actual
ARGB8888 advertisement. A healthy round trip with missing globals is a functional
failure. Connection errors, crashes or expiry cannot stand in for that failure.

The executed initial boundary uses one normal compositor after its Rust worker
accepts bootstrap and ordered CHANGE. The real UID/GID1000 client observes mask0
and naturally exits77 with the exact missing-globals event. A post-sync public
snapshot remains healthy Ready with the same ten reconciliation watermarks.
The [cohort](source-cohort.json) binds the frozen source, actual matching native
providers, build and independent qualification. This is actual x86 KVM virtio
integration; it is separate from emulator, ARM and physical-phone evidence.

The guest generates the stable xdg-shell client header and private protocol C
with its actual wayland-scanner/XML. It compiles generated C with C11 and this
client with C++26, -O2 -Wall -Wextra -Werror, links only wayland-client, and checks
actual loader relocations. No Rust archive or compositor plugin is linked into
this consumer. Its arguments are the runner's original absolute CLOCK_BOOTTIME
work deadline and the exact owned compositor PID; its control input is a pipe.
The independently owned runner must bound output, retain children and actually
wait for exit and EOF.

Later client commands are implemented for the same session:

| Command | Actual client action |
| --- | --- |
| G | Create the real XDG surface and send the initial empty commit |
| A | Attach/commit the first5×3 ARGB memfd image, seed23 |
| M | After release/callback/sync, overwrite, unmap, close and destroy its producer |
| B | Attach/commit a new image, seed91, on the same surface |
| D | Dispose the second producer after release/callback/sync |
| Q | Disconnect successfully after the second content and disposal complete |

The recorded RED sends none of these commands. Client callbacks alone prove no
native presentation; later two-frame proof also needs actual Rust admission,
same-buffer rendering/readback, correlated pageflip and safe replacement/retirement.
Native Stop, drain and join need their own actual receipts while the client stays
alive. Main process disposal supplies no native cleanup.

Original35s work/45s close,2s public requests and64 records/64KiB combined evidence
remain fixed. All38 direct waits/45 PIPE EOFs close, with QMP and three public
response EOFs counted separately. The initial launch failed before QEMU because
of an old runner phase label. Exactly three label checks were corrected; that
failed epoch remains separate, without a production-bug RED claim.

The fixture requests the Pixel10 profile. This boundary measures no native output
geometry or capture, so it produces no qualified UI preview. See the
[report](../../../docs/native-render-format-policy.md#actual-normal-wayland-client-boundary).
