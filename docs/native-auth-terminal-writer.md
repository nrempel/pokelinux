# Native authentication terminal writer

Four host tests now execute the exact fixture Terminal writer through actual
nonblocking private pipes. The same selection passes59 library tests, strict
formatting/default/all-target/all-feature Clippy and separate ARM64 fixture
compilation. The new tests use an authored outcome source; they do not construct
Attempt, run the root fixture entry or qualify authentication cleanup.
[Bounded provenance](evidence/native-auth-terminal-writer-provenance.json) keeps
that boundary separate from the earlier [ARM59](native-auth-closed-outcome-arm.md).

Before extraction, the four new tests fail to compile because the private
TerminalSource adapter is absent (E0432/Cargo101). Zero test bodies execute; this
is a compile prerequisite, not a reproduced runtime defect. A later first build
fails strict Clippy before any tests run. Both immutable labels remain failed.
The final selection passes all59+4 host tests with3 explicit library helper
entries invoked by their parents.

The small private adapter preserves the existing writer's decision/current/write
order. Its only runtime implementation is actual Attempt: take_record consumes
the owner's opaque WorkerOutcome through the existing serializer, and current
checks the real owner. No public constructor, protocol compatibility or actor
result ingress is added. The test implementation authors a raw denied record
and models revocation/original BOOTTIME expiry. Only that test model can supply
raw bytes to this private interface.

The tests fill an actual pipe to EAGAIN before extraction, then drain it and
require exactly one outcome followed by the identical CLOSED snapshot. A second
layout leaves one48-byte record of space in the actual tail page: outcome writes,
CLOSED hits EAGAIN, and retries must never repeat the outcome or take another
decision. Revocation after delivered outcome cannot retract bytes; CLOSED still
uses the captured snapshot. Separate cases first require a pending outcome,
then revoke or cross its unchanged original clock limit while output is full.
After drainage only CLOSED may appear, with no new decision. Test layout uses
the kernel page size through a dev-only rustix feature, rather than assuming4KiB.

All four tests call the exact writer method and decode its actual bytes. The
classification source, closed workload snapshot and owner-current checks are
models. The actual Attempt bridge, full supervisor/parent loops, original PAM,
helper/cgroup cleanup, effective faillock, actor/Source binding, armed receipt,
exact unlock, durable clear and Messages reopen remain unqualified here. Broken
pipe/EINTR paths, actual suspend and matched Google/physical behavior are not
newly established. Existing samples are not atomic against asynchronous
cancellation or clock passage; the receiving owner/actor must revalidate.

The separate actor Source budget remains100ms with no renewal/revival. The
historical ARM59 library selection keeps its earlier Cargo/source snapshot and
does not qualify these four new binary tests. No phone was accessed.
