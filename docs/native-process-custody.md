# Shared native process custody

`native-process-owner` now holds the existing cooperative bootstrap, one-child
supervisor and absolute boot-time deadline mechanism. Radio production and
fixture callers import it directly. This is a behavior-preserving extraction:
67 selected host tests and an additional actual generic ARM production-worker
regression pass, with no new behavioral RED.
[Summarized proof](evidence/native-process-custody-provenance.json).

The three former radio-local modules are removed. `WorkerCommand` uses the
single `--pokelinux-process-worker` selector; durable refusal is
`StartError::AdmissionUncertain`. Explicit byte/hex identity codecs replace
access to private representation. Those codecs are data conversion, not
custody, permission or publication authority. See the
[crate contract](../native-process-owner/README.md).

The final build freezes 96 source files, including six shared-crate files;
the original extraction preparation separately records 93 tracked inputs.
Independent review finds six new, 25 updated and three removed paths, with
existing dependency versions unchanged.
Strict formatting/Clippy and ARM production/fixture compilation pass. Actual
host execution passes eight shared and 59 radio tests; two radio tests remain
ignored. Compilation is not an execution claim for every built ARM binary.

The new ARM run uses a public synthetic manager/peer and the current production
radio worker under the generic Arch 7.1.6 kernel. Exact PID/PPid, ELF, argv,
UID/GID 1000, empty supplementary groups and zero effective capabilities bind the
worker before the incarnation ACK. It performs one named lookup and one
registration, receives the real callback as RAW slot 0 / serial 7 / message 137 with
one byte `07`, then observes actual peer control 107 death and ServiceDied.
Stop revokes admission; the Rust supervisor actually waits its SIGKILL worker,
fences stale output and refuses both the same and recreated owner against the
unchanged journal. The guest is not a second worker reaper.

Four guest direct-child waits qualify before their original CLOCK_BOOTTIME
cutoffs: binderfs mount exit 0, client exit 0, peer exit 0 and manager signal 9. These are bounded
regular-file logs, not pipe-EOF evidence. The original RAW/terminal/cleanup
budgets remain 1/2/3 seconds. Normal powerdown is shown by the console and outer
CLI exit 0/exact-CID closure, without a separate BOOTTIME poweroff wait receipt.
Owned live CID observation, actual CLI wait, fresh exact absence, same daemon
and independent selected/original input rehash checks all pass.

Preparation failures remain distinct: build1 omitted its derived script before
Docker invocation; build2 hit private identity access during compilation;
build3 passed but had no selected ARM runtime because its inherited printed
worker argv omitted the actual `--slot 0`. Source3/build4 correct only that
fixture witness before execution. None is a behavioral RED.

Separate tooling maintenance retires the historical provenance writer: actual
exit 1 occurs before artifact I/O and its old public proof stays byte-identical.
The containment collector adds shared/transitive source roots and drops stale
commit/binary-build claims; that collector received AST/static review only.
Its new hash is inventoried separately from the old collector pinned in the
96-file runtime snapshot, where the tool was not executed.

The earlier [parent-death bootstrap](native-radio-bootstrap.md),
[four policy controls](native-radio-bootstrap-policy.md), permanently blocked
registration and installed-service runs keep their original sources and scope;
they were not rerun by this historical Radio selection, which executed no Data
worker/service. Its APN, vendor cleanup, safe-successor and hardware limits are
unchanged. A later separately pinned Data selection is described below.

## Later Data consumer

A host actual failed-spawn admission-receipt RED→GREEN passes nine shared tests
and exposes readonly metadata for returned file/directory sync before spawn.
This does not simulate power loss or clear the admission fence. The historical
eight-shared/59-radio proof above remains byte-for-byte separate.

The [Data service tracer](native-cellular-service.md) then runs the three unchanged final-core
production executables through actual generic ARM Binder and an independently
controlled TUN. Main actually waits its worker9 before guest-owned Main0, and
unchanged-journal restart78 starts no new worker/setup. All34 guest direct
waits/EOF are separately counted from that nested native wait. No Binder pool
join, remote provider cleanup, safe successor operation, permission/APN grant
or normal systemd installation is inferred; manager fallback remains untested.

## Later stdout EOF custody repair

A separate host regression retains a duplicate of a real child's stdout writer
after that child exits. The original owner actually waits the child but returns
its status and releases capacity while the pipe still returns `EAGAIN`; the
compiled positive assertion fails101 after cleanup. The exact same frozen test
passes after the shared owner caches wait status and retains stdout until a
nonempty-buffer read witnesses EOF. Publication is revoked at actual wait;
repeated Stop cannot signal the reaped PID. Each poll drains at most4096 stale
bytes, with nonblocking mode enforced even after an initial setup error.

The nine existing Supervisor unit controls also pass, alongside production-file
formatting and strict all-target Clippy. The GREEN has19 direct Scope waits and
19 stdout PIPE EOFs (13 host plus six inside), with two regular Cargo stderr
handle closures counted separately. The child has one additional native wait,
independently witnessed by `ECHILD`, and actual stdout EOF; those are not extra
Scope receipts. Both scopes close with `ECHILD`, no adopted waits, unchanged
inputs and exact container absence.

[Summarized proof](evidence/native-process-stdout-eof-provenance.json). These
results are host-only and separate from all earlier Radio/ARM/Data evidence in
this report. Neither local EOF nor wait proves remote cleanup, clears the
admission marker or authorizes retry. A separate token/literal-checked formatter bridge preserves all predicates and cutoffs. The published formatted fixture also passes the real pipe test and the same nine controls, full-crate formatting and strict Clippy; its current-source receipts remain separate from the earlier frozen test.
