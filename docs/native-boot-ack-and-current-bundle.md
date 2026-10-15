# Current diagnostic bundle: intact ACK and full-stream execution

One current complete development bundle now connects the native build, exact
original-unpacked generic ramdisk and normal diagnostic child to two actual
kernel consumers. The unchanged Arch consumer accepts four intact readback
ACKs, matching child wait37 and powerdown. The retained full four-frame Google
consumer also accepts native PID1, the expected uname, four ordered ACKs,
matching child wait37 and QMP guest shutdown. Owned waits, exact container
absence and input closure pass. [Bounded proof](evidence/native-boot-current-bundle-provenance.json).

These are actual ARM kernel executions on synthetic QEMU hardware. The complete
vendor stream is an explicitly chosen counterfactual; no real loader route or
physical Pixel admission follows. Original binaries, archives and raw execution
outputs remain private.

## Preserved integration RED and bounded repair

The previous PMSG-enabled package had two actual Arch consumer failures. Kernel
console text interleaved inside event2's raw ACK in the first and event4's
completion ACK in the second. Both consumers actually exited1 despite normal
child37, PowerOff, Docker CLI0, exact CID absence and unchanged inputs. The
unchanged checker correctly refused each corrupted message. Those runs remain
the integration RED; neither was relabeled a fixture error or silently accepted.

A small extracted ACK helper first preserved the existing stdout-lock,
`writeln!` and flush semantics. Repair changed only that helper to format the
entire ACK, including its newline, before one direct `rustix::io::write`. A
short count or error returns failure without replay. The required epoch/event,
kernel-ring readback, ACK contents, child ownership and PowerOff policy remain.
The existing Arch consumer then actually exited0 on the new package, with its
release-specific checker unchanged.

The host packet control is a separate result: **baseline and repair both
passed**. Each observed one whole packet for the139-byte kernel ACK and158-byte
completion ACK. There was no packet-control RED. Host strace also observed one
fd1 write in both versions of the139-byte control, with full count and actual
wait/EOF. These observations do not establish that the old native TTY path used
multiple syscalls, prove why the earlier console interleaved, or establish a
general console/TTY atomicity guarantee. The host control therefore does not
identify the physical serial interleaving mechanism. Current kernel-consumer GREEN is the
measured end-to-end result, not a claim that this source change prevents every
future interleaving.

## One current bundle and its exact consumers

The full8MiB bundle was built twice reproducibly. Original AOSP unpack and AVB
checks, independent Rust geometry/payload inspection, exact authenticated stock
restore, four unchanged reference assets and two isolated tamper refusals pass.
Fresh NONE metadata remains unsigned and does not establish stock trust-chain
or deployment acceptance. The prior PMSG-profile metadata correction is separate
from both ACK repair and the earlier complete-image format TDD.

| Current artifact | Bytes | SHA256 |
| --- | --- | --- |
| Full candidate | 8,388,608 | `9bf8d4ab99a9ef82fa6c82546517cf494d64855832a4f15422b898d9f82410e7` |
| Compact intermediate | 1,466,368 | `b9b3be959ac35c2de1290877a8acb8bc1a856f02b841e28195dd963fc55eefdd` |
| Native init | 1,255,888 | `c47752349f7df514708d6046904bac66782bfecc5cb7d7fce8ec69f5b49dd7ad` |
| Original-unpacked generic | 1,461,420 | `be802346c2a9a35d42d7606fef5786b3bc65c90d3dc91d6488e726d01de50fb4` |

The full manifest SHA256 is
`78b8c5c511e5a3c90f1bc402681b5fb7d0d1fde7d2e06946dbfbf286a4205fc1`.
The normal packaged child remains the exact selected native child, not the PMSG
retrieval witness or host packet fixture. The host fixture is pinned only as
build/test infrastructure and never attached to either kernel consumer.

The Arch run consumes the package's exact original-extracted generic with
kernel7.1.6-1-aarch64-ARCH. Its four ACKs retain one epoch, events1–4 and strictly
increasing observed kernel sequences; its actual child is executed and waited37.
The consumer, Docker wait0, powerdown, same-daemon/CID absence and input checks
all pass. This closes the current selected Arch ACK checkpoint without changing
the earlier failed results.

