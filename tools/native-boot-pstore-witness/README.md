# Synthetic warm-reset retention witness

This experimental Rust child and Python observer are the exact authored sources
used in the [Google-kernel warm-reset experiment](../../docs/native-boot-warm-reset.md).
They are guest test infrastructure, separate from production native PID1. The
witness requires root, PPID1 and the selected Google kernel release; it creates a
PMSG node and mounts pstore inside that disposable guest. It is not a host tool
or an installed phone service. No proprietary inputs or generated binaries ship.

The original build used Rust1.98.1, GCC14.2 and the frozen offline registry:

```sh
RUSTFLAGS='-C target-feature=+crt-static' cargo clippy --all-targets --target aarch64-unknown-linux-gnu --locked --offline -- -D warnings
RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target aarch64-unknown-linux-gnu --locked --offline
```

These commands need the provisioned AArch64 GNU cross linker, target standard
library and exact Cargo.lock dependencies. The recorded binary is static GNU,
with neither PT_INTERP nor PT_DYNAMIC; source relocation alone does not promise
byte reproducibility. Formatting and Clippy passed before the actual kernel runs.

A fresh RAM-only archive supplies the unchanged production init, this child,
`/dev/kmsg` character1:11 and `/pstore`. The owned paused QEMU observer validates
all boot-blob extents before cont, waits for the actual first READY, requests one
authenticated QMP reset, then records second-boot retrieval and shutdown. A full
write/READY alone cannot pass. The first child is reset, not observed waited.
The production init's existing two-second child and100ms readback bounds remain.

`observer.inspect(full_console, qmp_records)` accepts full raw serial text and
records containing `message` dictionaries. Run ordinary Python with assertions
enabled. It checks the whole canary, different boot/native identities, previous
console markers, six ordered ACKs, second child wait37 and actual reset reply/
RESET-before-SHUTDOWN ordering. Its returned raw retained bytes are private
experiment outputs. The saved-output controls are component replays, separate
from actual kernel execution. This module supplies neither a Docker/QMP driver
nor phone boot/retrieval commands.

The [bounded proof](../../docs/evidence/native-boot-warm-reset-provenance.json)
records exact source/binary/runtime bindings, the failed paused prerequisite,
actual no-backend refusal and observer controls. Qualification covers console/
PMSG on synthetic reserved RAM, not panic records, physical retention, power loss,
bootloader admission, Pixel USB or usable phone recovery.
