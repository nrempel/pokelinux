# Initial SIT attempt gate

One native Rust attempt now composes the qualified initial SIT request, base
response parser and modern report with an owned process source. The crate owns
only the local observation lifecycle. Its explicit Unix `SOCK_SEQPACKET` fixture
supplies actual Linux process custody through `native-process-owner`; it is not
a factory device adapter or installed phone service.

The later [receive ownership](native-sit-received-chunk.md) checkpoint adds a
bounded raw copy at the fixture worker's actual receive buffer, before full
storage overwrite/check and serialization. Its new source-bound contract keeps
the controller, bytes,14-field report and lifecycle oracle fixed. The original
attempt RED/GREEN evidence below remains historical and unchanged.

The [owned descriptor transport](native-sit-owned-transport.md) now replaces
fixture-local send/recv/copy with the native one-generation primitive. The
attempt's parser, report, source gate and historical evidence stay separate.
The latest codec regression count is50; its receive ownership controls extend
the earlier40-test report checkpoint.

## Contract

An attempt fixes one validated setup, initial token0, source generation and
absolute deadline. Taking the sole non-Clone dispatch packet records
`OutcomeUnknown` before exposing bytes or the first possible outside write.
Dropping it and failed writes cannot restore `NotSent`.

Preparation requires a complete144-byte record, source/token/CID checks and the
qualified decoder/report subset. Decode and owned projection occur outside the
mutex, with the original deadline and lifecycle checked on both sides. Invalid
records do not consume the one valid-candidate slot. A valid opaque non-Clone
candidate consumes it permanently. Commit consumes the exact same-owner candidate,
rechecks the gate and records the complete immutable source/record/report as both
current and history. It leaves dispatch outcome unknown.

Cancellation, exact deadline expiry and matching-source loss are terminal local
withdrawals. Fresh snapshots clear current, retain completed history and preserve
the first withdrawal reason. Old snapshots are historical samples. Clock calls
are bounded, monotonic and non-reentrant. Linux `CLOCK_BOOTTIME` checks need no
delivered timer; this run contains no suspend/wake experiment.

Identity codecs establish no process or socket custody. The caller must bind the
source to its retained supervisor/read channel and notify loss. `poll_reap` and
admission checks precede commit; process death can occur between them. Current
means valid against locally recorded custody state, without atomic external
liveness or provider/APN authority. A new data-level attempt cannot clear a
supervisor's durable uncertainty fence.

## Qualified execution

| Stage | Result |
|---|---|
| Strict component baseline |13pass/two Unsupported missing-completion failures, actual test exit101; component/tool fmt and Clippy pass |
| Actual host process RED | Completion101 after cleanup; expiry0 and cancel0 |
| Actual generic ARM kernel RED | Same101/0/0 and unchanged oracle |
| Narrow repair | Only attempt `src/lib.rs` changes; all47 other selected files, tests, fixtures, codec and process-owner prerequisites stay identical |
| Component GREEN |15host passes and, separately,15ARM64 user-QEMU passes |
| Actual host process GREEN |0/0/0, full completion/history plus expiry/cancel controls |
| Actual generic ARM kernel GREEN | Same0/0/0, actual UID/GID1000, groups[1000], all capability sets0 and NoNewPrivs1 |

The fixed public `v3-ip.hex` request has983 bytes. The original-reference TSV's
`v4` response has144 bytes, token0, context1, IPv4 and presentation index0.
An independent authored peer captures the complete request, supplies that exact
response and compares all14 emitted semantic report fields. The baseline refuses
otherwise-valid completion with Unsupported. It emits its specific missing marker
and exits101 only after worker Stop, actual wait and durable successor refusal.
For GREEN the full committed source/record/report must match, and current/history
must share the same immutable observation. Expiry/cancel publish neither.

Each case observes the worker blocked in the fixture receive while100 snapshots
finish below200ms. Accepted `SO_PEERCRED`, actual PPid, argv, executable, task and
proc/map records bind the owned processes and peer. The controller keeps the peer
open through actual Rust-parent wait and both output-pipe EOFs, then observes peer
EOF with no second request. The supervisor revokes admission, sends signal9 and
actually waits its worker within the original two-second bound. The unchanged
journal refuses a successor with `AdmissionUncertain`; final controller ECHILD
has no adopted/residual child. The operation's original BOOTTIME deadline is
established before launch and checked before dispatch, preparation and commit.
Each controller case retains its original20-second MONOTONIC outer cutoff.

Each ARM run has17 owned waits: seven host helper/CLI waits with both PIPE EOFs,
two boot and two guest waits with merged PIPE EOFs, three controller→Main waits
with both EOFs, and three separately recorded Main→worker signal9 waits. No
separate worker-output EOF claim follows. Fresh private0700 runtimes/journals and
source incarnations share one disposable guest; no new PID/net namespace per case
is claimed. The generic Arch7.1.6 kernel is distinct from Pixel's matched kernel.
Root independently rehashes75/76 selected and77/79 external RED/GREEN inputs,
including the readonly backing, and reviews raw case/proc/credential/map/Stop
receipts, original cutoffs, same-daemon/exact-CID closure. Dynamic guest maps are
rehashed by the controller/guest and checked against ELF dependencies; Root's
post-run review does not freshly read destroyed guest library files.

Earlier formatting/lint/cache/file-permission setup failures remain setup
failures. A prior fixture paired a token-zero request with a dual response token
`0xffffffff`; the decoder correctly refused TokenMismatch. These are not the
valid completion RED. The captured goldens and token check were never relaxed.
The40 codec tests are unchanged prerequisite evidence, not rerun by this selection.
No new original vendor execution occurs in this fixture.

[Publication provenance](evidence/native-sit-attempt-provenance.json) records the
fixed source/controller/reference and independent review hashes.

## Fixture interface and remaining gates

Build the component and authored process fixture with Cargo. The controller and
frozen contract/public references live under `tools/native-sit-attempt-fixture`.
Its input is an independently reviewed build bundle containing `source/`, a
`build-review-root.json` source/artifact inventory, and the native executable at
`target/debug/pokelinux-sit-attempt-tracer`. It verifies the source inventory,
frozen protocol source and actual uname/ELF machine/artifact digest before use.
The historical frozen build/VM receipts remain private evidence; no general
bundle builder or VM provisioning workflow is claimed by this interface.

```sh
cargo build --locked --offline --manifest-path tools/native-sit-attempt-fixture/Cargo.toml
python3 tools/native-sit-attempt-fixture/controller.py \
  artifacts/native-sit-attempt-reviewed-build \
  "$PWD/artifacts/native-sit-attempt-host-green" green
```

The output directory must be new. `red` qualifies only the baseline's specific
cleaned-up missing-completion failure; the final source requires `green`.
Unix packet/EOF behavior establishes no CPIF partial-I/O, zero-read, ioctl or
modem semantics. Factory framing and buffer retention, descriptor/channel/
subscription identity, matched driver policy, provider/APN lifecycle, complete
IMS, normal installed supervision and physical modem/carrier behavior remain
open. Stop and reaping establish no vendor cleanup, safe retry, journal reset
or token reuse. See [factory transport findings](native-sit-transport.md).
