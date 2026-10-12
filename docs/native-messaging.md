# Native messaging

The device-neutral Rust crate `native-messaging` implements bounded SMS-SUBMIT
payload encoding and multipart planning. This is the first native messaging
component, not a complete messaging service or working modem path.

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
```

See [provenance](evidence/sms-submit-provenance.json).

## Remaining integration

Implement authenticated permission admission, durable conversations/drafts and
operation receipts, duplicate handling, uncertain-result recovery, a background
owner and native Messages UI using the live Omarchy theme. Recover the actual
modem/IMS send, receive and status-report contracts. Test installed service and
UI restart, locked/closed-window notification behavior and revocation. No
implicit retransmission is permitted after an uncertain send. Audit MMS, SMSC,
carrier policy and national-language encodings before promising support.

Actual Pixel modem acceptance, incoming messages, delivery reports, carrier
compatibility and suspend/wake remain hardware gates. No phone was accessed.
