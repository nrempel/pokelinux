# Native incoming receipt and test ACK ownership

The Linux `ReceiptOwner` persists bounded incoming GSM occurrences in a private,
exclusive SQLite store. Its acknowledgement API is explicitly limited to injected
or synthetic **test backend** scope. It connects no production modem ACK adapter,
infers no receive deduplication, and claims no modem acknowledgement success.
The recovered [transport contract](native-sms-transport.md) remains the source of
the unresolved backend gates.

Each admitted arrival gets a fresh local UUID, complete raw bytes, `Deliver` or
`StatusReport` kind, explicit slot, owner UUID, source incarnation, and backend
scope. Equal bytes remain separate receipts. These UUIDs identify local
observations and commands; they supply no durable modem receive key. Parsing,
text assembly and delivery correlation are separate: storing a status report
neither creates incoming text nor completes an outbox part. Current TP-MR zero
and later reference reuse cannot establish unique delivery.

The store admits at most 4,096 occurrences of at most 512 raw bytes, including
empty or malformed bytes. Capacity or store failure releases no ACK authority.
It uses a canonical private owner directory, owner-only regular files, a permanent
exclusive file lock, rollback journal, `synchronous=EXTRA`, and immediate
transactions. Admission verifies exact bytes and provenance before committing;
this equality check validates storage, never deduplicates content. Snapshots omit
raw bytes and return at most 32 summaries. A zero SQLite busy timeout bounds lock
contention. `control()`, permission revocation and `stop()` perform no database I/O.
Stop revokes all ACK windows immediately, while bounded late arrivals can still
be stored in quarantine. It cannot reattach or regain permission.

ACK permission defaults to denied. Only an explicitly attached test source with
current permission can open a live window. SMS and status-report arrivals share
one window per slot. A second arrival before qualified completion stores another
occurrence and revokes that source's window. Replaced callbacks, source loss,
permission changes and stop revoke local authority. Stale or foreign-source raw
arrivals are still stored without transferring authority to a newer source.
Factory ordering and cross-object callback ordering remain unqualified; the
conservative window can quarantine a legitimate next arrival that races the
previous command's final response.

`claim_test_ack` commits `Uncertain` with a fresh attempt UUID before returning a
non-cloneable permit. Dropping, invalidating or dispatching it cannot refund it.
`dispatch_test_ack` consumes it through the owner and rechecks permission, source,
window, cancellation, attempt and durable uncertainty immediately before the test
adapter. Cancellation revokes the affected live source before attempting its own
durable update, including when that update fails. Later raw callbacks on that
source stay quarantined. Explicit test reattachment supplies fresh local authority
only; it establishes no vendor producer drain/reset or network-message identity. Dispatch failure keeps
uncertainty and revokes the source. Restart recovers raw receipts and states,
never incoming windows or dispatch authority.

One-way IPC return and intermediate `acknowledgeRequest` are progress only.
An exact adapter-qualified test final response can record `TestBackendAccepted`
for its original dispatched attempt. Duplicate, old-source, cancelled, undispatched
or late-after-death results are ignored. Storage failure emits no accepted result.
The dedicated synthetic messaging V3 adapter binds command serials to attempts,
keeps serials unique within its bounded callback incarnation, and accepts only
`acknowledgeLastIncomingGsmSmsResponse`, `SOLICITED`, and `RadioError::NONE`.
This describes its test policy; it proves no modem receive correlation. Its chosen
success flag and unused cause are synthetic values, not a production ACK policy.
`UNSOLICITED_ACK_EXP` raw arrivals are quarantined because the separate
`responseAcknowledgement()` obligation is unqualified. No generic ACK path exists.

| Event | Durable and local result |
|---|---|
| Arrival commit fails or bound is exhausted | No test ACK eligibility; source revoked. |
| Receipt committed, permit absent | Raw stays stored; restart cannot reconstruct its window. |
| Permit claimed, dropped, dispatch lost or outcome unknown | `Uncertain` remains; no implicit retry or reconnect replay. |
| Equal-looking arrival, including after restart | Fresh local occurrence; no deduplication or permit transfer. |
| Exact test final accepted | Only its original test command records `TestBackendAccepted`; no network success claim. |
| Cancellation, death, replacement, permission revocation or stop | Held dispatch and callbacks fenced; raw history retained. |

The authored SQLite tests have runtime red-green evidence for missing permit
ownership, a real child dying before the claim, trigger-mutated admission
metadata, and cancellation reopening a source window. Native x86 tests include a real owned child SIGKILL/reap and restart;
ARM64 user-mode results exclude those process tests. A separate networkless
ARM64 system-QEMU guest executes the exact corrected binary as unprivileged
`alarm`; all19 tests, including SIGKILL/reap/restart, pass under its real Linux
7.1.6 kernel. This is Linux process evidence, separate from x86 Binder and
physical modem evidence. The dedicated frozen Binder
runner uses an independent original NDK synthetic endpoint and generated Rust
callbacks, real SQLite and kernel Binder on an owned SDK emulator. Evidence and
exact source/binary identities are recorded in
[receipt provenance](evidence/native-sms-receipt-owner-provenance.json).

Factory callback registration, incoming-window ordering across reset, exact
producer lifetime, success/failure-cause policy, durable redelivery identity and
ACK idempotence remain required before any production ACK adapter. A production
receive path also needs a private retention/deletion policy and an explicit
nonblocking worker/queue integration. This bounded owner grants no exactly-once
receive or carrier delivery promise.

Run the real ARM64 process qualification with
`python3 tools/receipt-process-vm.py` in the existing graphics simulation image.
The guest checks final selection, Cargo artifact, implementation/test hashes and
full test counts. Its root disk uses a disposable snapshot, project export is
read-only and the VM has no NIC. Existing status-report and superseded receipt
proofs remain unchanged.
