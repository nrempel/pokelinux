# One initial SIT attempt

`pokelinux-sit-attempt` owns the local lifecycle of one initial-token-zero setup
request and source generation. It composes the qualified983-byte request,
144-byte response decoder and owned modern22 report. It performs no I/O or
process management; its caller supplies source custody and loss notifications.

`Setup::new` validates the request and explicit presentation index0–99.
`Attempt::new` fixes source and absolute deadline. `take_dispatch` records
`OutcomeUnknown` before exposing the non-Clone packet. Dropping it or observing
a failed write cannot reverse uncertainty; a byte borrow cannot enforce syscall
count or prevent outside retries.

`prepare_response` checks lifecycle/source, decodes exactly144 bytes for token0
and matching CID, and projects outside the mutex before rechecking the original
deadline and lifecycle. Invalid records do not consume the valid-candidate slot.
One opaque non-Clone candidate consumes that slot even if dropped. `commit`
consumes the exact same-owner candidate, rechecks the gate, and records its full
source/record/report as the same immutable observation in `current` and `history`.
Dispatch remains unknown.

Cancellation, expiry and matching-source loss clear current observations while
retaining completed history and the first withdrawal reason. Snapshots remain
readable. Previously returned snapshots are historical samples. `current` reflects
locally recorded custody state, without atomic proof that an outside process is
still alive. Public `ProcessIdentity` codecs grant no custody; bind them to the
retained supervisor and actual admitted channel.

Clock implementations must be bounded, monotonic and non-reentrant. Every gate
samples the original absolute deadline without delivered timers. The fixture
uses Linux `CLOCK_BOOTTIME`; no suspend/wake experiment is claimed.

The fixed15 tests pass on host and ARM64 user-QEMU after compiled13-pass/two-
missing-completion RED. The unchanged process fixture separately passes actual
host and generic ARM kernel RED→GREEN, including full983/144-byte/report
comparisons, held-worker snapshots, expiry/cancel and actual Stop/wait/journal
refusal. The40 codec tests remain separately qualified unchanged prerequisites.
See [evidence and limits](../docs/native-sit-attempt.md).

```sh
cargo test --locked --offline --manifest-path native-sit-attempt/Cargo.toml
```

The [process owner](../native-process-owner/README.md) owns admission, Stop and
actual wait. Local reaping proves no vendor cleanup and clears no durable fence.
This crate grants no provider acceptance, APN acquisition, link/channel binding,
retry, token reset or cellular readiness.
