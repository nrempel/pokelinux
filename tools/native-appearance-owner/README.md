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
