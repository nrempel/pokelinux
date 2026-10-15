# Owned SIT attempt process fixture

An authored Unix `SOCK_SEQPACKET` peer/controller drives a Rust Main and its
supervised worker through exact public983-byte request/144-byte response
fixtures. This is test infrastructure; it opens no CPIF device or physical phone.

```sh
cargo build --locked --offline --manifest-path tools/native-sit-attempt-fixture/Cargo.toml
```

`controller.py BUILD ABS_NEW_OUTPUT green` consumes a separately reviewed build
bundle, rather than the bare Cargo target directory. BUILD contains:

- `source/`: the complete selected component/prerequisite/process-fixture source
  inventory, matching the review's `source_files` path/hash array.
- `build-review-root.json`: `passed=true`, `source_files`, and
  `compiled_executables` records with target, relative path, SHA256 and ELF machine.
- `target/debug/pokelinux-sit-attempt-tracer`: the executable matching the unique
  reviewed artifact for actual uname/ELF62 or183. ARM staging can use an identical
  execution alias for the reviewed cross-compiled artifact.

The frozen contract pins this fixture's Rust source and unchanged public
references. A different source requires a separately reviewed fixture version.
Historical baseline `red` requires the specific missing completion101 only after
actual worker cleanup and durable successor refusal; final source uses `green`.
The final source does not select behavior from that phase argument.

Three cases cover completion, a held candidate past its original BOOTTIME
cutoff without timer delivery, and cancellation. Each observes100 snapshots
while the worker blocks in its second receive, then actual Stop/worker wait9,
unchanged-journal `AdmissionUncertain`, parent wait/both PIPE EOFs and ECHILD.
Full prepared report fields and committed source/record/report are checked.

The V5 source additionally copies the accepted receive slice into
`ReceivedChunk`, overwrites/checks the entire145-byte reusable buffer and emits
only the owned bytes through the same source/peer/hex frame. Its controller and
full request/reply/report/lifecycle oracle are unchanged. This is additional
integration after the separate missing-copy component RED/GREEN, with a new
source-bound contract; no second missing-completion RED is claimed. See
[receive ownership](../../docs/native-sit-received-chunk.md).

The frozen reviewed build and disposable-VM envelopes are private provenance,
not a general published bundle-builder or VM provisioning command. See
[qualification and limits](../../docs/native-sit-attempt.md). Local status,
packet/EOF behavior and reaping establish no factory transport, cleanup, retry,
provider/APN ownership, IMS or hardware compatibility.
