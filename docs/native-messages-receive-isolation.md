# Independent Messages read and receiving lifetimes

A held app read can now close, expire or lose its child while the same live
background incoming writer continues to receive and project. The correction
keeps one owner of `ReceiptOwner`, `Inbox` and the source backend, and gives app
queries an independently supervised readonly child. Reader cancellation and
local recovery never attach Binder, replace the source generation or recreate
raw arrivals. Whole service Stop remains terminal and owns both lifetimes.

This qualification uses an authored frozen messaging V3 NDK peer, retained
original Bionic/libbinder_ndk libraries, an actual ARM64 Binder kernel, SQLite
and distinct-UID native lifecycle clients. It uses no session Provider or Qt
window. It therefore qualifies the receiving/read boundary independently of the
later compositor/session composition. The normal production Messages entrypoint
still has no incoming backend, Binder dependency, ACK or send endpoint. No phone
was accessed and no vendor modem executable ran.

A later [Google4K-kernel checkpoint](native-google-kernel-services.md#later-current-source-binder-composition)
executes the current native sources through this existing synthetic receiver:
three callbacks commit three raw occurrences, held-reader revocation preserves
the same writer/source, and clean reopen retains exact IDs without reattaching.
Its reader is stopped before the query; the older actual SQLite-transaction
controls below remain separate. The normal daemon remains history-only.

## Ownership and publication

`Provider::incarnation()` names the readonly child, while source packets retain
the writer incarnation and exact slot/source/generation binding. Read requests
and replies match that reader incarnation, a monotonic token and the original
absolute BOOTTIME deadline. A successor reader receives a fresh incarnation;
old buffered replies and stamps cannot become its replies. `cancel_read(token)`
acts only on the exact pending reader request. `read_occupied()` retains that
child until actual `try_wait` reap; aggregate `occupied()` retains every child
for shutdown. No child is detached and a failed reap never refunds capacity.

The lifecycle actor retains its publication checks immediately before every
nonblocking send, including EAGAIN retries: current principal/pidfd generation,
policy, lifecycle, readonly incarnation/liveness and original read deadline.
The two pending read EOF/deadline sites now cancel the read lifetime. The fixed
desktop launch can request one local readonly recovery only after actual reap
and after the writer projection phase finishes, within the existing two-second
intent deadline. A failed reader bootstrap is local unavailability and does not
implicitly retry. Reader polling errors fence and stop the reader locally;
writer ownership errors still terminate the service boundary.

## Completed projection prefix and SQLite exclusion

The readonly child opens only an existing private projection database with
SQLite READ_ONLY, query_only and trusted_schema disabled. It owns no writer
`owner.lock`, raw journal, schema writes or repair path. Bootstrap filesystem and
SQLite work run inside the supervised reader. The writer qualifies an explicit
Inbox record sequence upper only after complete raw reconstruction and the
current-tail check. Each read keeps that upper fixed in one actual transaction.
Both conversation membership/counts and message rows derive from records at or
below it; new partial-tail conversation labels and cached counts cannot leak.
A successful reply follows explicit transaction completion and lock release.
Read initialization is supervised under its own original deadline, and later
queries retain their app deadline through SQLite work and every reply attempt.

SQLite remains in DELETE journal mode with zero busy timeout. To avoid a stopped
readonly transaction causing writer commit failure, the writer commits each raw
occurrence once and waits for an authenticated projection permit. The parent
reserves the writer phase before sending the permit. It admits no new read or
reader bootstrap until that exact occurrence completes. The permit matches the
writer/source incarnation, generation, slot/source binding, occurrence, exact
committed receipt ID and the original one-second arrival deadline. It grants
projection only after observed authenticated read/init completion, which
certifies released SQLite work, or actual reader reap. Stale, duplicate,
mismatched and expired permits cannot release another occurrence.

An unobserved dead child still withholds permission. If the reader cannot finish
or be reaped before the original arrival deadline, receiving explicitly fails
with the raw ID retained. This correction adds no deadline renewal, SQLite
retry, journal-mode change, automatic raw re-admission, Binder replay,
reattachment or uncertainty reset. All source failure/capacity rules remain.
Complete raw enumeration still uses owner-bound fixed-upper pages, eight imports
per writer turn and the existing append-only4096 bound. Inbox limits remain4096
keys,2048 text messages and512 conversations; replies remain4096-byte packets
with at most8 conversations or2 messages.

## Executed evidence and reproduction

[Clean provenance](evidence/native-messages-receive-isolation-provenance.json)
selects exact source, binaries, inputs and executed results. Historical incoming,
enumeration and installed correction proof remains unchanged. Two desired ARM
reds precede their respective implementation corrections:

1. Against `84ac3f8`, stopping the shared child during a read then closing the app
   kills the receiver; the next actual Binder callback cannot add raw receipt2.
2. Against the frozen initial split, a fixture-stopped actual readonly transaction
   blocks an independent `BEGIN EXCLUSIVE`. The next Binder callback commits raw2
   but projection remains1 and receiving terminally fails with
   `projection_storage_unavailable`.

The corrected ARM scenarios qualify held-read close/reopen, late permission
denial, read expiry, locked background receiving, whole Stop, actual held SQLite
transaction cancellation, graceful transaction completion, reader death and
local recovery, dead-but-unobserved reap withholding through the original arrival
deadline, readonly bootstrap OS failure, and33 exact live Binder occurrence IDs.
Seventeen selected ARM groups include fourteen integration groups and three
component groups. All51 focused cases (9 projection,22 receipt and20 Inbox) pass
on x86 and the actual ARM kernel; ARM process
cases use `--include-ignored` because the default target policy excludes them
from user-QEMU runs. The receipt suite covers full4096 enumeration. The existing
Inbox test compares durable metadata with33 recorded original AOSP decoder
outputs; it does not rerun an Android parser or create new original matches.

Run from this isolated worktree with the pinned local builder/simulator images,
the existing frozen public-generated fixture helpers and readonly retained
runtime/kernel inputs. Each VM reuses one owned backing image with snapshot=on,
no NIC and a private lock/result label:

```sh
bash tools/native-messages-receive-isolation/prepare.sh
# Only for a fresh isolated workspace without its owned backing:
bash tools/native-messages-receive-isolation/run.sh clone
bash tools/native-messages-receive-isolation/check.sh
bash tools/native-messages-receive-isolation/build.sh
bash tools/native-messages-receive-isolation/run.sh green chosen-held-close-label
bash tools/native-messages-receive-isolation/run.sh lock-green chosen-lock-label
bash tools/native-messages-receive-isolation/run.sh errors-green chosen-errors-label
bash tools/native-messages-receive-isolation/run.sh enumeration-green chosen-enumeration-label
```

`index.py` freezes the selected labels listed in that script and refuses changed
bytes at an existing selection. New reproduction labels remain independent proof.

The `red` and `lock-red` harnesses must use their frozen source/binary selections;
current corrected binaries are expected to pass the desired behavior. Setup
errors, earlier incomplete greens and first test-log aggregation/ignore-policy
attempts remain unselected diagnostics. Final selected runs never mix their
binaries with those earlier attempts.

Physical receive/producer identity, production backend admission and permissions,
ACK, sending, carrier behavior, notifications/wake, retention/eviction, cold
backlog availability, general filesystem stalls and the joint installed
compositor/session/Qt path remain separate qualification work. This change does
not classify those native implementation or integration gaps as hardware proof.
