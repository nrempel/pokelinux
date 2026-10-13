This prepares diagnostic-only instrumentation; Root must approve the exact patch before any diagnostic build or execution. It preserves the production Source610aa, candidate78 source files, all GREEN487 proof paths/current authored pins and old RED evidence. Only a fresh diagnostic source snapshot replaces native-session/src/lib.rs with the visibly different diagnostic-session.rs. The other77 candidate core files and the exact candidate provisioner fixture remain unchanged. No production hook, guard, lease, Source frame, app behavior, revival or ptrace is introduced.

The fixed112-byte trace record goes through a nonblocking CLOEXEC AF_UNIX datagram socket to an authored Root-only guest receiver. That receiver sets SO_PASSCRED, bounds each packet/ancillary allocation, and requires exactly the actual service PID/UID0/GID0 SCM_CREDENTIALS, no transferred handles, magic/version/length and monotonic counters. It records receive BOOTTIME independently. Trace setup is mandatory in this diagnostic build and completes before the first real Source lease starts. Trace send happens only after an existing rejection predicate fired or on terminal source stop; failure to send increments a saturating drop counter and never changes an original decision. Every successful heartbeat has extra fixed bookkeeping but no trace write. The setup, bookkeeping and fault tracing can perturb scheduling; the new epoch cannot retroactively diagnose the old failed epoch.

Records use little endian:

| Offset | Field |
| --- | --- |
| 0 | Eight-byte magic PKSDIAG1 |
| 8,10,12 | version u16, category u16, shape flags u32 |
| 16..64 | Six u64: trace BOOTTIME, previous sequence/sample, current sequence/sample, original live-sample examination BOOTTIME |
| 64..88 | Six u32: drain count, signed raw errno bits, packet bytes, rights message/handle counts, recv flags |
| 88..112 | attempted/sent-before-this-record/dropped-before-this-record u64 |

Category0 is one terminal-stop summary. Categories1/2/3 are receive error/drain bound/JSON error. Categories4..12 classify the first original static/frame guard: source, sequence, lock-with-focus, completion-without-lock, bad focus PID, backwards sample, discontinuous/overflowing chain, future own sample, expired/overflowing own sample. The original guard retains its original order and short circuit; classification repeats static fields only after rejection. Examination time is0 when that guard never reached its original live_sample call. Categories13/14 indicate focus/descriptor presence or actual kernel pidfd-type rejection; the original type check is evaluated once and its exact result retained. Category15 is semantic revision overflow. Category16 records malformed ancillary receive shape/flags and counts;17 missing Source credentials. The outer receive-error category can follow16/17, preserving the original error propagation.

Category16 shape bits are empty packet, MSG_TRUNC, MSG_CTRUNC, extra rights handle, unexpected/duplicate ancillary, multiple rights messages in bits0..5. Categories13/14 use bits0..3 for focus present, peer handle present, type examined, correctly typed. No descriptor is re-opened/re-targeted or inspected for liveness by tracing. The kernel-owned Source/focus handles and their original validation remain unchanged.

The receiver must observe a final category0 summary and reconcile attempted with received records and dropped counters. A dropped/untrusted/malformed trace or missing summary disqualifies exact failure diagnosis. Actual rejection fields qualify only that new diagnostic epoch. If a particular failure does not recur, the earlier failed aggregate remains unexplained and failed; no green relabel or deadline margin change follows.