The full-stream preparer independently unpacks the full candidate and checks
Rust/header geometry before assembly. It preserves the exact45,338,032-byte
vendor prefix: all5,343,791 vendor-kernel bytes,17zero bytes, all39,994,220
vendor_boot bytes containing both fragments, then4zero bytes. Only the final
generic suffix changes. The new assembled stream is46,799,452 bytes, SHA256
`baf877cc85ded60022dff16a4f925564fc26f9161434a4e963364406216ed8e9`.

All151 prior external input bindings remain, with additional current bundle,
build/source and artifact pins. The frozen LZ4/newc readers and original
libarchive cross-check every complete frame, entry and payload. The seven
existing namespace vectors pass, then every actual record is applied to the
unchanged source-bound namespace model. Final init/child bytes and root0600
character `/dev/kmsg`1:11 match. That model is data evidence, not kernel VFS/LSM
or arbitrary vendor-program execution proof.

The retained full-stream runner changes only its ROOT literal; acceptance,
ROM extent checks, QMP, release-specific ACK matching and original deadlines
are unchanged. Command identity/mount paths select the fresh output and new
stream. Guest512MiB/container1GiB,2CPUs, paused pre-cont whole-boot-blob checks,
45s outer/5s setup bounds, security flags, no NIC/root disk/host devices and
owned closure remain selected. The actual authenticated Google4K kernel
6.6.127-android15-8-g91d825611104-ab16252310-4k consumes all four frames and
completes normal native child37 with four intact ACKs and guest SHUTDOWN.
Its driver actually exits0; no fixture error or parser relaxation is involved.
Vendor programs are not explicitly launched; every possible kernel usermode
helper was not independently traced.

The new native init also passes the separate [PMSG regression](native-boot-native-pmsg.md)
fixtures: success37→37, controlled failure3→37 and actual backend absence37.
Those five boots retain20 ACKs and exact first-boot native records in the reset
cases. They use a read-only retrieval witness and synthetic reserved RAM;
they do not turn the ordinary package consumers into hardware-retention proof.
Earlier package, full-ramdisk and canary experiments retain their own bindings.

## Phone experiment gates

No phone was accessed and the full deployment profile remains refused. The
matched kernel's built-in pstore/PMSG/ramoops configuration and retained DTB
metadata are favorable static prerequisites. Actual selected image/slot, DTB/
DTBO/bootloader edits, effective arguments, backend registration, permissions
and hardware access remain unqualified. No guessed address, fixed device major,
module substitute, disabling flag or restart string is introduced.

Synthetic `-no-shutdown` plus owned reset preserves modeled RAM. A real Pixel
PowerOff may lose DRAM; the current payload provides no qualified RAM-preserving
recovery transition. Warm retention is not power-loss persistence. An independent
recovery entry, current-state restoration set, observable execution and a working
host retrieval path must be established before a physical experiment.

The first compatible recovery boot matters: ramoops initialization copies valid
old console/PMSG records and resets the backing rings. See the
[matched common-kernel retention source](https://android.googlesource.com/kernel/common/+/91d825611104e75e1a9e0e0f6b25580eddb68828/fs/pstore/ram_core.c)
and [ramoops initialization](https://android.googlesource.com/kernel/common/+/91d825611104e75e1a9e0e0f6b25580eddb68828/fs/pstore/ram.c). A read-only pstore mount
does not prevent that earlier initialization effect. Export the first snapshot
before another boot or unrelated consumer can replace it; do not unlink records,
which invokes backend erasure. A future read-only collector needs qualified
filesystem/file identities, whole bounded binary captures, actual EOF, sizes
and hashes, then epoch/event interpretation. Hardware files may contain other
writers or wrapping/truncation; the synthetic4KiB PMSG/64KiB console caps and
exact-four-line whole-file expectation are not hardware collection limits.
Physical reset/DRAM preservation, USB/transfer custody, early-failure retrieval,
watchdog/thermal safety and recovery remain open. This document supplies no
physical-access or partition-operation instructions.
