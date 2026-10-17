# Installed ARM private audio backend

The installed Rust audio owner now observes a live private PipeWire backend in
a separate fresh generic ARM boot. The unchanged availability-only probe passes
its independent graph checks and one whole-target Stop with all four Main exits0.
No physical phone is accessed.

Before backend installation, the actual ARM owner reports a completed
observation at revision46 with `available:false`; the frozen black-box probe
fails its availability assertion. This is a functional failure after real IPC,
not a missing-test-tool error. The earlier two-owner degraded-boot result remains
separate in [installed services](native-installed-services.md).

## Selected deployment

The explicit [VM site bundle](../native-audio/packaging/vm) adds a private D-Bus,
PipeWire and WirePlumber under UID/GID11040. The generic audio service is unchanged;
a selected drop-in adds dependencies and target Stop propagation. All files pass
through the existing authenticated `install-native-services` Make target and
finite manifest. Staging never activates host services.

A shared tmpfiles rule creates the private runtime/state directories at0700.
One service's Stop cannot delete another service's shared socket directory.
Root-owned site configuration selects only the private manager endpoint, virtual
nodes and policy-disabled WirePlumber profile. No desktop bus is borrowed and no
hardware discovery, default Pixel route or call admission is selected.

The bus and PipeWire units use a new native Rust `backend-ready` executable as
ExecStartPost. It uses nonblocking Unix connections and an original five-second
CLOCK_BOOTTIME deadline, including time suspended. It requires the expected unit
MainPID and UID/GID before returning success, sends no protocol bytes and owns no
workers or child processes. This checks socket admission, not daemon protocol or
audio health. The unit startup limit remains15s. Systemd includes ExecStartPost
in dependency ordering, [as documented](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html#ExecStartPre=).

Each upstream daemon runs directly, without a shell or forking parent, with
Restart=no, cgroup shutdown and a five-second Stop limit. The Rust owner retains
its existing mixed shutdown and owned SIGTERM drain. Requires/After order startup
as bus, PipeWire, WirePlumber and Rust. PartOf propagates one backend-target Stop;
dependency ordering shuts down services in reverse. Process death/cgroup removal is distinct from a direct-parent
wait and from remote vendor cleanup.

The graph contains one driver, `source.media`, `source.microphone` and three null
sinks (`route.speaker`, `route.earpiece`, `route.capture`). No `source.call` producer,
tone or PCM client is started. The existing single `vm-speaker` profile is
byte-identical to the original353 image. CALL_UID0 and APP_UID11042 are explicit
test roles, distinct from backend11040; this test issues no authority operation.

## Rust readiness regression

Three frozen actual Unix-listener/child tests first compile and fail against the
unimplemented helper. The same tests then pass: a matching live peer connects
without protocol bytes; an absent socket reaches the original five-second
cutoff; and a live wrong peer is refused before that cutoff. The wrapper's generic
nonzero label in the original RED is retained; actual Cargo output demonstrates
three assertion failures after successful compilation.

The repaired source passes fmt, strict Clippy, all28 active audio tests and
host/ARM Release builds. One preexisting distinct-UID listener test stays ignored.
No dependency or lockfile changes are needed. These host tests and ARM compilation
are separate from actual installed ARM service execution. The new helper neither
changes the existing audio owner ELF nor inherits old GUI evidence.

## Native integration contract

The unchanged [backend probe](../tools/native-installed-services/backend-probe.py)
first requires a completed Rust observation with `available:true`, no lease,
no desired route and no observed links. It then checks four real enabled PID1
services, their executable/UID/cgroup identities, a private manager socket whose
peer is the actual PipeWire Main, and an independent real `pw-dump`. That graph
must contain the selected virtual nodes and actual PID1 WirePlumber client, with
no service-source route links.

One target Stop must leave each unit inactive/dead, MainPID0 and Result=success,
with empty cgroups and retained process deaths. Rust must report code1/status0;
upstream daemons may report code1/status0 or code2/status15. Those outcomes were
fixed before the backend RED. The guest does not claim parent waits for PID1
services; the audio SIGTERM regression supplies separate direct-child evidence.

The first host staging attempt rejects source file modes before any VM runs.
Its newly copied environment/helper files have0644 instead of0600/0755. The
failure is retained; a fresh bundle with exact manifest modes passes the same
production Make target for eleven selected files. No production bytes or frozen
probe change supplies that correction.

Actual ARM preparation then installs exactly32 signed offline runtime packages,
moving353→385 while preserving every original version and all50 selected Qt
library/plugin hashes. Installed-unit `systemd-analyze verify` passes. All four
services and the target are enabled while stopped, then started through the
single target. The unchanged probe observes Rust availability at revision5,
an independent six-node graph and the actual private manager/WirePlumber peers.
One target Stop leaves all four Main outcomes code1/status0 and Result=success,
with empty cgroups and retained process deaths.

Only after actual QEMU exit0 is the prepared image layer retained. A distinct
fresh boot uses a new overlay over the readonly prepared/original/raw chain.
It performs no installation, reload, enable or start before the frozen check.
All services have fresh invocation identities; both real readiness ExecStartPost
commands report exit0. Rust reports a live backend at revision17 with no lease,
desired route or links. Independent graph and whole-target Stop checks pass
again, with all four normal Main exits0.

Preparation and cold boot close39 and24 inside direct waits/merged PIPE EOFs;
host staging closes another3. Each VM's outer CLI wait0 and two inspection waits
remain separate, with regular console logs. Both QEMU exits0, exact container
absence, unchanged original inputs and the same Docker daemon are verified
within the original420/540/600s work/QEMU/outer bounds.
[Hash-only provenance](evidence/native-installed-audio-backend-provenance.json)
records the original backend RED, helper RED→GREEN, failed stage and both actual
ARM successes. Root independently checks both closed epochs and the exact source
and installed-file bindings; no old GUI result is inherited.

The selected scenario qualifies backend availability and shutdown only. Genuine
session-authorized playback, routing/PCM through the complete phone path,
background audio, physical routes, suspend and Pixel hardware remain separate
gates. No Source, focus, permission or unlock substitute is introduced, and no
physical phone is accessed.
