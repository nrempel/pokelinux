# Combined native ARM development image

The prepared generic ARM image now cold-boots the installed battery, brightness
and local-network owners together. Two distinct cold boots retain their site
configuration across a runtime-only reinstall and a subsequent reboot. Existing
qualified ELFs are used; this epoch adds no package, compiler, kernel or driver.
The original353-package inventory and its50 selected Qt libraries/plugins remain unchanged.

The frozen Python positive consumer is recorded as `source/combined-consumer.py`;
its [repository copy](../tools/native-installed-services/development-image-probe.py)
is published unchanged.
Before installation, actual PID1 checks pass the existing battery/brightness pair
and fail the absent network unit and normal Settings executable: TTFF. The
positive consumer exits1 for those installed-boundary assertions. This is an
authored Python orchestration RED, separate from the unchanged native C++ Qt case.

Production `install-native-services` stages twelve selected files. Its three
host commands reach actual wait/merged PIPE EOF and close handles before Docker.
The guest normally Stops the pair, applies or verifies the selected files,
checks loaders and units, and enables all three while stopped. Preparation
closes47 waits/EOFs and exports a layer after direct QEMU wait0/EOF/handle closure.

Both fresh cold boots find all three enabled PID1 owners active before any
install, reload, enable or manual start. Their invocation IDs change between
boots. The unchanged UID11050 offscreen Settings component sees network0/0,
then a normal network Stop, expiry without Qt events, unavailable dashes and0/0
through a fresh service invocation. Getter expiry after loss is2.000105034seconds
in cold1 and2.000270254seconds in cold2. The initial power owners retain their
identities throughout this intentional, separate network recovery exercise.

Readonly power observations report honest battery unavailability with cleared
capacity/status. Linux GPIO-backed backlight disappearance and recovery produce
Client revisions1→2→3, with an absent observation between real attribute reads.
No brightness Set is issued. All three normal service Stops finish with exit0,
dead retained Main pidfds, absent sockets and empty cgroups. PID1 owns these
Mains; those outcomes are separate from fixture-parent wait receipts.

For cold1 the host stages ten runtime rows before the VM, excluding both site
configs; the guest applies them after the service Stops. Both configs and an
authored unrelated sentinel retain bytes/hash, mode, ownership, inode and mtime
before and after reinstall, and through cold2. Cold2 performs no host staging.
The prepared layer is24,051,712bytes; the reinstalled layer is28,835,840bytes.
Each export follows direct QEMU wait0, PIPE EOF and handle closure.

The closed phases record47+78+72=197 actual waits and197 merged PIPE EOFs, all
within their original cutoffs. The expected preparation assertion is the only
nonzero command outcome. Sources/inputs remain unchanged; container and daemon
closure checks pass. The retained records distinguish stage, guest, boot,
consumer and native Qt command ownership without double-counting nested copies.

The normal Settings ELF is installed but its main/window is not executed here.
This proves prepared development-image coexistence, reinstall and reboot;
full session/Source, physical phone and hardware readiness remain unqualified.
GPIO attribute readback is not luminance; network counts are not connectivity.

[Hash-only provenance](evidence/native-arm-development-image-provenance.json) keeps
these three executions separate from provider builds and hardware evidence.
