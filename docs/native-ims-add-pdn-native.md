# Native add-PDN request encoder

`native-ims::add_pdn` serializes a bounded typed request105 payload. Five complete
outputs match the separately qualified original getBytes vectors byte for byte
on x86 and ARM64 user-QEMU. All35 native-ims tests pass on each target, with no
ignored tests, and strict formatting, Clippy and release builds pass.

This is a pure codec. It does not acquire connectivity, submit a request, settle
a response, create registration prerequisites or qualify a physical phone.
The existing radio registration view remains unavailable in ordinary owners
until a real production APN-context writer exists.

## Boundary and evidence

The [original probe](native-ims-add-pdn.md) executes unmodified original request
construction,40 setters and getBytes on SDK37 x86. That run is not repeated or
relabeled as ARM testing here. The new [fixed Rust tests](../native-ims/tests/add_pdn.rs)
read the [selected vectors](../native-ims/tests/fixtures/shannon-add-pdn.tsv), whose
seven complete hashes match the original bounded provenance. Expected bytes were
captured before the encoder existed and remain unchanged across RED and GREEN.
Raw records, original APK, decompilation and private method notes stay private.

The frozen baseline validates inputs but has no positive serialization. Actual
x86 and ARM user-QEMU tests each produce six policy passes and five missing-encoder
failures, with compilation and lint already passing. Only the serializer source
changes for repair. The same five complete comparisons then pass, along with
six policy tests and all24 previous tests:35 per target. Real compiler-selected
ELFs are checked as x86 machine62 and ARM64 machine183. ARM uses
`qemu-aarch64 -L /usr/aarch64-linux-gnu`; no guest kernel, Binder or Pixel execution
is claimed. Both containers have actual CLI waits, live exact CID registration,
fresh absence, same-daemon checks and unchanged read-only input closure.

An earlier private formatting prerequisite failed Clippy before behavioral tests;
it is retained and is not counted as RED. The actual failing comparison pair is
identified in the [native provenance](evidence/native-ims-add-pdn-native-provenance.json).

| Original input | Native result on both targets |
| --- | --- |
| Numeric IPv4/IPv6, one P-CSCF, wire0, fixed time | Exact683 bytes |
| Empty P-CSCF, wire2, fixed time | Exact663 bytes |
| Empty P-CSCF, wire2, null optionals | Exact663 bytes |
| IPv4 P-CSCF count8 | Exact695 bytes |
| IPv6 P-CSCF count8 | Exact791 bytes |
| IPv4 P-CSCF count9 | Explicit TooManyIpv4; original input was clamped to8 |
| IPv6 P-CSCF count9 | Explicit TooManyIpv6; original retained9 |

## Native input policy

Thirty-eight typed fields replace40 original setters because both list counts
come only from their actual slices. Each address family is limited to eight;
the maximum output is823 bytes. Addresses are numeric `Ipv4Addr`/`Ipv6Addr`
values, MAC is exactly six bytes or absent, and selectors0/2 remain opaque wire
values. No cellular/WiFi meaning is inferred from them.

MCC/MNC is six ASCII decimal characters, country at most two ASCII bytes, and
E911 aid at most32 ASCII bytes. Explicit valid Gregorian components use
2000–2255 and a signed16 whole-minute UTC offset. The API does not perform DNS,
calendar normalization, charset selection, wall-clock or timezone conversion.
Invalid dates/text are rejected even when selector0 would ignore those fields.
These are deliberate native domain restrictions, not broader original-code
behavior equivalence or evidence that a modem accepts arbitrary allowed inputs.

The wire preserves numeric MCC/MNC digit bytes, network-order IP octets,
little-endian scalars, signed date offsets, the main TAC low16 bits and its full32
TLV value. Logical additional-data lengths12/48 precede the entire500-byte padded
backing array. Empty/null fields produce explicit zeros; no previous request's
optional data is cached. The finite comparisons do not qualify every possible
accepted input or combined maximum address lists.

## Later Binder composition and remaining gates

The later [typed transport tracer](native-radio-add-pdn.md) now connects this
encoder through a typed current-session adapter and actual ARM Binder to an
independent exact code105/slot1/663-byte capture. Three fresh VM groups qualify
owned dispatch, cancellation and real BOOTTIME expiry after genuine Unsupported
RED. Its kernel/Binder evidence is separate from this pure component checkpoint.
Pre-dispatch refusal remains NotSent; dispatched operations remain OutcomeUnknown
through cancellation, peer death and subsequent Stop. A returned transaction or
captured payload cannot
establish APN acquisition or IMS readiness. Installed command ingress still needs
durable uncertainty and current permissions before dispatch.

Real APN/network ownership must supply these fields. Subscription/provider and
worker incarnation, original response/reset semantics, installed services/apps,
modem-facing vendor userspace replacement, carrier behavior and matched Pixel
hardware remain open. No physical phone was accessed.
