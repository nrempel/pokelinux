# Native ARM Settings preparation helpers

These are the byte-identical original helper functions from the native ARM
preparation cohort recorded in
[native Settings connectivity](../../../docs/native-settings-connectivity-window.md#native-arm-build-and-installation).
They are source components, not a newly measured public VM launcher or image.
Their historical proposal/unexecuted docstrings describe the pre-run drafts;
the recorded closed cohort subsequently executed these exact bytes.

`native_arm_settings_build.py` accepts the source/three-archive selection and
existing bounded `run(argv, label, ...)` owner described by the parent fixture.
It requires guest root on AArch64, builds only the unchanged normal Settings
CMake Release target with explicit archives/hashes and `-fPIC`, checks the
native loader and stages exactly three production CMake rows in a DESTDIR.
It returns the generated app identity and staging proof. It executes no app,
component, compositor, connectivity Main or worker.

`guest_install_settings.py` accepts that exact generated row manifest and
DESTDIR. It requires an AArch64 systemd guest, root credentials and protected
root-owned destination ancestry. It authenticates all three rows before guest
installation, checks the installed hashes/modes and activates no service.
Call it only in an owned disposable guest, never against a live host root.

The actual envelope separately installs the qualified ARM compositor and its whole 498-file
SDK, builds the existing profile/capture helpers against that SDK and checks
all native loaders. The 393-package inventory and 50 selected Qt files stays unchanged.
Guest child subreaping, original cutoffs, actual waits/stdout EOFs and QEMU
closure precede image export. The source bundle does not distribute the image,
SDK metadata snapshots, provider binaries or private receipts.


`denied_peer.py` is the byte-identical minimal actual UID11050 private-socket
probe from the third failed ARM window epoch. It runs only as an owned child in
a disposable fixture, with an absolute CLOCK_BOOTTIME cutoff supplied by its
parent. It sets no-new-privileges, drops supplementary groups/UID/GID, checks
actual credentials/capabilities and attempts a real SEQPACKET connection.
Only EACCES qualifies; up to three bounded flushed markers precede one final
JSON result. The parent separately owns actual wait and output EOF under the
same original cutoff. Its host private/accessible controls and ARM denial pass;
the later window/worker guard still fails. This is test infrastructure, not a
production permission service or a newly qualified public VM launcher.
See the [preserved ARM failures](../../../docs/native-settings-connectivity-window.md#arm-window-attempts-remain-failed).
