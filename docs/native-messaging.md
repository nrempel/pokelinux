# Native messaging

The device-neutral Rust crate `native-messaging` implements bounded SMS-SUBMIT
payload encoding, multipart planning and a durable single-owner outbox. These
components do not form an installed messaging service or working modem path.

## Implemented contract

Use the default GSM alphabet where every character is representable; otherwise
use UTF-16BE consistently across the entire message. A single part supports
160 GSM septets or 70 UTF-16 code units. Multipart messages use a 16-bit
concatenation reference and seven-byte header, leaving 152 septets or 66 UTF-16
units per part. Splitting preserves GSM extension pairs and Unicode scalar
boundaries, including surrogate pairs. It does not promise grapheme boundaries.

The pure planner accepts an explicit reference, caps input at 16,384 UTF-8 bytes
and output at 32 parts, and rejects empty intent or malformed recipients. The
current recipient contract permits 1–20 ASCII digits and an optional leading
plus; alphanumeric addresses, dial commands and national-language shift tables
are not implemented. Each TPDU excludes the SMSC address and uses TP-MR zero.
A transport must establish correlation; a payload does not establish submission
or delivery. The caller must durably reserve the concatenation reference before
transmission. This crate performs no I/O, permission admission or retry.

GSM character mapping reuses the pinned CC0 `sms-pdu` 1.1.0 implementation. Actual
AOSP execution exposed its omitted form-feed extension entry; a narrow wrapper
adds that standard entry. The rest of the alphabet remains in the upstream crate.
Dependency identity is recorded in `Cargo.lock` and the evidence manifest.

## Incoming text decoding

The bounded SMS-DELIVER decoder parses a complete GSM PDU with its SMSC-length
prefix. Numeric and GSM alphanumeric senders, default GSM/UTF-16 text, reply-path
flag, service-centre time and 8/16-bit concatenation metadata are supported.
Reference width remains explicit so an 8-bit reference cannot collide silently
with a 16-bit reference. Sender and timestamp are untrusted network content.

Framing caps the input at 176 bytes and user data at its GSM/UTF-16 bounds. It
rejects truncation, excess trailing bytes, invalid calendar/BCD, broken UTF-16
or escape sequences and inconsistent/duplicate concatenation headers. Binary,
message-waiting, nonzero PID and application/port/language headers need a
qualified separate path and currently return an explicit unsupported error.
Malformed input is not made compatible with Android's lenient parsing.

GSM mapping reuses the pinned upstream decoder with the verified form-feed
wrapper. Calendar/offset handling uses pinned `time` 0.3.55. Our framing and
signed quarter-hour handling are explicit; the upstream complete-PDU parser
does not enforce the needed length/timezone contract. Thirty-three synthetic
inputs were parsed by actual AOSP `createFromPdu`; native text, sender, time,
reply-path and concatenation match those outputs on x86 and ARM64 user-QEMU.
Five boundary tests include all truncated prefixes and bounded byte mutations.
See [receive provenance](evidence/sms-receive-provenance.json).

This supplies parsed parts only. Durable receive admission/deduplication,
multipart lifetime/collision handling, modem acknowledgement, background
notifications and carrier delivery remain separate work.

## Durable outbox

The Linux outbox owns a permanent lock inode and a private SQLite state directory
(0700 directory, 0600 regular files, no symlinks/hard links). It checks the actual
SQLite DELETE/EXTRA/foreign-key configuration and uses immediate transactions.
This excludes other UIDs; it does not isolate applications sharing the service UID.
Authenticated IPC and application permissions remain separate integration work.

Preparation stores the intent, digest and complete part payloads while reserving
a 16-bit reference atomically. Duplicate IDs return the same receipt; changing
an intent under the same ID fails. References never wrap or reuse in this store.
At most 4,096 message receipts are admitted; SQLite also has a 32,768-page
budget. Exhaustion refuses new admissions rather than deleting uncertainty.
There is no migration, receipt deletion or automatic reference-space reset.

Explicit claim commits `Uncertain` before exposing a non-cloneable owner-specific
dispatch permit. Claim/drop, cancellation and restart cannot refund it. A stale
permit from a previous owner cannot finish a restarted outbox. Subsequent parts
require earlier transport acceptance. Cancellation is durable and remains set
even when a held request succeeds later. `TransportAccepted` means a qualified
transport acknowledgement, never delivery; `NotSubmitted` requires an adapter
proof and still grants no retry of the original part. No transport is attached
yet. A prepared part requires an explicit owner claim after restart.

