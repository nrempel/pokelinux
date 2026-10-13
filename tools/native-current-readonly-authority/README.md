# Current readonly-process authority fixture

This versioned wrapper runs the current split Messages writer/readonly-worker
architecture through actual host Linux process, seqpacket and pidfd boundaries.
It imports the archived `tools/native-session-authority/runner.py` assertions;
that archived file and its failed results remain unchanged.

The wrapper selects the actor's exact `--messages-read-worker` child, retains its
pidfd, checks its parent and observes actual SIGSTOP/T-state. It also retains the
owned actor pidfd for the original coalesced lock/unlock/refocus case. The source
is explicitly synthetic UID1000 SCM_CREDENTIALS/focused-rights test infrastructure;
the actual Rust Observer610aa and actor execute. No original compositor/adapter,
incoming Binder, ARM kernel or physical-phone qualification follows.

The selected evidence qualifies15 distinct current groups:14 v1 passes and one
narrow v2 pass. Heartbeat/held-read and EAGAIN assertions preserve the original
100ms lease. Source expiry/EOF/death require retained app pidfd death, process
absence, zero principals and refusal of an explicit UID1000 successor launch;
the actor remains live and authenticated Snapshot/Stop respond.

The old suite remains11pass/4fail. It held only the writer, and its received page
130ms later has no recorded successful service-send instant; it cannot prove
publication after expiry or qualify a held reader. Its three old terminal cases
expected a driver reply after the current required app retirement. The new v1
has14passes/one fixture error: it lacked the actor signal handle before the
coalesced body. Only that case reruns in corrected v2. Neither failed aggregate
is relabelled successful, and no production source or deadline changed.

Root independently verifies443 private bindings, exact current runtime sources,
binaries and wrapper bytes in the [public provenance](../../docs/evidence/native-current-readonly-authority-provenance.json).
The executed v2 command selects `--case held_semantic_burst`; the wrapper's default
full15 list is not claimed to have run as one passing v2 aggregate. The separately
published27 lifecycle/storage/cleanup groups are not counted again here.

Run only inside the isolated Root fixture realm with
`NATIVE_MESSAGES_ISOLATED=1`, readonly source at `/source`, selected binaries at
`/bins` and a fresh private `/results`. The inherited fixture owns, stops and
actually waits its processes. This wrapper is not an installed service manager,
original-compositor source admission or a remote cleanup receipt.
