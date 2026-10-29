# Same-Source timing recovery

The current Rust contract distinguishes a loss of fresh Source authority from a
terminal identity, wire or protocol failure. The sample budget remains 100 ms
BOOTTIME. A living process, queued packet or later heartbeat cannot renew an old
sample, read, authentication attempt, preview or effect deadline.

## Loss and reestablishment

An observed sample expiry or producer sampling gap withdraws positive authority.
The observer checks the lease it actually exported before draining replacements;
the lifecycle actor also checks its accepted lease before polling the provider.
A queued fresh successor cannot conceal a lapse in already exported authority.
Raw Source incarnation, exact sequence, timestamp ordering, lock-generation and
sticky protocol-unlock history remain available for strict validation.

Each boundary records its actual BOOTTIME discovery as the recovery floor,
rather than using the earlier nominal expiry. Two fresh samples must have
strictly advancing timestamps after that floor and remain within a continuous
100 ms sample interval. Both must still be fresh at the final validation boundary,
including after kernel identity IO. A repeated sample, equal timestamps, one new
sample or packets sampled before discovery cannot establish recovery. Expired
candidates are discarded; repeated observation of the same unavailable interval
does not move its floor or repeatedly advance its loss fence.

The internal Provider reports these states explicitly:

| Update | Authority meaning |
| --- | --- |
| `AwaitingFirst` | Initial pair is not established; the original startup deadline still applies |
| `AuthorityLost` | Deliver the observed timing loss before any replacement `Current`, including loss found in the same bounded drain |
| `Recovering` | Continue denying positive authority while awaiting qualifying samples |
| `Current` | Supply candidate facts; the actor independently revalidates them |

The observer and actor retain their own discovery floors and pair checks. An
observer export does not bypass the actor's pair requirement. `snapshot` cannot
export a replacement while loss delivery is pending. Recovery commits a new
semantic fence even when the raw focus/lock facts equal the old facts.

## Policy and ownership

The first timing-loss transition advances lifecycle, clears old permits, retires
authentication/restoration attempts and cancels the existing finite previews.
The session becomes locked and its app principals become background-denied.
Recovery preserves registered principals and their owned processes; it does not
restart the compositor, replace Source, relaunch apps or replay work. Durable
grants, durable pending-lock intent and independent root lock/background intent
remain intact. Fresh facts still need the existing live app identity and grant
checks; timing recovery cannot authenticate or clear a lock.

Old protocol-unlock occurrences remain subject to raw wire validation but cannot
be reused as authentication evidence across initial export or recovery. A new
eligible lock/unlock occurrence and a new authorized operation are separate
requirements. Retired reads, authentication and preview authority never revive.

Malformed/truncated wire, wrong credentials/incarnation/owned identity, skipped
or regressed sequence, backwards or future sample time, inconsistent lock/unlock
history, descriptor errors, drain overflow, source death and Stop remain terminal.
Later valid packets cannot recover those failures. Frame/drain limits, original
startup/request/cleanup deadlines and the 100 ms sample budget are unchanged.

An existing limitation remains: a late lifecycle-bound durable write crossing a
lifecycle change still permanently invalidates the actor under its completion
rules. Timing recovery does not repair or reset that failure. Preserving durable
grants during timing loss does not qualify successful concurrent durable writes.

## Evidence scope

The [hash-only provenance](evidence/native-session-timing-recovery.json) binds
the original actor two-case RED and receiver one-case RED, each with actual exit 101,
to these separately qualified host selections:

| Boundary | Qualified result | Scope |
| --- | --- | --- |
| Actor GREEN 3 | 117 passed, 26 ignored; original two RED cases also replayed byte-unchanged and passed | Authored policy/Provider facts, actual BOOTTIME and owned kernel handles; no Source IPC |
| Receiver Sol 2 | Original RED: 1; recovery controls: 9; process: 57; establishment: 69 pass | Authored credentialed Linux Source traffic, pidfds and receiver validation |
| SourceOwner Sol 2 | 34 pass: Source 8, cleanup 3, status 12, startup 5, Stop 6 | Authored Source processes with actual owner/actor/admin/startup/Stop integration |

Receiver targets share process cases; their counts are per target and must not be
summed as unique tests. Original-case replays are reported separately without
deriving a combined unique total. Formatting and strict all-target Clippy
pass in all three selections. SourceOwner also builds the feature-free optimized
normal x86 owner: 3,399,504 bytes, SHA-256
`ee7fb3d96e1c95d2deb2f406d39f3e4b4d083a83f9f862a67c119d99b194faf5`.
Compilation is separate from normal graphical execution.

Earlier whole cohorts remain failed: actor GREEN 1 lacked required component
setup; actor GREEN 2 retained an obsolete terminal-expiry expectation; receiver
Sol 1 retained two migration expectations. Their focused actor two-case or receiver
1/9-case passes retain only that narrow scope. SourceOwner Sol 1 failed a mechanical
test-only `Option`/`Update` migration before any of its 34 cases ran; its separate
normal release build succeeded. These test/setup corrections are not additional
production recovery repairs, and no failed whole cohort is promoted.

