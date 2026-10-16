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
  `observed_at_boottime_ns`, plus nullable `observer_rejection`.

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
Snapshot, so post-Stop inspection is not promised. Only bounded sample metadata and
one first-failure record are retained. No raw packet, Source identifier, focus,
PID or descriptor is added to this root-only reply.

When a Provider owns the real Observer, `observer_rejection` retains its first
terminal decision: typed `stage` and `reason`, one-based `drain_ordinal`, optional
parsed `sequence`/`sampled_at_ns`, `previous_accepted`,
`validated_at_boottime_ns` and optional `receive` facts. Ordinal zero means a
liveness or cached-sample decision outside packet draining. `previous_accepted`
means the Observer's preceding accepted frame. It can advance within a drain
that later fails, while the actor's `last_accepted` remains at its actual
Authority acceptance point.

The validation timestamp is the exact single BOOTTIME used by that frame's
freshness check when reached. For an earlier predicate, such as sequence or
sample-chain gap rejection, it is the failure observation time; no freshness
check is implied. Receive facts contain only the returned byte count and
payload/control truncation flags, or a numeric syscall errno. Fields unavailable
on syscall error are null. `receive_zero_bytes` means the receive returned zero
bytes: both a known socket-close fixture and an authored zero-length SEQPACKET
produce this fact, so it alone does not establish EOF.

Observer keeps its first rejection across repeated poll, snapshot and Stop.
SourceOwner exposes that retained record, and the actor copies it before its
existing provider Stop. Intentional Stop creates no rejection. Providers without
an Observer, owner startup/death failures preceding an Observer decision, and
Authority rejection may have null `observer_rejection`. Neither this record nor
its timestamp establishes a producer crash cause. No packet bytes, parser text,
Source identifier, focus or peer identity is retained.

Provider, SourceOwner, Observer, packet/bootstrap, grant and lock decisions stay
unchanged. The100ms sample lease,120s maximum startup allowance and terminal
Source refusal remain in force. Observation adds no retries, replacement, revival,
poll logging or new endpoint.

## First Observer rejection evidence

Seven initial root-admin cases compile against the existing Snapshot API and
reach the missing diagnostic assertion before production edits. A later frozen
eight-case baseline adds an authored zero-length packet and precise zero-byte
naming. Its test binary exits101 with eight missing-record failures after actual
failure controls and owned cleanup. Missing-record RED does not claim that the
deeper new-value assertions ran. The exact final eight test/fixture bytes then
pass against the diagnostic implementation.

The matching root-admin cohort passes12 cases: those eight plus the four existing
Source status cases. It uses production actor/storage, SourceOwner and Observer
with real authenticated Linux SEQPACKET IPC and authored UID1000 producers.
A single initial accepted packet followed by explicit controls separates an
expired queued intermediate frame from a genuine sample-chain gap. Sequence,
known socket close, zero-length packet, truncation, malformed data and drain17
are distinct controlled inputs. Existing fail-closed state/ErrorKind assertions
precede the new diagnostic checks. Repeated authenticated snapshots retain the
same first failure, and bounded-key assertions exclude raw data and peer identity.

The57-case Observer process suite passes, including two new regression-only
checks for retained typed diagnostics after Stop/repeated poll/snapshot and no
fabricated rejection on intentional Stop. Those typed-API checks have no separate
feature-RED claim. Four existing lifecycle observation cases also pass. Strict
formatting and all-target Clippy pass for session, session-owner and lifecycle.
The matching release owner and fixed Sounds launch client build as native
ELF machine62; this is compilation, not original compositor execution.

The final baseline records eight actor waits0 with16 PIPE EOFs and eight Source
waits9. GREEN records12 actor waits0 with24 PIPE EOFs and11 Source waits9; Source
streams are DEVNULL. Their original eight-second custody bounds hold. Each outer
epoch has ten actual CLI waits and merged-output PIPE EOFs, exact same-daemon
container absence, outer ECHILD and unchanged selected-source hashes. The source
selection contains514 Cargo/lock/Rust inputs. The runner-only missing-Python and
missing-environment attempts remain setup failures; the zero-packet fixture's
first generic send is rejected before its readiness receipt and is not a complete
RED cohort. An initial strict Clippy size failure remains failed; boxing the one
optional actor-held diagnostic fixes it without suppressing the lint.

[Hash-only rejection provenance](evidence/native-source-rejection-provenance.json)
binds the changed sources, exact fixed tests, native artifacts and private
receipts. This host cohort supplies a diagnostic for the next original replay;
it establishes no new compositor, normal GUI/PCM, ARM or phone qualification.

Matching ARM64 default-feature release owner/client compilation also passes
strict formatting and selected Clippy. Both are ELF64 little-endian machine183.
Its ten actual CLI waits/merged PIPE EOFs, original cutoffs, same-daemon exact
container absence, source closure and ECHILD qualify separately. No ARM/QEMU/VM
or phone execution occurs in that compile cohort.

The following [original native replay](native-session-x86-runtime.md#typed-rejection-replay)
uses the typed record at the real boundary: sequence2 is rejected for a110.824ms
sample-chain gap while only5.576ms old at failure observation. This identifies a
producer gap in that selection; the responsible compositor operation remains
unknown. It supplies no normal GUI/PCM or physical qualification.

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
