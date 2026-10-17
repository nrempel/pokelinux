# Installed ARM Messages history across cold boots

The normal production Messages service now starts automatically in two distinct
fresh ARM boots, reconciles authored raw records and retains their exact selected
IDs/provenance/state/body across a clean shutdown and reboot, with its app closed.
The production entry opens local history only. Incoming SMS, modem registration,
ACK/send, notification delivery and app data-release permission remain separate
gates. No Binder supervisor, Source substitute or physical phone is involved.

## Existing boundary and actual RED

The frozen [history probe](../tools/native-installed-services/messages-history-probe.py)
first checks the actual PID1 deployment. On the unchanged original353 service
image it fails with `Installed production Messages unit must be present`, after
real systemctl reports LoadState=not-found. Its expected-data file is absent and
never opened before that failure. This is a missing deployment requirement,
not a Rust history defect or missing test tool.

The earlier [synthetic incoming composition](native-messages-incoming.md)
continues to qualify its own selected receiving path. Its manager/peer setup runs
before manual unit startup; it supplies no existing cold-boot Binder topology.
This new scenario uses the production history-only entry and authored raw input,
without describing it as new incoming reception.

## Selected providers and deployment

A fresh offline build selects current committed Rust sources and five distinct
ARM Release artifacts. Production `messages-service`, `messages-stop` and
`lifecycle-launcher` use normal features. The separate `messages-seed-fixture`
alone enables `isolated-fixture` and has a separate target/output directory.
The provider build verifies ARM machine183, optimization3 and bundled SQLite;
no new runtime package, Cargo dependency or lock change is required. These build
observations are separate from native guest execution.

The [VM profile](../native-messages/packaging/vm) names the real normal ARM
Messages executable and six normal arguments, with protected shared appearance
QML. The app is an unlaunched dependency here; no GUI/session readiness follows.
Use the existing production unit unchanged, its six service arguments,
RuntimeDirectory/StateDirectory and actual `messages-stop` ExecStop.

All selected files pass through the existing finite `install-native-services`
Make stage and authenticated guest copies. No host service is activated. The
separate seed program runs once as root during disposable image preparation,
with `NATIVE_MESSAGES_ISOLATED=1`. It writes two distinct occurrences of the same
authored GSM DELIVER payload, scope unqualified and ACK state stored. The expected
raw IDs/provenance are recorded from that native output, then fsynced. Require
the Inbox database absent and the enabled Messages unit stopped before retaining
the image. The production service never runs in that preparation phase.

## Frozen native assertions

The first separate cold boot must perform no install, seed, reload, enable or
start before its checks. The actual production service must reconcile the exact
raw IDs/provenance into two plain-text `hello` records. The probe checks the real
writer and independent readonly process: both own Inbox descriptors, only the
writer owns raw storage, and Main owns neither database. The actual root admin
socket must belong to the PID1 Main. Snapshot requires locked policy, zero app
principals/permits, and no receiving or incoming counters.

One real systemctl Stop must record Main code1/status0, a successfully exited
`messages-stop`, inactive/dead state, Result=success, empty cgroup and retained
process deaths. No guest parent wait for PID1 processes is claimed. The service's
existing code owns and drains its children before Main exits.

A second distinct cold boot must retain the exact expected/raw/inbox identities
and contents, with a fresh service invocation and no reseed or manual startup.
It runs the same probe and Stop checks again. This tests clean-shutdown/reboot
persistence. It does not qualify power-loss recovery, incoming delivery, app reads,
notifications, suspend or physical hardware. The earlier desktop/window and
synthetic receiving proofs retain their own evidence selections.


## Recorded native result

The production host Make target stages eight exact files. Actual preparation
executes five loader checks and installed-unit verification, then runs the
separate native seed exactly once. Two distinct raw IDs are recorded in a
root0600 expected file and fsynced; the Inbox database remains absent. The unit
is enabled while stopped and never starts in preparation. The353 package map,
all50 selected Qt library/plugin hashes and the original audio/appearance pair
remain unchanged. This image is separate from the earlier385-package healthy
audio-backend image.

Only after QEMU exit0 is the prepared layer retained. Cold1 uses a new overlay
and no installation, reseed or manual startup. Credentialed read-only process/
socket/policy admission uses one original30s budget. The frozen probe observes
the actual writer and readonly children, two plain-text `hello` records matching
the two raw IDs/provenance, locked policy and zero app principals/permits.
Receiving stays false and all incoming counters stay zero. Main and the real
ExecStop helper exit0; the unit is inactive/dead with Result=success, empty
cgroup and retained process deaths.

After cold1's QEMU exit0, its closed layer supplies another readonly backing for
cold2. The second boot has a fresh invocation and preserves the same selected
raw/inbox rows, with no seed, reload, enable or start command. The identical probe
passes every ownership, history and Stop assertion again.

The three VM phases close39 inside direct waits/merged PIPE EOFs:17 preparation,
11 cold1 and11 cold2. Host staging closes another3. Three outer CLI waits0 with
regular console logs and six separate inspection waits retain their own scope;
no inspection PIPE EOF is invented. All QEMU exits0, exact container absence,
same-daemon and original-input closure pass within420/540/600s work/QEMU/outer
bounds. The [hash-only provenance](evidence/native-installed-messages-history-provenance.json)
records the missing-unit RED, current provider builds and three successes. A
prelaunch hash-argument refusal occurs before any Docker/VM execution. Unused
inherited generic fields in preparation metadata remain preserved; explicit
Messages file/stage pins and actual executed source identify this scenario.

Root independently verifies both closed cold epochs and their matching selected
rows, fresh invocations, native exit outcomes and source bindings. The earlier
normal Qt executable remains an unlaunched GUI-cohort dependency, with no old
window result inherited into this headless service proof.
