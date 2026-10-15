# Native SIT modern report projection

`native-sit-codec::modern_report::ModernSetupReport22::project` now converts the
complete typed base response into an owned modern-only report observation.
Fourteen captured original report comparisons match on the host and ARM64
user-QEMU. All40 crate tests pass per target after compiled host missing-report
RED, with formatting and strict all-targets Clippy. This composes the current
[base response parser](native-sit-codec.md#independent-server-families) with the
report projection; it does not open a modem transport or acquire an APN.

## Typed contract

The fixed comparison premises are version22, status override0, natural IPv6-first
order, matched fresh context, complete144-byte base input and empty dynamic
containers. There is no version switch or legacy report path. The function takes
one `BaseSetupResponse`, without reparsing raw bytes, and explicit presentation
index0–99. It refuses a larger index and inconsistent publicly constructible
observations: invalid numeric activity/selectors or family presence inconsistent
with protocol/server selectors. Such refusals are native policy. The nonzero
context remains a wire value, never a pending request or channel identity.

The owned value covers emitted cause, opaque i64, CID, activity, protocol code,
MTUv4/v6, optional interface/address strings and DNS/gateway/P-CSCF vectors. Fixed
empty arrays represent only the observed empty structured-address/QoS/TD
containers; unobserved fields and complete264-byte ABI equivalence are excluded.
Activity0 returns cause65535, retained CID/activity, null text, empty vectors and
zero MTUs. For the measured activity1/2 subset, protocol maps to0/1/2/4, interface
text is present and MTUv4 preserves its observed32-bit pattern. The opaque i64
is0 and MTUv6 is0 in this subset, without retry or MTU configuration policy.
PDP-error, mismatched-context and null-update original controls remain outside
this typed parser-success API.

DNS and P-CSCF use their independent server selectors, filter each all-zero
entry and retain ordering and duplicates, IPv6 before IPv4. Local zero addresses
remain in text. Gateway strings are report defaults, included for nonzero local
addresses only; they are not discovered routes. Non-IP with the tested selectors0
has `Some("")` address text, empty vectors and retained reported MTUv4.
The explicit `rmnet{index}` name is presentation only. The identical dual packets
at indices0/17 produce different names without binding a modem channel.

## Additional original address controls

A second original run reuses the exact qualified helper and controller for a
load check and six predeclared complete input/getter mutations. Independent
qualification reconstructs each144-byte input and both193-byte getters, checks
all114 emitted report fields and qualifies seven actual child waits0/stdout EOF,
original cutoffs, complete maps/process census and current39-file runtime pins.
Root freshly rehashes and checks the emitted fields, reusing that exhaustive
independent lifecycle/mapping audit. The outer CLI and five Docker helper
processes additionally have actual waits and both-pipe EOF receipts. QEMU strace
is a closed regular file; it is not mislabeled stderr-pipe EOF.

| Original control | Report observation |
| --- | --- |
| Zero IPv4 local | `0.0.0.0` text, no gateway; nonzero DNS/P-CSCF survive |
| Zero IPv6 local | `::/64` text, no gateway; nonzero DNS/P-CSCF survive |
| Zero first servers, dual | Each zero DNS/P-CSCF entry omitted; remaining IPv6 precedes IPv4 |
| IPv4-compatible IPv6 | `::192.0.2.7/64`, with dotted compatible DNS/P-CSCF tails |
| IPv4-mapped IPv6 | `::ffff:192.0.2.7/64`, with dotted mapped server tails |
| Equal IPv6 zero runs | `2001::1:0:0:1:1/64`, compressing the first equal run |

The native formatter preserves the captured compatible spelling when the first
96bits are zero and the high16 bits of the IPv4 tail are nonzero. Other values
use Rust's IPv6 display policy. The finite comparisons do not establish arbitrary
Bionic `inet_ntop` equivalence. [Rust's documented presentation policy](https://doc.rust-lang.org/stable/std/net/struct.Ipv6Addr.html)
and [historical public Bionic source](https://android.googlesource.com/platform/bionic/+/ics-mr0/libc/inet/inet_ntop.c)
are references; neither proves the matched binary's behavior. Actual selected
original outputs supply these test oracles.

## Native RED and GREEN

All fourteen complete input/report fixtures and tests are fixed before RED.
Eight comparisons come from the first original run and six from the additional
address run. After compilation, formatting and strict Clippy, all23 prior tests
and three new fixture/policy controls pass; fourteen comparisons fail with
`Unsupported`, actual host exit101. ARM64 compilation in this baseline is not
an ARM behavioral RED. Only `src/modern_report.rs` changes for repair. Every
source prerequisite, test, fixture, manifest and lockfile remains fixed.

All40 host and40 ARM64 user-QEMU tests then pass, including fourteen complete
owned-value comparisons per target. Root qualifies both native closures; a
separate source review rechecks all fixture fields and the sole changed source.
Original lifecycle qualification is separate from native codec testing. Local
preparation failures remain private setup records, without RED/GREEN claims.
[Provenance](evidence/native-sit-report-projection-provenance.json) pins the exact
original/native receipts, helper, fixtures, source and ARM executables.

## Original reference

A separately qualified original run under Bionic ARM64 user-mode QEMU covers
one load control and eleven complete parser → fresh context memory-update →
modern report cases. It uses authored synthetic 144-byte records, unpatched
original constructors, version 22, status override 0 and the original IPv6-first
order. The [authored comparison helper](../tools/native-sit-codec/original-report-probe.c)
is pinned to the executed build. [Evidence](evidence/native-sit-report-projection-provenance.json)
records the original run, independent review and separate load prerequisite.

Nine first updates copy the complete captured 352-byte DataCall prefix; eight
return the address-change bit 1. Non-IP copies with bit 0, while mismatched-CID
and null-input controls leave the prefix unchanged. All eleven identical second
updates return 0 with unchanged-prefix and source `memcmp` checks. Context state
0 and null APN/TD/NetIf-related pointers are captured before report construction.
The helper checks the same pointers after construction; post-builder context
state is not recaptured.

Independent post-run review compares all nineteen emitted fields in each report,
including scalar values, interface/address text and DNS/gateway/PCSCF vectors.
This covers the emitted projection; complete 264-byte field equivalence is not
claimed. Structured-address, QoS and traffic-descriptor vectors are empty in
these cases, so nonempty element decoding remains open. IPv4 address text has
no suffix; the tested IPv6 addresses use `/64`, with IPv6 preceding IPv4 in dual
reports. The long-string case has 38-character DNS/PCSCF values, forcing the
helper's heap-string branch; short-item representation tags are not recorded.

The cross-selector case retains IPv6 DNS and PCSCF in an IPv4 report even though
the original IPv6 getter returns zeros. Family getter projection therefore does
not describe the full raw DataCall. Inactive and PDP-failure reports return
cause 65535 with retained CID/activity and null text fields. Mismatch/null-update
reports return cause 65535 with zero CID/activity and opaque i64 −1. Non-IP has
empty address text and vectors.

The original report's checked getter returns a borrowed interior 264-byte view.
Cleanup uses its actual vtable deleting D0, followed by the builder's bound
non-deleting D1 and guarded context/adapter/packet cleanup. The declared report
allocation size is not independently measured. All twelve owned children have
actual wait 0 and stdout EOF within their original MONOTONIC cutoffs; held
namespace censuses contain only controller and owned QEMU, with no adopted or
residual process after each wait. Current selected originals and guest mappings
match their qualified hashes. Host-image file hashes were captured live; no
current closed-image rootfs audit is claimed.

The prerequisite qualifies natural original initialization/load/hold and a
successful `dlclose` call. It does not establish all finalizers or unmapping.
The controlled container uses `--network none`, passes no host devices and hides
host sysfs; observed failed UNIX/property/sysfs attempts remain explicit.
These original observations provide no normal handler/context-selection,
activation, APN/transport authority, live channel/provider, AIDL ABI or physical
phone qualification. Native formatter and TDD receipts have separate provenance.

## Next production gates

One owned request/response attempt must compose the setup encoder, source
incarnation, pending token, absolute suspend-aware deadline, decoder and report
through a transport whose framing/read-write/lifetime contract is established.
It must fence wrong/late/duplicate observations and retain uncertain outcomes
without implicit retry. The independent scripted peer will qualify composition,
not real device compatibility. Normal caller/context selection, channel/slot/
subscription/provider identity, APN lifecycle, reset/deactivation and matched
kernel/modem/carrier/Pixel behavior remain open. No physical phone was accessed.
