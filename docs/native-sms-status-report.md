# Native GSM status-report codec

`native-messaging::status_report` decodes complete SMSC-prefixed GSM
SMS-STATUS-REPORT bytes or bounded ASCII hexadecimal input. It returns untrusted
metadata only. It has no outbox, transport, delivery-finalization or modem
acknowledgement path.

The decoder retains a numeric SMSC address, message reference, numeric recipient,
service-centre receipt timestamp, discharge timestamp, raw status byte and report
qualifier. The service-centre timestamp is not a handset send-time measurement.
Numbers use the supported unknown/international/national ISDN type bytes; recipient
length is1..20 digits, SMSC storage is bounded to12 octets after its length prefix.
Both timestamps validate BCD digits, calendar dates and clock values, apply the
signed quarter-hour offset and return UTC epoch milliseconds. The two-digit year
cutoff matches AOSP:90..99 map to1990..1999,0..89 to2000..2089.

Optional PID and DCS retain presence separately from a default zero value. Optional
user data distinguishes an absent field from present empty text. Qualified text
support is default GSM7 and UTF-16BE, including the tested general-coding message
classes, without a user-data header. Four parameter-indicator octets are the local
maximum; continuation octets with unknown parameter bits are explicit unsupported
inputs. Binary/compressed/message-waiting alphabets and application headers are
also unsupported. The decoder requires complete consumption, bounds user-data
length and rejects odd/non-ASCII hex, trailing bytes, malformed BCD, invalid
calendars, orphan GSM escapes and malformed UTF-16.

The independent TPDU acceptance limit is164 bytes; the complete-PDU limit is177
bytes including the largest supported SMSC prefix. The TPDU budget is a conservative
limit derived from the status-report layout's variable recipient/optional-field
budget, rather than an assumption that all independent field maxima can occur in
one report. The supported subset follows the field layout in
[TS23.040 V19.0.0](https://www.etsi.org/deliver/etsi_ts/123000_123099/123040/19.00.00_60/ts_123040v190000p.pdf),
sections9.2.2.3 and9.2.3.11..27; it is not a complete standards conformance claim.

## Executed original-code comparisons

The isolated `SmsStatusReportOracle` invokes the owned SDK37 emulator's original
`com.android.internal.telephony.gsm.SmsMessage.createFromPdu`. Its full parser
consumes and discards TP-DT, so the harness also invokes the original private
`PduParser.getSCTimestampMillis` at the synthetic discharge timestamp. Thus TP-DT
comparison is original timestamp-decoder execution, not a full-parser accessor.
The public [Android17 parser source](https://android.googlesource.com/platform/frameworks/base/+/android-17.0.0_r1/telephony/java/com/android/internal/telephony/gsm/SmsMessage.java)
was retained privately with its hash; only the authored harness and synthetic
fixtures are checked in.

Actual execution produced126 synthetic result records. Native comparison checks57
supported reports field-by-field: SMSC, message reference, recipient, both UTC
timestamps, raw status, optional PID/DCS and optional text. They cover reference
zero/255, status ranges, odd/national/long addresses, complete SMSC prefixes,
leap dates/year cutoff, positive/negative quarter-hour offsets, optional absence,
GSM/Unicode limits, extension characters and bounded PI continuation.

The other69 inputs exercise61 malformed and8 unsupported native cases. AOSP parsed
41 of those; native rejection is intentionally stricter. Examples include invalid
calendar values that AOSP converts to timestamp zero, forced odd-address filler,
unknown optional encodings/PI bits, trailing data and an oversized TPDU. The original
fixture records preserve these differences instead of treating AOSP as a strict
malformed-input validator.

Runtime TDD first observed the missing decoder failing a valid report. A second
runtime red observed a well-framed165-byte TPDU passing through a short SMSC prefix.
After correction,12 focused tests and one executed-original fixture comparison
pass on x86 Linux and ARM64 user-QEMU, together with the full native messaging
regression, formatting and clippy checks. The actual AOSP oracle exited0 and its
owned emulator container/network were removed. Source, fixture, binary and run
hashes are in [provenance](evidence/native-sms-status-report-provenance.json).

Run `python3 tools/prepare-sms-status-report.py`,
`bash tools/framework-suite.sh sms-status-report`, then
`bash tools/build-native-messaging.sh build` on the prepared host. The original
runner writes its output into ignored `artifacts/sms-status-report`; the checked-in
fixture is a reviewed snapshot of that execution, not automatically overwritten.

## Remaining ownership gates

A raw status0 does not select a unique outbox part. Current submits use TP-MR zero;
reference reuse, colliding recipients and delayed reports need separately qualified
per-slot/per-attempt correlation. Calendar-valid timestamps are still untrusted
network content. No delivery correlation, native receive identity, ACK idempotence,
production service, physical phone or ARM64 kernel Binder qualification is added.
The proposed next [incoming receipt/ACK owner](native-sms-receipt-owner.md) preserves
each local arrival and durable acknowledgement uncertainty without deduplication
or replay inferred from PDU content.
