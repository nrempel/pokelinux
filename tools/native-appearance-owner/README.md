# Appearance process boundary control

`process-control.py` tests an exact SHA-selected native appearance ELF in a
disposable, network-disabled x86 Linux container. Run as the container's root with
test-only SYS_PTRACE and fixture provisioning permissions. The qualified run uses
Docker's default14 capabilities plus SYS_PTRACE and no-new-privileges; that is not
the installed production profile.

Invoke `python3 process-control.py --binary /inputs/appearance
--expected-sha256 SHA256 --output /proof` inside that isolated fixture. Preserve
each result under a fresh label, including failures. The host must separately own
the container, actually wait its CLI/container exit and verify exact-CID absence.
This script alone does not qualify host container ownership.

The script traces only its directly launched service and descendants. It holds
the actual child before execve/execveat without rewriting registers or syscall
results, sends real credentialled socket requests, and observes the owner's actual
wait4 return. A separate synthetic root0600 descriptor is deliberately inherited;
after UID1000 and collector filtering it must be absent. No original desktop,
vendor runtime, phone or proprietary input participates. Syscall-register decoding
is x86-specific; do not classify this as ARM execution.

The [bounded evidence](../../docs/evidence/native-appearance-owner-provenance.json)
separates chronological RED, later baseline replays and final GREEN. Raw synthetic
homes, logs and failed labels remain private. Rust Cargo tests separately cover
real process/access controls and same-process real-kernel output backpressure.

The separate `selected-input-control.py` imports this public owned tracer and
holds a fresh collector at the real successful seccomp syscall return, after
input acquisition and before retained content reads. The own fixture root uses
`renameat2(RENAME_EXCHANGE)` to swap two UID1000-owned0600/nlink1 color files
inside the unchanged selected generation. The old inode stays at the staging
pathname; no hardlink or cooperating theme writer is used. Mutation is by the
fixture root, not an observed UID1000 writer.

Invoke it with the same selected ELF arguments plus `--expected-base-sha256`
for `process-control.py` and explicit `--wire-schema 1` or `--wire-schema 2`.
This fixture schema selection is test infrastructure, not a production
compatibility reader. It observes real Job/Completion bytes, packet credentials,
pidfd, ACK and parent wait4 without rewriting syscalls, registers or replies.
A successor is held before exec so its newer collection cannot hide the selected
completion. After release a fresh actual collection must reflect the replacement.

The acceptance assertion refuses an obsolete retained-input publication,
including renewal of its original deadline with unchanged colors/revision. A
failed assertion remains `passed:false`; there is no anticipated-RED conversion.
Preserve setup failures under their own fresh labels. The host still owns exact
container/CLI waits and independent exact-CID absence; no installed desktop,
Original Omarchy, Google kernel or phone behavior is qualified by this control.