The SourceOwner snapshot explicitly includes an older unused `process.rs`; its
owner targets neither compile nor execute that receiver test. Its production
libraries match the qualified implementation; unrelated dependency inputs remain
selected from public base `63c855c4be59bd491aaafd92cd0ed36a725302ef`.
The original-compositor x86 selections below separately record a failed audio
successor and one diagnostic trace passing recovery plus the selected Play/revoke
flow. Repeatability, authentication, ARM and phone recovery remain unqualified.
Earlier reports retain their frozen results and terminal-expiry behavior.

## Genuine x86 recovery, PCM and revoke pass in one diagnostic trace

A later genuine Omarchy x86 KVM trace passes the whole selected flow. The same
held compositor, normal Rust owner, Sounds app and window survive the pre-Play
pause: observed Source sequence 146 becomes locked `recovering` with null first
failure, then returns to fresh observed sequence 149, received 54.852 ms after
the CONT request. Preview is idle and the media source absent during the pause.

After recovery, actual Speaker, 50% and one Play touch need no Reconnect. The
paired tone window measures 440 Hz speaker RMS 0.0883867 and a silent earpiece;
after the operator fixture revokes permission, both routes measure exact zero
and the retained producer pidfd is terminal with process/client/source/links
absent. Each stage measures 24,000 frames per route, 96,000 total, in addition to
48,000 initial no-grant silent frames. The original work, retirement and closure
bounds, simulated 1080 × 2424/QWindow DPR 2.625 profile and positive manager
Stop-after-Ready case pass. Grant/revoke is test administration, not user consent.

This run uses separately qualified feature-free diagnostic Owner/audio binaries.
PCM criteria, algorithm and Python code are unchanged; the oracle JSON only
rebinds the expected Main ELF identity.
The added diagnostics do not repair or explain the earlier failed cohort below,
prove repeatability, or qualify a full one-second startup opportunity: the
current request deadline remains Source-clamped. The controlled Source loss is
before Play, so audio-active Source-loss cancellation remains untested. This is
x86 KVM evidence, without ARM, physical-speaker or phone qualification. See the
[current Sounds result](native-cold-omarchy-sounds.md#diagnostic-trace-source-recovery-pcm-and-revoke-pass)
and the separately bound hash-only records.

## Genuine x86 desktop prefix; whole audio run failed

A fresh x86 KVM trace now observes timing recovery with genuine Omarchy and the
same retained compositor, normal Rust owner, Sounds app and mapped window.
Authenticated Source sequence 150 is observed before SIGSTOP. After the original
100 ms sample expires, the actor reports `recovering`, locked denial and null
`first_failure`. SIGCONT is followed by fresh `observed` sequence 153, received
52.257 ms after the CONT request. Held pidfds and unchanged process starttimes
confirm continuity of all three processes and the same app window.

The first boundary also passes the actual simulated Pixel 10 profile at
1080 × 2424 with QWindow DPR 2.625 and 48,000 quiet PCM frames. The Source pause
occurs before Play, with preview idle and no media source. This qualifies that
same-process pre-Play recovery only, not audio-active cancellation or repeatable
desktop recovery.

The whole functional cohort remains **FAILED**. Later Play yields only the first
1,024 authored 50% PCM frames before producer loss; no `tone.json`, full tone
window or revoke success is qualified. Main records Starting-state retirement at
guest monotonic 73.004120 s, reported callsite 271; the producer is observed dead
around 73.009 s, before the later 64-observation graph bound. Those records identify
neither the retiring guard nor its cause, and do not establish Source continuity
around the audio retirement. See the [separately scoped Sounds report](native-cold-omarchy-sounds.md#source-recovery-prefix-whole-audio-repetition-failed).

Actual host/boot/guest waits and EOFs, QMP guest shutdown, QEMU exit 0, same-daemon
exact-container absence and unchanged selected inputs qualify closure separately
from that failed functional result. The hash-only provenance binds both scopes.
No new screenshot is published; no ARM or phone recovery follows from this x86 run.

## Fixture and core selection

Migrated tool Provider adapters require the current `Update` contract and their
matching selected core. All seven selected fixture binaries pass offline, locked
`cargo check` with zero warnings; this is compile-only evidence. Historical frozen-core Python recipes retain their
original fixture/core versions; they are not recipes for current migrated
fixtures. A new execution needs a matched, frozen source/build selection rather
than mixing old core inputs with current helpers. No compatibility path is added.

The implementation is in the [Source observer](../native-session/src/lib.rs),
[Provider updates](../native-lifecycle/src/session.rs),
[actor authority](../native-lifecycle/src/authority.rs) and
[actor ingress](../native-lifecycle/src/service.rs). The
[Source owner](../native-session-owner/src/source.rs) keeps the same owned child
and original first-establishment deadline.
