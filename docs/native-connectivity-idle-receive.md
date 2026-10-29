# Persistent connectivity idle receive

A deterministic native x86_64 host fixture reproduces retirement of the normal
persistent worker during empty idle polling. Its unchanged 50 ms receive guard
starts before full parent authentication, although the channel receive itself
is nonblocking. A fixture-owned 80 ms stop before the authentication thread check
therefore expires that guard without a private receive or effect request.
The final desired assertion fails after all owned cleanup. A separate consumed
frame control and actual parent-death control both pass.

This is a controlled host reproduction, separate from the earlier failed ARM
Settings runs. Their error context measures a guard span, without establishing
the receive result, which guard fired, syscall duration or scheduling cause.
The earlier failed ARM evidence and unknown cause remain unchanged in the
[error-context report](native-connectivity-worker-error-context.md).

## Boundary and change

`BoundParent::message_ready` performs a zero-time joint poll of its retained
channel and both parent pidfds. Existing local main-thread, UID/GID, PPID and
PDEATHSIG checks bracket that poll. Parent readiness, channel hangup or any
unexpected event is terminal, including hangup with a queued record. The hint
copies no payload, authenticates no record and starts no command deadline.

The persistent worker constructs its existing 50 ms envelope only after a readable
hint. Its existing `receive_message` still brackets nonblocking `recvmsg` with
the full parent credentials, retained pidfd/procfs identity and BOOTTIME checks.
An actual consumed frame that exceeds the original envelope remains terminal;
no timeout becomes `None`, and the original action cutoff is not renewed.
Observation readback, refresh, lease, retirement, Stop and mutation uncertainty
are unchanged. Main's receive path is outside this change.

Two socket/pidfd tests prove that repeated readiness hints leave a queued
SEQPACKET record intact and that hangup with queued data is refused without
consuming it. The affected process-owner and connectivity scopes pass 30 active
tests, with four existing ignored selections, formatting and strict Clippy.
Four normal host/ARM Release providers build with opt-level 3 and no features.
The separate actual host pair below qualifies the candidate integration.

## Actual host selection

The qualified RED uses the unchanged normal Main and worker, fresh disposable
Docker network namespaces and the existing retained peer-namespace keeper.
Main irreversibly drops its capabilities; only its normal worker retains
NET_ADMIN. The fixture's additional namespace/ptrace capabilities do not belong
to either production provider. No physical interface, phone, VM, Settings app,
packet payload or process memory is inspected.

The tracer selects only the actual single-threaded worker child of that owned
Main, retaining its pidfd, executable identity and original parent. It uses
syscall metadata and nonterminal trace stops, then detaches before Main's actual
terminal wait. It never changes syscall results, reads payload bytes or
substitutes a tracer wait for the normal parent's terminal wait.

| Case | Actual result |
| --- | --- |
| Baseline empty idle | One 80 ms stop follows an actual 47-byte observation and loop sleep. No effect request or later private receive occurs; exact `ReceiveCommand`/`TimedOut` is observed. The desired same-worker/fresh predicate fails. The original qualified RED's final wait is signal 9; the revised baseline naturally exits 1. |
| Candidate empty idle | The same worker remains live after the identical stop and publishes fresh kernel readback, with no effect request or private receive. The desired persistence predicate passes. Explicit cleanup later stops that worker; it is not claimed to exit naturally. |
| Consumed command, both providers | One actual 19-byte receive precedes its 50 ms envelope cutoff. After an 80 ms stop, resume and worker detection exceed that cutoff but precede the unchanged 100 ms Main action cutoff. Worker exits naturally 1; independent address readback remains empty, without effect retry. |
| Parent death, both providers | Explicit SIGKILL of owned Main is actually waited with output EOF. The exact known adopted worker is independently waited with signal 9, with no ordinary Main-worker wait claim. |

All three container IDs/names are absent after actual Docker wait and removal.
The 54 direct Scope waits/PIPE EOFs per cohort comprise 39 outer, nine inner and six
controller waits; two normal Main worker wait/EOF reports and one explicitly
known adopted-worker wait are separate witnesses. Retained namespace handles,
same-daemon checks, unchanged selected inputs, original bounds and ECHILD all
qualify before the one final desired assertion.

The prior 120 ms consumed-control epoch remains failed: its hold also exceeded
the 100 ms action cutoff, and a signal 9 wait did not prove worker post-receive
validation. Earlier missing-container text and missing Python runtime outcomes
are setup failures. None is relabeled as the qualified three-case RED.

The first candidate cohort remains failed even though its isolated idle case
passes: the consumed fixture submitted Apply only after a private `recvmsg`
entry, while the new idle hint correctly waits for a readable channel. No
request was submitted and the unchanged 8192 syscall-stop bound was exhausted.
Cleanup qualified; that fixture ordering failure is not a production RED or a
whole-cohort GREEN.

A fresh baseline/candidate pair now passes its RED-to-GREEN qualification. Both use the same
revised tracer, submitting the one Apply at the existing post-publish/post-sleep
`gettid` stop, before either the old authenticated receive or the new readiness
check. The actual 19-byte receive-exit 80 ms hold, pure consumer, controller,
original cutoffs and both controls are unchanged. The candidate replaces only
the normal worker, retaining exact baseline Main. Actual baseline requalification
closed before the candidate was admitted and executed. The baseline's final
desired assertion fails, while the candidate's one final assertion passes after
all three cases and cleanup. The consumer, controller and all seven runner files
are byte-identical across this final pair; only the normal worker provider and
the outer expected-outcome aggregation differ.

The [exact fixture sources](../tools/native-connectivity-worker/README.md#persistent-idle-readiness-consumer)
and [hash-only evidence](evidence/native-connectivity-idle-receive-provenance.json)
bind the source, formatted build inputs, normal providers, retained failures and
closed results. Normal ARM cross-compilation remains separate: this host proof
does not execute the candidate persistent ARM loop. Existing ARM one-shot
fixed-address/packet evidence remains unchanged. No Settings, ARM persistence,
wireless ownership, real user consent or Internet claim follows from this
investigation.
