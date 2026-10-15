# Lock admission waits for storage capacity

At planning time, an authorized lock request now refuses successful acknowledgement
if the lifecycle actor owns an unresolved storage operation. This includes a repeated
lock whose cached policy already says `lock_pending=true`. The actor still fences
memory authority and advances the lock epoch before checking capacity. Idle,
already durable locks retain their successful acknowledgement without another
write. Wrong credentials and invalid packets remain denied before planning.

The actual host daemon/socket regression exposes the previous behavior: with
root-seeded pending intent, an ordinary grant replacement is synced and its exact
storage child held by the existing isolated SIGSTOP hook before its response.
The UID1000 lock request incorrectly returns `ok` while the actor remains ready,
nondegraded and storage-busy. After the guard, the unchanged fixture returns
`unavailable` for both pending-true and pending-false cases. The latter still
locks memory immediately while its refused admission leaves disk pending-false.
This is genuine behavioral RED→repair→GREEN at the production socket boundary,
not a missing-API compile failure or an authored authentication result.
[Bounded provenance](evidence/native-lock-storage-barrier-provenance.json)
records the exact inputs and failed prerequisite separately.

The grant connection remains open and unanswered throughout the lock request;
its timeout or EOF cannot supply a different cause for refusal. Both before and
after snapshots require ready/nondegraded/busy state, and the exact worker stays
stopped with no successor. Client UID/GID1000, groups[] and CapEff0 are recorded.
The original one-second write deadline is unchanged. Snapshot and Stop remain
responsive; the grant returns unavailable during Stop, both actor parents wait0,
and the exact storage pidfds are dead with proc entries absent. Storage Child
waits belong to the actor; the fixture observes death rather than independently
recording those numeric wait statuses. No forced cleanup counts as success.

All 40 existing host lifecycle tests also pass, with no ignored or filtered tests,
formatting and strict default/all-target/all-feature Clippy. The ordinary daemon
and library tests cross-compile to AArch64; this supplies no new ARM runtime
evidence. A runner prerequisite failed on Docker's transient empty CID file
before container start. Its nonce-qualified created container was removed and
fresh absence checked; the lost CLI parent wait is unqualified. That label is
neither behavioral RED nor GREEN. The corrected runner owns actual CLI waits,
freezes inputs outside the writable results bind and checks fresh exact-CID
absence on the same daemon. No independent container wait is claimed here.

This is a decision-time check, not complete send-time exclusion. A successful
reply queued while idle can still be published after another connection starts
a write. That separate publication gate needs its own actual regression and
repair; these held-before-request cases cannot qualify it.

This advances the boundary needed before authenticated durable clear exists.
It does not implement clear serialization: a future in-flight clear must resolve
truthfully, keep reads denied, and either quarantine uncertainty or sync a newer
pending-true replacement before a later lock can succeed. In-memory rejection of
a stale completion cannot undo bytes already committed to disk. Stop with an
unresolved write is unclean and proves no healthy same-directory restart.

These are headless host controls with an ordinary grant write and direct JSON
clients. They run no Source, normal Messages page, `lifecycle-lock` executable,
original effect, PAM, authentication scope, broker, armed receipt or unlock clear.
Source100ms is unchanged; no suspend, matched Google kernel or phone is qualified.
The public [fixture](../tools/native-session-lock-admission/storage-barrier.py)
and [runner](../tools/native-session-lock-admission/storage-barrier-run.py) contain
only authored code; private ELFs, images and raw logs are not published.
