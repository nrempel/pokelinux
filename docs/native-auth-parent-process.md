# Native authentication parent process boundary

This report records the historical41-test selection committed as `baf7b79`.
Later PKSCOPE4 behavior and results are in [closed outcomes](native-auth-closed-outcome.md).

The actual Rust ScopeChild now passes a host prompt/response regression against
one owned ordinary peer process. The complete host suite passes41 tests, with
formatting and strict default/all-target/all-feature Clippy. This adds a positive
integration test; production behavior and constructors are unchanged. The earlier
[record/command REDs](native-auth-parent-forwarding.md) remain separate.
[Bounded provenance](evidence/native-auth-parent-process-provenance.json) binds
the executed test ELF, source, log and independent source review.

The test owns a real Child, retained pidfd and nonblocking private pipes before
fallible setup. Its peer is the exact test executable with a cleared environment
and exact ignored-test filter; it never forks. A fixed marker and explicit barrier
keep libtest text outside the binary record stream. ScopeChild is then constructed
directly in cfg-test code, without changing or exercising root/provisioning gates.

Production poll accepts a ready state and matching prompt; pending_hidden_prompt,
scope_snapshot and respond_hidden_prompt observe and answer it. The peer captures
the expected-length frame through its real stdin into a0700 directory/0600 file,
containing known public dummy input only. The parent compares that prefix after
actual peer exit. This test alone does not exclude extra trailing or duplicate
outbound bytes; the earlier direct queue completion controls remain separate.

The peer emits synthetic worker Wait/State/CLOSED records. Production parent poll
also performs its actual peer try-wait, observes actual stdout EOF and reconciles
bookkeeping. The test requires natural wait0, retained pidfd death, proc absence,
no fault/retirement and occupied=false. Worker PID100, worker wait0, ECHILD,
worker EOF and populated0 are synthetic wire facts. Parent occupied=false in this
selection does not establish an actual authentication workload or cgroup closure.

Ordinary failures are retained as values until peer cleanup. Admission failure
closes input; bounded production polling continues actual wait/EOF. Failure-only
exact kill/wait cannot satisfy the natural-success assertion. After actual closure,
residual quarantine aborts the aggregate rather than clearing fault or occupied
flags. Unrecoverable kernel failures preventing emergency wait/EOF remain failed
infrastructure, not cleanup evidence. No blocked worker is detached.

Both ignored helper entries remain explicit and are invoked by their owning
parent tests. All21 frozen source bindings match; Root checks the executed host
test ELF, source review and fresh exact-container absence on the same daemon.
The container actually exits0. An ARM64 fixture also compiles, without execution.
The [earlier actual ARM40 suite](native-auth-components-arm.md) retains its earlier
source and cannot qualify this new process regression.

No actual Attempt, authentication worker, supervisor fixture loop, original PAM
or effective faillock executes. Full owner composition, broker/actor result and
receipt, exact-object unlock, synced durable clear and normal Messages reopening
remain gates. The crate stays isolated-feature-only. Matched Google, secure frames
and physical-device qualification remain separate. No phone was accessed.
