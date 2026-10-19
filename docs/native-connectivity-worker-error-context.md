# Native worker error context

The normal connectivity worker now preserves operation-stage context on errors
after an actual missing-context CLI RED. The unchanged frozen consumer passes
with `ReduceCapabilities`, `original_until_ns: None`, an error-only BOOTTIME
detection sample and the original source errno1/`PermissionDenied`. The worker
still exits naturally with code1; the GREEN consumer exits0.

## What changed

Only `native-connectivity-control/src/service.rs` changes production behavior at
error reporting: a private `WorkerFailure` wraps the original `io::Error`, keeps
its outer kind and exposes the source chain. The outer custom error's
`raw_os_error()` is not claimed unchanged. The detection clock is sampled only
when a selected call fails. Seven compiled contexts are `ReduceCapabilities`,
`BindParent`, `PublishInitial`, `ReceiveCommand`, `PublishRetirement`,
`PublishCompletion` and `PublishObservation`. Only the first executes in this
control. Kernel error mappings, validation,47/19-byte records, control flow,
cutoffs, retries and retirement remain unchanged.

## Actual evidence

| Epoch | Qualified result |
| --- | --- |
| Baseline build |35 active host tests:10 connectivity,10 process-owner and15 kernel-identity;7 ignored. Network-state compiles only. Four normal host/ARM Release ELFs build without execution. |
| CLI RED | Actual UID1000 parent/child, all capability sets0 and NNP1. Normal worker returns exact unwrapped EPERM and naturally exits1. The consumer's desired assertion fails after closure. |
| Candidate build |10 connectivity tests pass,3 ignored; unchanged shared QA is inherited from baseline. Four current normal host/ARM Release ELFs build without execution. |
| CLI GREEN | Same8290-byte consumer and actual cap0 boundary. Context contains `ReduceCapabilities`, no cutoff yet and detection within actual spawn/wait. Original EPERM and natural worker exit1 remain. |

Both CLI controls qualify one actual direct-child wait and merged stdout/stderr
PIPE EOF under the original1s cutoff,4096-byte output bound, closed namespace
handle, closed Scope handles and ECHILD. The separate3s custody cutoff never
renews functional qualification. Child process/task metadata is sampled before
parent poll/wait; a zombie sample is not a running claim. The helper, raw result
and normal artifact hashes are recorded in the
[curated provenance](evidence/native-connectivity-worker-error-context-provenance.json).
The two build envelopes close16 Scope waits/PIPE EOFs each;31 baseline and22
candidate foreground-shell regular-output waits are recorded separately.

An extra candidate validation assertion initially omitted generated
`source.sha256`; that tool assertion fault is preserved. Frozen pins were
verified before launch, actual build pins passed, and the independent source
comparison was corrected after the build. It supplies no product RED.

## Reuse and limits

The [exact public consumer](../tools/native-connectivity-worker/error-context/consumer.py)
uses the existing cooperative selector and four trusted arguments. Its
[invocation contract](../tools/native-connectivity-worker/README.md#normal-cli-error-context-consumer)
requires the original owned/closure helpers beside it and the qualified normal
Release provider. No special worker flag, capability or network setup is added.

This is native x86_64 error-reporting evidence. It qualifies no virtual-link
effect, other-stage execution, ARM error context or phone. The failed
[ARM window trace](native-settings-connectivity-window.md#fourth-attempt-partial-touch-and-packets)
retains its original failure and unknown timeout stage. One contextual ARM
diagnostic is the next selected tracer; no ARM cause or fix follows from this
CLI pass.
