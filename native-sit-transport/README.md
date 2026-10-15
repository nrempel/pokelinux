# Owned SIT descriptor transport

`WorkerTransport` consumes one Linux `OwnedFd` and one original absolute
`CLOCK_BOOTTIME` deadline. It exposes no descriptor, clone, replacement, reopen,
reset or retry operation. It does not establish source identity or readiness.

One exact983-byte request may be dispatched. Uncertainty is recorded before the
single actual write. Short/zero/error writes stop the local generation without
continuing or retrying. Each subsequent `read_chunk` makes one actual raw read
into65,568 reusable bytes and copies its positive slice into `ReceivedChunk`.
Short, opaque and coalesced input stays opaque; framing and complete-record
admission belong outside this primitive. Zero/error/copy failure is terminal.

Every operation checks the original suspend-aware deadline before I/O, after
I/O and after receive copying. Deadline expiry drops the local handle and cannot
clear dispatch uncertainty. Snapshots are historical local values. Syscalls and
close may block: use this primitive inside the separately supervised worker,
with Main retaining process custody, cancellation and durable admission.
Reaping or descriptor close grants no vendor cleanup or safe successor retry.

```sh
cargo test --offline --locked --manifest-path native-sit-transport/Cargo.toml
```

Ten real Linux descriptor tests pass on host and ARM64 user-QEMU after compiled
missing-dispatch and missing-read REDs. The existing Main/worker fixture also
uses this primitive; its process integration has a separate evidence class.
See [qualification and remaining gates](../docs/native-sit-owned-transport.md).
