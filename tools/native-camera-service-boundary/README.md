# Native camera actor fixture

This authored integration fixture composes the real lifecycle actor and camera
process owner with a generated public Rust/Rust CameraV4 Binder peer. The frozen
CameraV4/common imports come from the existing camera schema manifest. Only the
authored ownership probe adds bounded test controls. No original or vendor
library loads here.

The chronological evidence is:

1. The unchanged actor denied the typed current Camera app request while the
   fresh real peer observed zero submissions. Two preceding pre-boot QEMU setup
   failures remain separate from that missing-route RED.
2. The composed actor, before its cleanup correction, left a ready worker alive
   after actual Supervisor socket EAGAIN and Capture grant revocation. The fixture
   cloned the retained parent descriptor via its own pidfd, stopped the exact
   ready worker via pidfd, observed its stopped state, and filled the actual
   channel to kernel EAGAIN. Stop subsequently completed actual child waits.
3. The corrected composed actor passed 17 actual ARM Binder integration groups:
   current independent permission and one bounded receipt; eight held-call
   lifecycle controls; three EAGAIN cancellation controls; deadline, peer death,
   parent death, protected attempt persistence, and separate FD/Owner smoke.

`freeze-composed.py` verifies an immutable source snapshot against the selected
worktree, binds all ARM binaries/generated interfaces, and hashes the selected
kernel, initrd and existing raw backing. `run-composed.sh` uses the pinned local
Docker simulator, no network or UI, a read-only backing, and a temporary qcow2
overlay within the existing bounded tmpfs. Its canonical shared backing lock
spans the actual Docker wait, post-input hashing, and receipt generation. The
historical VM reader takes that lock exclusively; historical preparation refuses
an already present backing separately.

Freeze selections, binaries, generated bindings and raw runtime logs stay in
ignored private artifacts. The authored reports distinguish this full-system ARM
public Binder realm from previous original NDK execution, actual Source ingress,
vendor execution, physical DMA/sync-file behavior, installed media/UI composition,
and physical-device evidence. The application receives bounded state only.
