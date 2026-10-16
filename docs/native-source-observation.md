# Observed Source status

The existing root-admin Snapshot now includes bounded Source observation history.
It lets an operator distinguish a Source that has never supplied an accepted
sample from an observed failure before cleanup. It does not repair or diagnose
the preceding [failed original-compositor replay](native-session-owner.md#gated-original-compositor-replay).

## Root snapshot contract

The `source` object contains three fields:

- `state`: `not_configured`, `awaiting_first`, `observed`, `failed` or `stopping`.
- `last_accepted`: null or the last accepted `sequence` and `sampled_at_ns`.
- `first_failure`: null or `category`, `error_kind` and
  `observed_at_boottime_ns`.

The last sample metadata updates only at Authority's actual acceptance point.
A Provider returning a sample, or an Authority call returning Ok, does not alone
establish acceptance: rejection can return Ok, and reconciliation can ignore a
sample. `observed` records past acceptance; it does not guarantee current health,
identity, freshness or permission. Existing `ready` continues to report
policy/storage readiness.

Failure categories distinguish `provider_error`, `authority_rejected` and
`missing_after_established`. Provider errors retain a static bounded ErrorKind
label; the other categories have null `error_kind`. Unmatched std ErrorKind
variants use explicit `Unclassified`, distinct from actual `Other`. No arbitrary
error message or OS text is exported. The timestamp records the actor's first
observation in BOOTTIME, not the producer's fault time or an established cause.

The first failure and accepted sample history survive later errors and Stop.
Intentional Stop without an earlier failure marks `stopping`; cleanup's later
NotConnected cannot manufacture a failure. The actor can exit before another
Snapshot, so post-Stop inspection is not promised. Only two sample integers and
one first-failure record are retained. No raw packet, Source identifier, focus,
PID or descriptor is added to this root-only reply.

Provider, SourceOwner, Observer, packet/bootstrap, grant and lock decisions stay
unchanged. The100ms sample lease,120s maximum startup allowance and terminal
Source refusal remain in force. Observation adds no retries, replacement, revival,
poll logging or new endpoint.

## Executed evidence

Eight fixed host cases first fail at the missing Source field, then pass with
unchanged tests after the observation change. Four use actual credentialed Linux
SEQPACKET admin IPC, production `run_with_services` and native storage code with
an authored UID1000 producer through the real SourceOwner/Observer. They cover
no Provider, awaiting-first/original timeout, accepted Source followed by a real
SIGSTOP/lease expiry, and actual child KILL/disconnect. Early replies are retained
and real controls complete before the missing-field assertions. RED establishes
the missing observation, without claiming its deeper new-value assertions ran.

Four separate local typed Authority/Provider cases cover rejection, missing after
establishment, ignored reconciliation and intentional Stop. They are not actual
producer or post-Stop IPC evidence. The neutral extraction of existing intentional
stop calls receives no feature-RED claim.

Each epoch records four actual actor waits0 with both PIPE EOFs and three actual
Source waits9 before original8s custody cutoffs. Source streams are DEVNULL,
without PIPE EOF claims. The two test binaries and ten outer CLI children have
applicable merged-output PIPE EOFs and actual waits before their original bounds;
controller ECHILD, same-daemon exact container absence and selected-source closure
also pass. Strict host formatting, Clippy and focused native compilation pass.
Compile/runtime preparation failures remain setup failures, separate from RED.

[Hash-only provenance](evidence/native-source-observation-provenance.json) binds
the eight code/test paths, exact native artifacts, fixed tests and sealed authored
and independent receipts. Private raw logs and binaries are not published.

## Remaining execution

The matching default production owner and fixed launch client compile on host
and ARM64 with strict formatting/Clippy. The current default lifecycle library
regression passes68 tests with20 explicitly ignored cases unrun. Its first runner
attempt passes49/fails19 on a missing isolated directory; correcting only runner
setup and selecting the built storage helper yields68 passes. That setup failure
is preserved separately from feature RED. Neither compilation nor these host
checks establish original-compositor execution.

The [matching generic ARM replay](native-session-owner.md#production-source-observation-arm-replay)
now retains actual authenticated Source status before Stop: initial accepted
sequence2, then accepted sequence37 followed by provider TimedOut at114.24ms
sample age against100ms. The extra fullscreen metadata probe exits255 before
Sounds/PCM; the gap's cause remains unknown. This is separate runtime evidence
for the observation contract, without functional GUI qualification. On failure,
obtain a bounded authenticated Snapshot before cleanup Stop when possible,
within the original observation cutoff. Its fields remain observations rather
than a crash cause or health guarantee. No new original Hyprland runtime, normal
GUI/PCM, ARM runtime or physical-phone qualification follows from this host cohort.

The later [startup-event witness replay](native-session-owner.md#startup-event-witness-replay)
retains the same root observation contract and a separate13-landmark diagnostic
prefix. All observed completion landmarks precede Source sequence50 by over1.35s;
provider TimedOut still occurs before Sounds/PCM. No startup-gate repair or cause
follows, and the full functional assembly remains unqualified.
