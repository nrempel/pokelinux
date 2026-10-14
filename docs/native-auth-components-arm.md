# Native authentication components on an ARM64 kernel

The unchanged40 library components from `7850115` pass under an actual Arch Linux
ARM kernel `7.1.6-1-aarch64-ARCH`, as UID/GID1000 with no supplementary groups and
zero effective capabilities. This is QEMU system emulation, separately qualified
from the [host forwarding selection](native-auth-parent-forwarding.md), ARM
compilation and ARM user-mode emulation. The complete parent/supervisor/PAM path
does not run. See [bounded provenance](evidence/native-auth-components-arm-provenance.json).

Only the AArch64 library test ELF is crosscompiled and selected. Its actual ELF
header is64-bit little-endian machine183, and the guest checks the retained ELF
and all20 frozen source bindings before execution. The test list contains41
entries. The one ignored setup-helper entry is explicitly listed and invoked by
its owning parent regression; it is not silently excluded. All40 ordinary
components pass, with0 failures and1 ignored entry in the parent summary.
No Rust source, original deadline or test-thread setting changes for ARM.

The ordinary helper exits0 naturally; its parent actually waits, observes stdout
EOF, a dead pidfd and proc disappearance. The pipe components also exercise prompt
records, two parent queue/command rounds, fragmented frames, real EAGAIN and
cancellation, calls begun after expiry and the unchanged original Rust wire.
Worker/cgroup fields in pure protocol controls remain synthetic facts. They do
not establish actual workload admission, recursive cleanup or authentication.

The guest adapter drops groups/GID/UID directly before exec, without a PAM session
launcher. Three actual test-process waits0 close listing, ignored listing and the
suite. QEMU naturally exits0 in25.208s inside its original360s bound. The CLI and
container waits also return0; fresh checks on the same selected daemon confirm
the exact container absent. Root independently rehashes the frozen source, ELF,
adapters, kernel/initrd, untouched base, private clone and retained evidence.
The guest has loopback only; no NIC or physical device is attached.

Earlier preparation labels remain separate: the first library compile is followed
by missing image Python in artifact postprocessing; the next compile succeeds
but host readelf is absent. A fresh build parses the standard ELF header directly.
The first QEMU launch exits1 before boot because snapshot scratch selected
read-only `/var/tmp`. A fresh container adds bounded temporary storage and
`TMPDIR=/tmp`, preserving the same immutable payload and360s bound. Those earlier
labels execute zero tests and do not become passing runtime evidence.

No auth-scope fixture binary, authentication worker, original PAM/helper,
ScopeChild/Attempt or actual cgroup workload executes. Broker/actor result and
receipt, exact-object unlock, synced durable clear and normal Messages reopening
remain open. Calls begun after expiry do not prove an atomic cutoff across
syscalls or physical suspend behavior. The matched Google kernel, secure frames
and every physical-device gate remain separate. No phone was accessed.
