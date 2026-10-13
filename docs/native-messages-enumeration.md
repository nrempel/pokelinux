# Bounded complete raw receipt enumeration

The receipt journal has an additive owner-bound page API over a fixed sequence
upper bound. Equal raw PDUs keep distinct existing receipt IDs. The legacy
latest32 snapshot API remains unchanged; Messages projection now consumes the
new complete enumeration rather than treating latest32 as a complete inventory.
The frozen `f733d3b` incoming milestone's explicit33rd-receipt refusal remains
historical evidence.

A scan captures a finite upper sequence and enumerates oldest first, at most32
validated summaries per page. Its private cursor belongs to the same open
`ReceiptOwner`; it cannot be used with another owner or after reopen. Admissions
past the captured upper are deferred to another scan. A completed checkpoint
identifies an already enumerated local prefix, not permission to read, ACK or
send. Page failure leaves the cursor unchanged. Sequence, raw length, slot,
canonical IDs and stored kinds/states are checked before advancing.

The current raw schema is an append-only journal bounded to4096 receipts. No raw
deletion or retention API exists. New admission assigns and validates an explicit
next sequence instead of relying on SQLite rowid fallback. Enumeration checks a
contiguous positive sequence; raw deletion/reordering or invalid inventory is
unavailable, with no schema migration. The scalar count/min/max inventory examines
at most4097 sequence entries. This is finite work and is not an O(1) guarantee.
SQLite internals and filesystem calls are separate from retained Rust page bounds.

`Store::reconcile` is now one projection turn, with at most eight imports. It
keeps an active scan and a completed prefix. Ready requires every captured ID to
have imported successfully and a current-tail check. A later tail is handled in
another turn. Errors discard the active cursor and leave storage unavailable;
reconstruction repeats exact IDs idempotently. It does not recreate raw callbacks.
Queries refuse while reconciliation is incomplete. Existing deleted, retained,
malformed, cancelled and status-report keys remain excluded from text.

The sole storage child retains an active receipt or read request across turns
and checks its original BOOTTIME deadline before the next turn. It completes the
exact admitted receipt only after full reconciliation, keeping source/generation/
slot/occurrence/receipt associations unchanged. Source attachment remains fenced
by the durable one-attempt marker; local and whole-service recovery open history
without implicit Binder re-registration. The lifecycle authority, publication
stamp, ACK/send fences and normal history-only production entrypoint are unchanged.

Per-turn work bounds do not prove bounded filesystem syscall completion. Existing
parent supervision and original deadlines can make a large cold backlog
unavailable; startup/read/arrival deadlines have not been enlarged. Raw capacity4096,
inbox4096 keys,2048 text messages and512 conversations remain finite refusal
bounds. Full enumeration establishes complete coverage inside those limits,
without introducing eviction, archival retention or production throughput claims.
The prior callback mutex wait and generated-decoder allocation caveats still apply.

The new executed component RED seeded33 actual SQLite occurrences and asserted
complete projection/Ready; the frozen implementation refused. A separate new
actual ARM RED submitted the generated33rd callback while the source was live:
raw33 committed, projection stayed32, completed remained32 and receiving failed.
The new desired assertion required complete closed-window projection and source
availability, rather than relabelling the historical snapshot33 refusal as a new
failure. Both pre-implementation sources and exact selected binaries are retained.

New synthetic ARM integration uses the immutable `f733d3b` lifecycle actor and
receive-only NDK peer so separate session-authority work cannot change the test
boundary. It qualifies the receipt/store/worker increment, not the later session
composition or a production vendor. Original runtime bytes stay private, the VM
has no NIC/host Binder/adb/USB or phone access, and independently owned synthetic
peers/managers are retained until actual reaping before explicit context reset.

The actual ARM green passed six integration groups:64 closed-window equal
callbacks and complete bounded native-client history pages;67 keys over two slots
with malformed/status exclusions; child/whole history-only restart; raw-commit
projection gap and completion loss recovered across34 IDs; and real external
SQLite refusal followed by exact33-ID recovery. Four independently owned peer
contexts were actually reaped/reset. Actual ARM and x86 passed22 receipt cases
and six projection cases, including4096 IDs over128 pages, fixed-upper tail
separation, stale-owner rejection, corruption without cursor advancement and
persistent deletion/retention exclusions. [Clean provenance](evidence/native-messages-enumeration-provenance.json)
records immutable pre-fix and current selectors without publishing runtime bytes
or private logs. No commit or production enabling is implied by these fixtures.
