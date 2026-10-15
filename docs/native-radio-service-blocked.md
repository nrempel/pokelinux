# Installed service with permanently held registration

The unchanged installed production radio service stays responsive while its
worker's actual Binder registration is held indefinitely by an independent public
peer. Three real status clients complete with `initializing`, revision0, no
failure and no raw history. Normal Stop reaps the worker and exits the service
cleanly; restart from the same journal refuses another worker while the peer is
still alive and held. This is additional integration coverage of the
[installed service](native-radio-service.md), with no manufactured RED or new
production recovery behavior.

## Selected boundary and observations

The public fixture adds one explicit `named-permanent` mode. It registers through
the existing named manager and takes the ordinary production client
`pokelinux-radio` into the existing unconditional Condvar wait loop. It has no
timeout or release control. A flushed marker binds entry to the actual worker
PID and UID; the client name is established by the pinned source branch, not an
invented marker field. No production argv or deadline changes.

The selected service, status client, worker, public manager and installed
unit/sysusers bytes are identical to the normal GREEN selection. Only the public
peer executable changes. Its new source/build closure is separate from the
normal production reference. The guest binds actual post-exec ELF bytes, argv,
parent/principal identities, retained pidfds and unit cgroup membership.

One lookup, one registration and one permanent-hold entry are required before
status sampling. No callback submission, Active observation or Raw observation
occurs. The three distinct status clients each have actual wait0 and bounded
pipe EOF. Together they finish in343.146502ms, including two requested100ms gaps,
inside one original3s window. That aggregate is not a per-query latency bound.
The peer, worker and service remain alive throughout the observations.

Operator Stop precedes the30s production lease expiry. The Rust Supervisor
actually waits SIGKILL9 approximately5.557536ms after its first retirement,
inside its original2s cleanup cutoff. The guest does not signal or reap that
worker. Systemd separately records Main exit0; the previously bound unit cgroup
is empty or removed. The public peer remains alive in the permanent hold.

Restart preserves the same journal and has a distinct systemd invocation with
Main exit78. No new worker, lookup, registration or hold entry occurs. Only after
this refusal does the guest dispose of its exact owned peer and manager, with
actual SIGKILL waits. The whole first-start case takes1948.524462ms inside its
original29s bound. Journal-reader and other direct child waits, actual QEMU/Docker
CLI wait0, fresh exact container absence, same daemon and all frozen inputs close.

Strict checks and all45 host library regressions pass before ARM compilation.
Root independently verifies the source/build/runtime bindings and unchanged
production ELF bytes. See the [bounded provenance](evidence/native-radio-service-blocked-provenance.json).

## What remains separate

The peer still being held after native worker wait is deliberate: local reaping
is not remote client removal, callback drain, request cancellation or vendor
cleanup. There is no retry, journal clear or successor admission. The normal
installed unit's manager fallback, unkillable tasks, blocked discovery, output
pressure, hostile-client pressure and installed principal negatives remain
separate controls. Startup filesystem stalls and actual suspend/wake remain open.

This is public synthetic Binder under a generic ARM kernel. No new original
vendor/Android execution, private REA input, authentication, UI, user-mode ARM
execution or physical phone is selected. Zero physical models are qualified.

The next semantic slice is one registration indication, after recovering its
exact original dispatch contract: callback kind, message ID, slot filter,
complete-payload boundary and selected state meanings. The existing
[original-compared payload decoder](native-ims-registration.md) qualifies fields,
not routing or carrier state. Connect that evidence through the native
owner/worker/service and keep historical decoded observations separate from
availability. Device policy, acknowledged vendor reset/cleanup, native IMS and
full phone hardware remain gates.
