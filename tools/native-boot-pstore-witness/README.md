# Independent pstore witnesses

This Rust child and Python observer are guest test infrastructure, separate from
production native PID1. They require root, PPID1 and the selected Google kernel,
then mount pstore read-only and validate bounded procfs/sysfs/pstore reads.
The witness has explicit historical-canary and native-record profiles. It is
not a host tool or installed phone service; no proprietary input or generated
binary ships.

## Native PID1 record profiles

An authored `/native-pmsg-profile` archive member selects success, failure or
no-backend. These branches only read backend identity and retained data: they
never create a PMSG node, open it for writing or emit a canary. With no prior
console, the selected first child exits37 or controlled3. Actual retained
console selects the next boot's retrieval branch; it reports bounded whole
console and actual present/missing PMSG, then exits37 even when PMSG is missing.
The independent goal can therefore distinguish working ordinary native boot
from missing native PID1 records.

`observer.inspect_native` appends a separate goal without changing the old
`inspect`. It requires exact native ACKs, actual matching child waits, distinct
kernel/native boot identities, whole previous-console markers and actual QMP
request/reply/event order. Whole retained PMSG must equal all four first-boot
ACK-derived canonical records. SUBMITTED markers/write counts alone cannot pass.
`classify_native_cases` requires the completed success, failure and actual
no-backend cases plus all owned closure before classifying RED or GREEN.
The separate Arch `diagnostics.py` remains unchanged and is not repurposed as
a Google-kernel oracle.

The published `qmp-profile.py` and `run-native-pmsg-case.py` preserve the executed
post-PowerOff profile and owned receiver. QEMU uses `-no-shutdown`: actual first
PowerOff/SHUTDOWN and stopped-status observation precede one authenticated reset,
actual stopped post-reset status and explicit cont. Second native boot/retrieval/
PowerOff precede final SHUTDOWN and owned quit. Terminal socket EOF/ECONNRESET
requires a successful quit reply and all required guest shutdowns. Merged pipe
EOF, actual CLI wait, original deadlines, ROM bounds, exact CID/same-daemon and
input closure remain separate requirements. No phone boot/retrieval commands
are supplied.

The runtime receiver is staged as `run-frozen.py` beside a fresh reviewed private
`selection.json` and `command.json`, with fixed `source/observer.py` and
`source/qmp-profile.py`. Its arguments bind both manifest hashes. Private kernel,
archive, output directory and shared input lock must already be selected; this
source is not a turnkey device launcher. `native-pmsg-goal.py` takes the completed
private case index and a fresh result path and requires the same positive goal
that actually failed before the backend repair. Run Python with assertions enabled.

[Native-PMSG proof](../../docs/native-boot-native-pmsg.md) records genuine
Unsupported RED followed by pmsg.rs-only GREEN: success37→37 and failure3→37
recover complete native records, while actual backend absence still completes
child37. Five GREEN boots produce20 ACKs. The current PMSG payload is now
repacked into two reproducible8MiB development bundles. A separate fixed metadata
check produced actual suite1 for the stale PMSG-profile boolean; changing only
that producer boolean produced suite0, without a new format RED. Two actual
exact-package Arch attempts then failed the unchanged ACK checker because
kernel console interleaved into event2 and event4 respectively. Normal child37,
PowerOff, Docker CLI0 and CID/input closure did not override either refusal.
Current Arch ACK qualification remains open; no relaxation or third retry was
used. The five-boot Google PMSG result stays separate, and earlier successful
Arch/full-ramdisk executions keep their historical binary/archive bindings.

## Historical canary selection

When the profile file is absent, the original
[Google-kernel warm-reset experiment](../../docs/native-boot-warm-reset.md)
remains available. Its first child discovers dynamic PMSG identity, creates and
validates its own0600 character node, writes one random canary, reports READY
and waits. That experiment resets the live first child; it does not observe its
wait. The second child reads the whole canary and prior console, exits37 and
is actually waited. This mechanism proof does not establish native PID1 record
retention by itself.

The original build used Rust1.98.1, GCC14.2 and the frozen offline registry:

```sh
RUSTFLAGS='-C target-feature=+crt-static' cargo clippy --all-targets --target aarch64-unknown-linux-gnu --locked --offline -- -D warnings
RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target aarch64-unknown-linux-gnu --locked --offline
```

These commands require the provisioned AArch64 GNU cross linker, target standard
library and exact Cargo.lock dependencies. Recorded binaries are static GNU,
with neither PT_INTERP nor PT_DYNAMIC; relocating the source does not promise
byte reproducibility. The historical fresh archive supplied its unchanged native
init, witness, `/dev/kmsg` character1:11 and `/pstore`. The owned paused observer
checked every boot-blob extent before cont and waited for actual READY before
reset. The native two-second child and100ms kmsg readback bounds were unchanged.

`observer.inspect(full_console, qmp_records)` retains its original exact source.
It checks the whole canary, distinct boot/native identities, saved console
markers, six ordered ACKs, matching second child wait37 and successful owned
reset/RESET-before-SHUTDOWN. Run Python with assertions enabled. Returned raw
retained bytes stay private. Saved-output controls are component replays,
separate from actual kernel execution.

The historical [bounded proof](../../docs/evidence/native-boot-warm-reset-provenance.json)
retains its original source/binary/runtime bindings, paused prerequisite failure,
actual no-backend refusal and observer controls. Both experiments cover explicit
synthetic reserved RAM. Neither qualifies Pixel addresses, hardware/power-loss
retention, panic records, bootloader admission, USB retrieval or usable phone
recovery.
