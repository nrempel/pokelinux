# Frozen initial AQ broker batch consumer contract

SOURCE ONLY. No API implementation, compiler, build, import, guest, VM, container,
compositor, test runtime or phone executed for this contract. Do not implement the
production batch API until Root retains an actual missing-API compile RED against
the current unchanged SDK. That RED is structural compilation evidence, not a
semantic production bug RED or a later runtime GREEN.

The agreed proposed public declaration is in Aquamarine::CSession, with the enum
in SessionEvents.hpp (shown here as a contract, never a local consumer fallback):

```cpp
enum class eSessionInitialRenderEventsResult : uint32_t {
    READY=0, NOT_READY, WRONG_OWNER, CAPACITY, INVALID_PREFIX, TERMINAL
};
eSessionInitialRenderEventsResult peekInitialRenderEvents(
    SSessionPendingEvent* out, size_t capacity, size_t* count) const;
```

Callers supply live aligned nonaliasing storage/count. Owner/INPUT_BROKER guard
comes first, then failed/shutting-down terminal state. Every refusal preserves
all output/count object bytes, queue position and descriptors. READY alone copies
one complete INIT(sequence1) -> DEVICE_ADD* -> ENUMERATION_DONE prefix, with
matching generation, contiguous sequences, bounded current metadata and at most
32 devices/34 records. The generic grammar permits zero devices/two events.
NOT_READY means no complete initial prefix (including an empty drained queue);
INVALID_PREFIX rejects malformed/noninitial prefixes, including a front advanced
past INIT. CAPACITY refuses insufficient caller storage or the source bound.
No partial copy, queue advance, FD duplicate/close, native work or callback occurs
in peek. Copied FD integers remain synchronous borrows from the broker.

The fixture calls the real attemptInputBroker with a real libseat seatd socket,
udev/libinput and virtual virtio_gpu primary/render capabilities. It selects
public service records with nonzero graphics lease IDs; lease0 libinput records
remain untouched and control-owned. The device count/bound applies only to those
selected graphics leases. The fixture it never injects private fields, queue records, fake FDs or a
mock seat callback. Enabled seat, real graphics device(s), real render capability,
expected virtual driver and intact initial front are setup preconditions. Their
absence, constructor failure, early real disable/revoke or timeout before setup
qualification is SETUP(77), never a target RED. Primary/render stat identities,
CLOEXEC on pending duplicates, actual udev node/boot-vga, source sequences and all
metadata values are checked. Descriptor closure concerns the pending duplicates;
original libseat service leases must remain live through every queue mark.

After READY: repeat the same full peek with no advance/closure, undersized capacity
and actual wrong-thread poison-output/count refusal, then exact fieldwise copies
to PrBootstrapEventV1. The fixture links the one original current Rust246 archive
and requires pr_bootstrap_capture_v1 acceptance while every source FD remains
live. Only after that success may it mark each exact head sequence, observe exact
next head/empty queue and EBADF for that event's original pending duplicates.
Remaining pending descriptors and all original service descriptors remain open.
After the first mark, re-peeking an initial batch refuses INVALID_PREFIX; after
the last it refuses NOT_READY. Existing Rust pack duplication/lifetime tests are
separate: this fixture creates no executor, mirror, backend, output registry,
fabricated worker discovery or graphics retirement aggregate.

The required preworker revoke callback always returns AQ_REVOKE_UNAVAILABLE,
latches terminal/no-admission, and validates owner/generation/reason. It never
returns AQ_REVOKED or AQ_ALREADY_STOPPED for an absent Rust owner. Initial enabled
seat admission requires zero callback calls. After the positive capture/mark
boundary, an explicit public failRenderTransport request tests that truthful
UNAVAILABLE/terminal latch and further admission refusal; it is not represented
as an actual libseat disable event. Final broker destruction is forced owned
service cleanup, never service-lease retirement or native/device drain evidence.

Freeze and pin this whole consumer before any production change. Compile it
unchanged against the exact current SDK first: direct references to the missing
enum/member must fail, with other include/link setup verified separately. No
feature-probe fallback or test-local method/enum definition may hide the failure.
After that retained compile RED, implementation may be authored; rebuild one
matching AQ15 library+headers, compile the identical consumer, and run only in an
explicit approved native KVM guest with LIBSEAT_BACKEND=seatd, an absolute real
SEATD_SOCK, UID/GID1000 with no supplementary groups and no LD_PRELOAD/LD_AUDIT
interposition. Link one Rust
archive only, plus actual AQ15, libseat, libudev and libdrm dependencies. Pin actual
pc/include/library queries, ELF DT_NEEDED and loader closure, with no AQ14 mixing.

CPU fixture bounds are fixed CLOCK_BOOTTIME start+5s work/start+8s close; the
supervisor's outer runtime cutoff is start+20s, with retained exact child wait and
actual merged PIPE EOF under that original cutoff. Constructor/destructor/native
calls cannot be interrupted by these CPU checks; a hung call is an unqualified
setup/closure failure disposed by the outer owner, never a late success. A blocked
wrong-owner test uses explicit process exit, never detach or a claimed thread
join. Preserve all failing logs and original cutoffs; no renewed deadlines. A
runtime PASS requires setup qualified, all assertions, actual join of the short
wrong-owner thread and broker/pack destruction before the original close cutoff.
No Source GREEN, main worker startup, ARM, phone or full smartphone gate closes.
