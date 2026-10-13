# Native bounded IMS wire reader

The independently authored `native-ims::reader::DataReader` reads qualified
little-endian signed/unsigned scalars and opaque length-prefixed CP fields.
It uses immutable borrowed bytes, one cursor and a permanent first-error fence;
there is no Java, stream, allocation, reset or copy-constructor implementation.
The local input limit is4,096 bytes, including any backing array supplied with a
selected view. That is a native admission policy, not a recovered vendor maximum.

REA6.4.0 with the pinned JADX0.7.1 provider inspected the factory reader method
table, range constructor, long/bulk/skip getters and two CP getters. The raw
results and original APK remain private. Public Android
[DataInputStream](https://developer.android.com/reference/java/io/DataInputStream)
and [ByteArrayInputStream](https://developer.android.com/reference/java/io/ByteArrayInputStream)
are the reference stream interfaces; those interfaces alone do not establish
this wrapper's byte order or bookkeeping. Actual original bytecode execution
supplies the comparison evidence below.

The authored reflection oracle invoked only the original reader with synthetic
bytes in an owned x86 SDK37 emulator. It neither installed/started Shannon IMS
nor called any service or modem. SELinux was enforcing. All247 cases completed:
scalar sign/endian boundaries, range construction, bulk reads, skips, truncated
inputs and both signed CP-length fields. The native comparison matches148 valid
original values and remaining-byte counts. The CP oracle returns
`Arrays.toString(byte[])` diagnostics, including signed byte values and NUL;
the native API therefore returns exact opaque bytes or absent for a zero prefix.
No UTF-8, NUL termination, registration-state or call-state meaning is inferred.
The public fixtures contain only authored synthetic inputs and their normalized
observations; they contain no factory bytes or proprietary implementation.

The same244 bounded fixtures additionally exercise96 refused cases. Three
original inputs exceed the native input limit and are recorded separately.

| Native refusal | Cases | Explicit difference or failure rule |
|---|---:|---|
| Truncated |61| Includes original EOFs and accepted oversized skips. Preflight keeps the cursor unchanged; all later getters fail with the first error. |
| Negative length |5| Rejects negative counts and signed CP prefixes that the original can treat as absent or reflect in incorrect bookkeeping. |
| Invalid selected range |30| Rejects negative/out-of-array/overflowing views instead of accepting or clamping them. |
| Input too large |3| Rejects the whole input before creating a view; original large synthetic CP inputs remain private evidence. |

No backwards compatibility is attempted for malformed behavior. `finish()`
consumes the reader and rejects trailing data or any earlier error. Individual
successful getters do not authorize publication: a future semantic decoder must
validate the entire packet and its response/producer identity before publishing.
The original reader's byte-array/copy constructors, mutable-array aliasing,
exception-side cursor effects and diagnostics are not retained as native APIs.

Strict TDD produced an actual exit101 scalar assertion, then ten runtime contract
failures against the unimplemented reader before the implementation. Ten focused
reader tests now pass on x86 and ARM64 user-QEMU, including148 original-code
matches, malformed fences, exact64-bit boundaries, NUL/non-UTF8 preservation and
input/view bounds. All five existing writer/message/segmentation tests, formatting,
clippy and both release builds also pass. The initial clippy-only test-helper
failure is retained separately and does not count as a behavioral red.

Run `bash tools/native-ims-reader-suite.sh` for original-code evidence using the
exact private factory APK, then `bash tools/native-ims-reader-tests.sh` for Rust
comparisons. The emulator runner holds `android-framework/suite.lock`, uses an
internal-only network, and reaps its container/network on exit. It never invokes
host adb or a physical phone. ARM64 user-QEMU checks portable decoder code only;
there is no ARM64 Binder or hardware qualification in this checkpoint.

The later [registration payload decoder](native-ims-registration.md) qualifies
one complete original-compared layout with stricter malformed-input policy.
Other IMS response layouts, request/error/slot/producer identity, valid state/RAT
semantics, SIM authentication, bearer/registration/call sequencing, IMS media,
carrier provisioning and remote cleanup/reset remain open. This component is
not an installed IMS service or a production response publication path.
