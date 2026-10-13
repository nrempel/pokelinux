# Isolated native session → current Messages authority fixture

This fixture runs the real lifecycle actor, native session Observer, native
Messages storage worker and SQLite projection in an owned Linux container. Its
facts source is a separately launched synthetic UID1000 process, not Hyprland.
The Messages client is the actual installed app launched as UID11010. The fixture
never calls manual root unlock/background controls to establish a positive read.
Root policy grants remain separate from source facts.

A root-created inherited `SOCK_SEQPACKET` channel carries kernel
`SCM_CREDENTIALS`; the creation-time peer credentials are recorded separately.
The installer transfers the source socket and an independently owned source pidfd
only to the fixture service. The actor gets a real Observer Provider. Each source
focus names the actual launched app's socket-derived `SO_PEERPIDFD` device/inode,
checked against a separately opened live app pidfd. No hard-coded PID or synthetic
pidfs identity is used for positive authority.

The fixture service exists only behind `isolated-fixture` in this separate crate.
Its inherited descriptor arguments do not add a selector to production
`messages-service`. Worker entry points dispatch to the existing native workers.

Run from the checkout with the already provisioned local Docker image and offline
Cargo cache:

```sh
python3 tools/native-session-authority/runner.py --phase red --label red-proof
python3 tools/native-session-authority/runner.py --phase green --label green-proof
```

Use `--case <case_name>` repeatedly to select cases. Labels are immutable:
existing evidence is preserved and refuses overwrite. A run owns a uniquely named
container only after successful creation, disables network access, and removes
that container in a `finally` block. Child processes remain owned until their
actual wait result is observed; service-owned app disappearance is checked.

The red selection freezes exact commit
`f733d3b937ab13fa9eb8d81bcf76ab72d0dcf184` for lifecycle authority/service and
Messages dependencies. Current session Observer code is separately selected.
Only shared `session` type declarations may be overlaid to compile the current
Observer; the old production authority/service bytes remain unchanged.
`baseline-ignored` constructs the real credential/pidfd-bound Observer but calls
the original `run_with_messages` entry, which has no session ingress. This is an
executed missing-integration baseline, not a mocked Observer or production shim.
The same assertions and actual processes run through
`run_with_messages_and_session` for green.

Cases cover unknown-source denial even after manual root controls; source-only
focus/unlock plus separately granted reads; null/ineligible focus and exact app
PID/UID/GID/pidfs matching; a lock→unlock→refocus burst queued while the actor is
actually stopped; same-state heartbeat preservation within an original read
lease; heartbeat refusal to extend that lease; actual source SIGSTOP, EOF and
SIGKILL with responsive snapshots/stop; real reply-queue EAGAIN retries; sticky
root lock intent; and a newly launched app's own identity plus policy grant.

`artifacts/native-session-authority/<label>/` contains frozen source hashes,
selected entry/features and image identity, offline build log, exact executed ELF
hashes, execution log, per-case real identities/timestamps/reap observations,
result JSON and proof hashes. Raw local-history content is not copied into the
publishable report. Build/setup failures are retained and cannot be described as
executed red cases.

These results qualify synthetic local process/socket authority ingress. They do
not qualify the original compositor/Quickshell protocols, actual layer/null
keyboard-focus observation, trusted production PAM unlocking, source code/config
containment, zero publication after an unobserved external transition, phone
hardware, modem behavior, or physical suspend. A `focus: null` packet here is an
ineligible synthetic source fact; original compositor evidence is separate.

The host Docker kernel is Linux6.8 and exposes `SO_PEERPIDFD` through shared
`anon_inode` metadata. Its pidfds cannot establish the required distinct pidfs
identity. That host lane qualifies missing-ingress red behavior and explicit
unsupported-platform denial only. Positive authority and wrong-pidfs separation
run in this fixture's own ARM QEMU system guest using Linux7.1.6-1-aarch64-ARCH:

```sh
python3 tools/native-session-authority/runner.py --phase red --label arm-red-build --target aarch64-unknown-linux-gnu --build-only
python3 tools/native-session-authority/vm.py --build arm-red-build --label arm-red-authority --case unknown_denied --case root_controls_unknown_denied --case source_only_read
python3 tools/native-session-authority/runner.py --phase green --label arm-green-build --target aarch64-unknown-linux-gnu --build-only
python3 tools/native-session-authority/vm.py --build arm-green-build --label arm-green-authority
```

The VM runner owns a separate guest clone, lock, container and evidence directory.
The compositor fixture's VM is untouched. It records the selected kernel/initrd,
root image, QEMU command and image/container identities, validates every selected
source/ELF hash inside the guest, requires actual `PID_FS_MAGIC`, and powers off
after child reap evidence is saved. This is ARM64 system emulation with the actual
guest kernel; user-mode QEMU is insufficient for this boundary.

Preserved initial authority red proof: `artifacts/native-session-authority/arm-red-proven/` executes the two missing-ingress failures on the actual ARM kernel, with default unknown denial passing and all child reaps recorded. The earlier host `red-authority/` proof records the same old behavior, but does not qualify distinct pidfs identity. Setup-only failures are retained in separate labels and excluded from behavioral evidence.

Final green proof: `artifacts/native-session-authority/arm-green-authority-final/` passes all13 groups on Linux7.1.6-1-aarch64-ARCH with exact real pidfs identities. The three host denial groups pass at `host-green-unsupported/`; that run explicitly reports `pidfs_identity_qualified: false`. The first ARM green label preserves12 passes plus the fixture reap-wait failure; the final label corrects only that bounded wait and records explicit owned worker/app pidfd death and process disappearance. Production authority source hashes are identical between those two green builds. [observed-evidence.json](observed-evidence.json) indexes selected source/ELF/result/proof hashes and states the evidence limits.
