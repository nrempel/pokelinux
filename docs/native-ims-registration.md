# Native registration-indication wire decoder

The Rust `native-ims::registration` component decodes the payload consumed by the
retained factory `RilIndRegistration.update`. It returns raw immutable fields
only after the complete selected range validates. It does not publish IMS state,
interpret state/RAT/features, validate a URI identity or activate a session.
The Shannon-specific layout stays in this backend, outside shared phone policy.

The later [original raw routing qualification](native-ims-registration-routing.md)
adds17 dispatch cases and8 separate parser calls: exact class wire code3012,
configured-slot filtering and complete-array delivery. It does not rerun the121
parser comparisons below, execute full semantic channel dispatch or connect the
native owner in that original-reference run. The later
[native owner](native-radio-registration-owner.md) now separately qualifies real
ARM Binder typed publication. The separate [installed slot1 tracer](native-radio-registration-installed.md)
now qualifies typed IPC/status, explicit packaged slot and retained history after
peer-loss withdrawal. Original producer/reset and decoded domain meanings remain
open.

## Original execution and native policy

The exact retained Shannon APK is pinned by SHA256 in the provenance report.
REA inspected the indication constructor/update and its unsolicited-message base
constructor; raw provider outputs remain private. A separately authored Java
reflection harness then executed original constructor/update/field reads with a
null adaptor channel and synthetic bytes in the owned SDK37 x86 emulator, with
SELinux enforcing. No APK installation, message dispatch, vendor service lookup,
modem request or physical phone operation is part of this proof.

There are 121 completed parser probes, plus one constants-metadata record. A second
qualified replay reproduces every observation exactly. All original calls return
normally to the harness, including malformed cases: the original catches IO errors,
can leave partially changed base fields, retains old missing optional fields, clamps
large URI/reason counts, and can ignore leftover bytes. This is original parser
behavior, not evidence that a modem accepted a request or registered IMS.

| Native treatment | Cases | Qualification |
| --- | ---: | --- |
| Complete admitted range | 37| Present scalar fields match actual original observations. Exact raw body slices are retained; valid UTF-8 nonempty bodies match original text bytes. |
| Truncated header/body | 40| Returns an error without exposing a partial result. |
| Leftover extension bytes | 30| Refuses a complete packet even though the original returns normally. |
| Oversized reason declaration | 7| Refuses instead of clamping to 256. |
| Oversized URI declaration | 4| Refuses instead of clamping to 1000. |
| Invalid selected range | 2| Refuses negative/out-of-buffer lengths instead of stream clamping. |
| Input over 4096 bytes | 1| Refuses before parsing; 4096 is local policy, not a vendor packet maximum. |

Seeded original probes demonstrate old optional values surviving later updates.
Native absent fields are `None`; no old value is carried. The original turns an
empty combined body into `UNKNOWN` and replaces invalid UTF-8 in its default-charset
String conversion. Native exposes neither sentinel nor replacement text: zero
counts yield empty slices, while NUL/non-UTF8 bytes remain exact. Production text,
URI, carrier and identity rules require separate recovery.

## Qualified layout

The twelve-byte prefix is little-endian and contains, in order: raw u8 state,
u8 feature, i32 feature mask, u16 status, u16 URI byte count and u16 reason byte
count. The two byte fields follow using their exact declared boundaries. Native
refuses counts above the observed original instance limits 1000/256.

Extensions follow the original greedy presence checks: read an i32 mask if at
least four bytes remain, then an i64 delegate mask if at least eight remain, then
a u16 feature extension if at least two remain. The native complete tails are
therefore 0, 2, 4, 6, 12 or 14 bytes. This is an observed layout, not invented version
negotiation. A final `DataReader::finish` must succeed before the result exists.
No allocations, String conversion, mutable previous message or permission token
are retained by the decoder.

Original optional-extension dynamic probes use positive values. Signed/unsigned
operations agree with recovered direct reads and the separately qualified scalar
reader; this checkpoint does not claim exhaustive original extension numeric
boundaries. Raw numeric values are not valid domain states or feature support.

## Executed red-green evidence

Eight actual runtime tests failed against the unimplemented decoder before the
implementation. The exact authored stub was later reconstructed and its SHA256
matched the recorded executed source; a frozen replay with the formatted final
tests again fails all eight groups. Formatting changes to the tests are identified
separately from the first red source hash.

All eight registration tests, ten reader tests and six prior codec/request/unit
checks pass on x86 and ARM64 user-QEMU: 24 checks per target. Formatting, Clippy
with warnings denied and both target release builds pass. Compiler-selected test
executable hashes and source snapshots are retained privately. That decoder-only checkpoint provided no
ARM64 guest-kernel/Binder or hardware execution. The later installed report
qualifies one authored complete payload through the actual ARM path separately.

Three incomplete original harness trials exited 137 without a completed result.
The first trial stdout was not captured. The two captured trials have no parser
records; the traced trial reached constant reads. The corrected probe supplies an
original object when reading the nonstatic limit fields; it then completes. These
incomplete trials provide no decoder red, vendor rejection, registration outcome
or cleanup evidence.

```sh
# Unique evidence directory; owns only the SDK emulator and suite lock.
bash tools/native-ims-registration-suite.sh parser
# Native-only isolated tests; the first run refuses existing green evidence.
bash tools/native-ims-registration-tests.sh green
```

Clean current identities and precise limits are in
[evidence/native-ims-registration-provenance.json](evidence/native-ims-registration-provenance.json).
The generic reader remains documented in [native-ims-reader.md](native-ims-reader.md).

Original production producer/reset identity and endpoint policy, final response/error
correlation, registration/RAT/bearer/SIM semantics, call/media/carrier operation,
wider app permission/lifecycle composition, matched Pixel policy and physical
execution remain open. A complete wire payload is not an authenticated
phone-state event. The existing raw owner and durable uncertainty fences remain
required; this decoder changes none of their recovery guarantees.
