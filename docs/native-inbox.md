# Native local inbox projection

`native-inbox` projects committed local receipts into private SQLite conversations.
It is a device-neutral Linux component, with no radio lookup, acknowledgement,
transport retry, delivery correlation, notification or installed Messages service.
It reads raw bytes only through `ReceiptOwner::lookup`; the raw PDU and original
receipt evidence remain solely in the [receipt owner](native-sms-receipt-owner.md).
The inbox stores decoded text and metadata, local provenance, and exclusion keys.
It stores neither raw PDU bytes nor a raw/body digest.

## Local identity and display grouping

`import` accepts a typed local `ReceiptId` and the source `ReceiptOwner`. The UUID
is copied unchanged into `ReceiptKey`; no identity is derived from content. A
second import of the same committed occurrence is idempotent. Equal raw bytes
under different receipt IDs create distinct messages, including after restart.
Changed source provenance or decoded content under an existing key is refused.
Receipt ACK state is irrelevant to projection and supplies no authority here.

Conversations group by slot, exact decoded sender string and GSM address kind:
unknown numeric, international numeric, national numeric or alphanumeric.
International plus signs, leading digits and alphanumeric case are preserved.
Numeric-looking alphanumeric labels remain separate from numeric senders.
This is untrusted display grouping, not carrier, subscriber, contact or E.164
identity. Sender labels and service-centre timestamps remain network content.
Local insertion ordinal orders messages and retention. Network time never decides
which arrival is newer for retention.

Only the existing qualified `decode_delivery` can produce text. The projection
preserves encoding, service-centre time, reply-path flag, message class and explicit
8/16-bit concatenation metadata. Multipart occurrences remain independent text
parts; sender/reference coincidence does not justify joining them into one message.
A status-report receipt, malformed or unsupported PDU, or durably cancelled receipt
gets an exclusion record and never becomes text. An empty supported text remains
an empty message, matching the recorded original parser output.

## Access and durable removal

The exclusive store requires a canonical owner-only `0700` directory and `0600`
regular single-link files, with a permanent lifetime lock. SQLite uses a rollback
journal, `synchronous=EXTRA`, `secure_delete=ON`, foreign keys, a zero busy timeout
and a bounded page count. Schema version1 has no migrations or compatibility path.
Admission verifies projected fields and provenance within the committing transaction.
Ignored/mutated inserts or removal operations emit no successful result.

`Control` grants local import and read leases separately; both permissions default
to denied, including after restart. Revocation/regrant changes the role's generation.
Old, foreign-owner and pre-restart leases fail. Stop permanently denies both roles.
Authority exhaustion stops the component instead of preserving an old grant.
The control object is a local trusted-code policy mechanism; it proves no Linux
peer credential, app permission or lock-screen authorization. Same-UID processes
can access owner files; installed service UID and ingress policy remain gates.

Read access is checked before and after storage work before returning text or
metadata. Import/removal authority is checked before admission, before commit and
after commit. SQLite progress-hook tests revoke during query/transaction work;
a commit-hook test revokes during commit and verifies an actual durable row still
exists while publication returns an error. Revocation can change local control
immediately, but cannot interrupt the in-progress synchronous filesystem call or
undo an already completed commit. Previously returned text cannot be erased from
a caller's memory. The component supplies no asynchronous worker, bounded disk
latency, automatic notification publication or blocked-filesystem supervisor.

Receipt cancellation is read fresh on import. Re-import reconciles an already
projected cancelled receipt by removing its text. This is explicit reconciliation;
the inbox does not monitor `ReceiptOwner` for later changes. `delete` removes local
projection content without changing the raw receipt or its ACK state. `retain_latest`
removes at most32 text occurrences per transaction beyond the requested local
insertion-order retention count. Empty conversation labels are removed atomically.
Deletion/retention/cancellation keep occurrence keys, so re-import cannot resurrect
removed text. SQLite logical removal and secure-delete configuration do not promise
forensic erasure from disks, journals, snapshots or backups. Raw retention requires
its own separately authorized receipt-owner policy.

| Bound | Policy |
|---|---|
| 4,096 receipt/exclusion keys | Refuse new imports when full; never evict identity keys to make room. |
| 2,048 text occurrences | Refuse new text until explicit retention removes content. |
| 512 conversations | Refuse a new display group when full; existing groups can accept text within other bounds. |
| 32 query rows or retention removals | Explicit bounded page/batch; message pages use local ordinal. |
| 640 UTF-8 body bytes and64 sender bytes | Validate the qualified decoded projection and stored reads. |

Query pages are independent reads, not a durable snapshot spanning multiple calls.
Owner operations are synchronous and single-writer. Memory-only stop/control can
operate independently of storage; SQLite lock contention returns immediately.
These component properties establish no background/suspend or installed app proof.

## Evidence and remaining work

Executed runtime reds cover missing durable projection, a real child exiting
before its commit, three storage-time authority fences, retained private sender
labels during removal, and access-generation exhaustion. Real SQLite restart and
owned x86 SIGKILL/reap tests exercise persistence. ARM64 user-QEMU runs exclude the
two real-process tests. A separate no-network ARM64 system-QEMU guest executes
the three exact qualified binaries as unprivileged `alarm`; all20 tests pass
under Linux7.1.6, including actual child SIGKILL/reap/restart. This is Linux
process evidence, separate from recorded AOSP, Binder and physical modem evidence. Thirty-three recorded outputs from actual original AOSP
`SmsMessage.createFromPdu` execution are rechecked through durable inbox imports
and restart. This milestone reuses those recorded synthetic original-code outputs;
it does not rerun the Android oracle or establish new modem evidence. Exact source,
fixture, test and executable identities are in
[inbox provenance](evidence/native-inbox-provenance.json).

Installed credential/lifecycle and lock-state integration, a supervised storage
worker, raw-retention reconciliation, privacy-preserving app/notification delivery,
qualified multipart association and real modem/carrier receive behavior remain open.
No physical phone, proprietary target or vendor dispatch is used.

Run `python3 tools/inbox-process-vm.py` in the graphics simulation image for the
real ARM64 kernel proof. The guest verifies the original qualified binary
snapshots and restored-builder metadata selections before executing all tests.
The VM has no NIC, uses a disposable root-disk snapshot and exports the project
read-only. Existing original receipt/status evidence remains unchanged.