Twelve tests use real SQLite files and cover duplicate admission, rollback,
reference/capacity exhaustion, restart, cancellation, stale permits and unsafe
lock files. An injected SQLite `RAISE(IGNORE)` exposed a permit despite an
unchanged row; the red-green correction verifies the write count and actual
uncertainty before committing. A native child is SIGKILLed after claiming a part;
the next owner opens successfully while both retry and continuation remain denied.
That process boundary passes on x86 Linux and under an actual ARM64 Arch kernel
in system-QEMU as unprivileged `alarm`. User-mode QEMU runs the database tests
but explicitly ignores process tests.

See [outbox provenance](evidence/sms-outbox-provenance.json).

## Executed evidence

`framework/shannon-oracle/SmsCodecOracle.java` invokes the actual Android
`com.android.internal.telephony.gsm.SmsMessage.getSubmitPdu` encoder through
reflection in the owned SDK37 emulator, with SELinux enforcing. It does not
discover a radio service or send an SMS. Twenty-seven synthetic fixtures cover
GSM basic/extension/control characters, Unicode, emoji, combining characters,
address padding, status-report requests, single-part limits and 16-bit
concatenation headers, including an ASCII fragment forced to UTF-16.

The public fixtures contain synthetic inputs and encoder outputs only. The
private framework JAR is hashed and remains ignored. Fixture comparison on a
new machine replays recorded outputs; it does not rerun Android's original code.

Strict red-green tests first failed with missing native encoding/planning, then
the original-code comparison exposed the form-feed discrepancy. Final checks
pass formatting, clippy with warnings denied, all 27 byte comparisons and five
planner tests on Linux x86-64 and ARM64 under user-mode QEMU. ARM64 here is codec
execution only, not system-QEMU Binder or hardware evidence.

```sh
bash tools/build-native-messaging.sh prepare
bash tools/framework-suite.sh sms-codec
bash tools/build-native-messaging.sh build
# In the prepared network-disabled graphics simulation container:
# Prepare exact suspend artifacts for the shared process runner first:
# bash tools/build-native-suspend.sh arm64-process (on Harbour)
python3 tools/native-process-vm.py
```

See [provenance](evidence/sms-submit-provenance.json).

## Submission-result owner

The device-neutral `SubmissionOwner` retains non-cloneable dispatch permits
and fresh source/attempt identities. Cancellation and local source revocation
remove pending ownership while keeping durable uncertainty. One-way return and
intermediate acknowledgement do not settle a part. Qualified exact final results
may persist TransportAccepted; duplicates, consumed stale callbacks and failed
SQLite acceptance cannot. Seven additional ownership tests pass on x86 and
ARM64 user-QEMU; ten actual frozen NDK/Rust Binder groups qualify the synthetic
adapter separately. See [transport qualification](native-sms-transport.md).
No modem adapter or receive acknowledgement replay is enabled by these tests.

## Remaining integration

Implement authenticated permission admission, durable conversations/drafts,
transport receipt qualification, receive deduplication, uncertain-result
reconciliation, a background
owner and native Messages UI using the live Omarchy theme. Recover the actual
modem/IMS send, receive and status-report contracts. The
[SMS transport report](native-sms-transport.md) now recovers the frozen standard
V3 ABI and records50 original Shannon codec/tracker outcomes plus8 message IDs.
Those tests do not qualify a modem adapter, receive acknowledgement replay or
unique delivery correlation. Test installed service and
UI restart, locked/closed-window notification behavior and revocation. No
implicit retransmission is permitted after an uncertain send. Audit MMS, SMSC,
carrier policy and national-language encodings before promising support.

Actual Pixel modem acceptance, incoming messages, delivery reports, carrier
compatibility and suspend/wake remain hardware gates. No phone was accessed.

The [native status-report decoder](native-sms-status-report.md) now passes57 actual
AOSP comparisons and12 focused native tests. It preserves untrusted status and
both timestamps, with explicit stricter malformed-input rejection; it never
correlates or finalizes outbox delivery. The [incoming receipt owner](native-sms-receipt-owner.md) now qualifies distinct
raw storage, durable uncertainty and test-only ACK fencing with19 SQLite tests on
x86 and a real ARM64 guest kernel plus12 actual frozen Binder groups. Production
ACK identity/policy, installed receive workers and app integration remain open.
