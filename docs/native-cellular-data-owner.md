# Native cellular owner observations

One fixed actual ARM Binder/kernel test first reached a qualified missing
consume/join RED, then passed on the first repaired run. Root's final qualifier
closes both cases and all 42 direct child waits/EOF. Three later actual ARM
runs qualify ordinary public-poll setup and independent link/list/death loss
against unchanged production code, adding 71 separately counted waits/EOF.
Root and independent review pass each checkpoint, with the coverage limits below.

`native-cellular-owner` keeps one retained Data endpoint, fresh callback pair and
one retained `KernelObserver` together. It separates an in-memory provider
submission disposition from an owner-local kernel association. Neither state
is an APN-ready, route, carrier, internet or installed permission grant.

The [bindings/NDK ABI prerequisite](native-cellular-data-abi.md) and
[kernel observer](native-network-observer.md) remain separately qualified.
The [package README](../native-cellular-owner/README.md) describes compilation;
[provenance](evidence/native-cellular-data-owner-provenance.json) records source
identities and summarized evidence. Raw audits and custody helpers stay private.

## Qualified RED before repair

One fixed ignored-by-default test, `kernel_tests::actual_binder_kernel_join`,
ran against a real generated NDK DataV3 peer on the generic AArch64
`7.1.6-1-aarch64-ARCH` VM kernel. It first establishes one callback attachment and
one setup 713 with all 10 arguments, including the 21-field profile. Ordinary IPC
return leaves disposition `OutcomeUnknown`.

An independent controller then creates nonpersistent TUN `pklabi0` after the
owner's observer is subscribed. Independent kernel dumps and that same owner's
retained observer establish the UP point-to-point link, MTU 1400, witnessed
lifetime, IPv4 local 192.0.2.2 distinct from peer 192.0.2.1/32, and IPv6
2001:db8:1::2/64 with explicit `nodad`. The owner receives actual raw wrong 714
and matching 713 callbacks, with full copied 18-field results and calling UID 1000,
after a finite 100 ms no-callback interval.

Only then does the baseline's production reducer return `Unsupported`, leaving
`OutcomeUnknown`, no provider report and no association. The test exits 101;
this is the qualified missing native consume/join capability RED. It is not a
baseline with already-settled provider history. All 21 baseline direct child
waits/EOF and original cutoff, module/namespace, runtime/input and owned-container
closure checks qualify. The raw callbacks and complete kernel candidate are
prerequisites already established before the missing behavior is reached.

The same fixed test and five custody-envelope files are frozen before repair;
only `native-cellular-owner/src/lib.rs` is permitted to differ. Compilation and
authored fixture import/hash preparation failures are preserved prerequisites,
not behavioral REDs.

## Repaired source contract

The production reducer accepts only the expected callback UID, ordinary response
type and bounded copied shape. A wrong serial cannot settle history. The first
matching response for the live, unretired `OutcomeUnknown` attempt retains its
provider report. It records `ProviderAccepted` only when both
`RadioResponseInfo.error` and `SetupDataCallResult.cause` are `NONE`; otherwise it
records `ProviderRejected`. Later or retired responses cannot rewrite that
instance's history. Accepted history does not establish an actual APN. The state
is local memory; no durable journal or recovery protocol is implemented here.

`poll()` asks the same retained observer for a fresh complete snapshot within
the original absolute `CLOCK_BOOTTIME` deadline. The first join requires a unique
matching interface name, witnessed positive lifetime, UP flag, nonzero MTU and
matching reported local numeric addresses/prefixes. The selected IP protocol
must agree with the nonempty address-family set; each reported family's MTU must
match, kernel address scope must be 0, and tentative/DAD-failed/deprecated flags
are excluded. Exactly one eligible local/prefix match is required per reported
address. This counts eligible records, not all raw kernel records with that
local address. Only zero provider address flags and explicit infinite Android
lifetime sentinels are accepted; finite Android lifetimes are not converted to
native boot-time values.

Provider DNS, gateways and P-CSCF remain reports. Extra kernel addresses cannot
supply a missing reported address, and a point-to-point peer is not confused
with a local address. A changed/missing joined namespace/index/lifetime retires
the current association without rewriting accepted provider history. Every
valid list event conservatively retires; a list cannot establish a new join.
Death marks the provider unavailable and retires the association. These policies
and refusal branches have source review; their negative controls remain open.

This is not an atomic cross-source snapshot. Matching authored provider
name/address fields to an independently created TUN does not prove that the
provider owns or created that link. Origin is the same owner and its continuously
retained observer, not a modem/provider-to-kernel authority witness.

## Initial RED/GREEN coverage boundary

The fixed test takes actual owned events from the private callback inbox and
calls the production `reduce` method used by public `poll`. It is not a
constructed-event replay. It nevertheless bypasses `poll`'s queued Setup/List/
Died dispatch. That ordinary dispatch path is not executed by this initial
test; the later three-run qualification below covers it separately.

The repaired execution checks wrong 714 ignored without history, matching 713
accepted as `ProviderAccepted { cid: 9 }`, then an ordinary `poll` obtains a
fresh same-observer kernel join.
The controller closes its sole TUN FD and independently observes absence; a
subsequent ordinary `poll` withdraws the association while retaining accepted
history. Actual empty-list and death events are then reduced directly. They
arrive after link-loss withdrawal, so they do not independently test list/death
withdrawal from a positive association. Accepted history and the complete provider
report remain unchanged throughout withdrawal and the later directly reduced
list/death events.

The actual association is observed at 26.176732133 s boot time, before sole-FD
close 26.243556387 s, independent absence 26.297350660 s, endpoint death 26.359231559 s
and final outcome 26.364338629 s. These timestamps belong to that single repaired
VM boot; they do not connect identities across separate boots.

| Case | Native test exit | Actual direct child waits/EOF |
| --- | --- | --- |
| Qualified baseline | 101 | 21: 18 exit 0; test 101, peer −9 and manager −9 |
| First repaired run | 0 | 21: 20 exit 0; owned manager cleanup −9 |

Actual `/proc` observations bind the native test, peer and manager to
UID/GID 1000, empty supplementary groups and zero effective capabilities. Actual
GNU/native and genuine Bionic mappings, plus the separate Binder/property
inventories, are pinned. No NDK compile stubs are runtime substitutes. The same
fixture generation, four fixture ELFs, runtime selection and five envelope files
are used in both cases. The explicit generation string is reused across two fresh
VMs with distinct network namespaces; it is not unique across runs. Only the
owner library source changes among 60 selected source paths. The test and other
six owner files are unchanged.

Both cases close controller TUN/namespace FDs, restore the original namespace,
unload the module after link/process closure, remove Binder/property resources,
reap every direct child with EOF before the original cutoffs, and power off.
The owned outer CLI is waited and its exact container freshly absent on the same
daemon. Root independently rehashes every selected and original input, including
the backing image, kernel and initramfs. Local owner drop occurs before its
receipt/process wait; individual native FDs have no independent `EBADF` check.
Missing Bionic linker-config warnings remain preserved; Android linker namespace/
configuration and SELinux are not qualified by this fixture.

## Additional ordinary public-poll qualification

Three separately owned actual ARM runs pass against the unchanged repaired
production owner, callbacks, bounded copies, bindings and kernel observer.
Only `src/kernel_tests.rs` is appended; its earlier 16,983-byte test remains an
exact prefix with its original digest. The new tests are ignored by default.
This adds execution coverage; no new behavioral RED or production repair is
claimed. The [earlier provenance](evidence/native-cellular-data-owner-provenance.json)
remains unchanged, while [public-poll provenance](evidence/native-cellular-data-public-poll-provenance.json)
records the new test source and three actual runs.

Every new case uses public owner methods, without taking events from the private
inbox or calling `reduce` directly. One actual NDK attachment and setup preserve
all 10 arguments, 21 profile fields and the full 18-field provider result.
Ordinary `poll()` consumes the normal matching `SOLICITED` response for serial
713, records `ProviderAccepted { cid: 9 }`, and has no association before the
controller creates any TUN. The independent controller then creates the sole
nonpersistent CLOEXEC TUN, with IPv4 local distinct from peer, IPv6 `nodad`,
UP/point-to-point flags and MTU 1400. Public `poll()` obtains a fresh association
from that same owner's retained observer.

| Actual case | Withdrawal control | Direct child waits/EOF |
| --- | --- | --- |
| Setup and link loss | Controller closes the sole TUN FD and independently observes absence before public-poll withdrawal | 21 |
| Empty list | Public-poll empty-list dispatch withdraws a fresh positive association while the TUN FD remains held and the peer remains live | 25 |
| Endpoint death | Actual peer exit 0/EOF and public-poll death dispatch withdraw a fresh positive association while the TUN FD remains held | 25 |

For both provider-loss cases, independent full kernel snapshots before and after
withdrawal confirm the same namespace-local ifindex, UP/point-to-point flags,
MTU, local/peer IPv4 and IPv6 addresses. The after-snapshot follows native
withdrawal; in the death case it also follows actual peer wait 0 and EOF. Both
native clients then exit 0 with EOF before the controller closes the sole TUN FD.
Thus their withdrawal is independent of kernel link loss. Accepted history and
the exact complete provider report remain unchanged. Subsequent actual TUN
closure and independent absence qualify cleanup separately.

The 71 additional direct child waits/EOF comprise 21/25/25, with every native
test and peer exiting 0 and each owned manager killed for cleanup and waited as
−9. Every wait/EOF is observed before its original absolute BOOTTIME cutoff.
Actual UID/GID 1000, empty groups, zero effective capabilities, four GNU/native
ELF maps, 15 genuine Bionic peer/loader/library ELF maps and three separate
property maps are bound in each case. The Binder device remains a separate
mapping; compile stubs are not runtime substitutes. Namespace restoration,
controller FD closure, module unload, Binder/property teardown, poweroff and
outer CLI 0/exact-CID fresh absence on the same daemon qualify. Root rehashes all
selected and original inputs, including the backing image; independent review
rehashes all selected and remaining small original bindings. Missing Bionic
linker-config warnings remain preserved.

The list and death VM boots happen to use identical numeric namespace device/
inode values. They are local identities inside independently owned fresh VMs,
not proof of identity across boots. New fixture generations differ by run;
namespace numbers, ifindices and local lifetime counters are not durable tokens.

Build prerequisites pass formatting, Clippy with warnings denied, host/ARM64
libtest compilation and ordinary ARM64 release compilation. No host behavioral
test is run. A skipped ignored test or a successful compile is not an actual
behavioral pass. Unknown-serial processing through ordinary public-poll dispatch
remains unqualified; the earlier direct-reducer wrong-serial check remains
separate. Worker/pool joins, responsive installed Stop, production permissions,
APN acquisition and provider-to-kernel authority remain open.

## Remaining boundaries

The compile prerequisite passes formatting, Clippy with warnings denied, host
and ARM64 libtest compilation, and ordinary ARM64 release compilation. It runs
no host behavioral tests. The ordinary release excludes test-only fixture code.
A skipped ignored test or a successful compile is not a behavioral pass.

Runtime negative controls for UID/shape/errors, duplicates, broader stale source/serial,
retirement races, IP/MTU/prefix/flags/lifetime policy, rename/reuse, overflow,
concurrency, deadline/suspend and load remain open. Raw duplicate local addresses
with different peers or ineligible scope/flags are outside the qualified subset;
no general raw-address uniqueness claim is made. Unknown-serial handling through
ordinary queued-event dispatch remains open; the earlier wrong-serial result
was fed directly to the production reducer.

Process-global Binder pools are fixture infrastructure. In these owner-library
runs, local owner/watch drop and process waits do not qualify pool/worker joins
or a supervised service Stop. No run proves remote deactivation or vendor cleanup. A cached `last_association` is not current
permission. Expected UID and endpoint discovery/version/hash are external
premises. These owner-library runs do not qualify production permission policy,
installed lifecycle or journal/recovery. The separately pinned supervised service
below adds its own conservative journal and local process-Stop qualification,
without installing a normal unit or granting current authority.

Actual modem endpoint/slot/subscription/registration mapping, provider ownership
of the kernel link, carrier/SIM/APN policy and acquisition, ACK_EXP/wake handling,
routes and traffic, matched Google/Pixel kernel/VINTF/SELinux and authorized
hardware qualification remain open. No APN-ready context is injected. The
controller's netns/TUN privileges and guest Binder/property setup are disposable
fixture powers, not production authority. No vendor endpoint, normal installed
app, authentication or physical phone is exercised.

## Separate supervised Data integration

The [production Data service tracer](native-cellular-service.md) adds one selected generic ARM
Main/worker/operator execution around this unchanged owner. It qualifies its own
conservative admission journal, exact full report through worker/status, real
controlled link-loss withdrawal, native child actual wait, Main exit0 and
same-journal refusal78, with34 separately counted direct waits/EOF. It is
additional integration after separate component REDs. The earlier owner/ABI
proofs above retain their old sources and scopes. A process wait is not a Binder
pool/thread join, modem deactivation, vendor cleanup or successor authorization;
no APN/provider-created link/current app grant or normal installed unit is proved.
Broader queued/structural negatives and hardware gates remain as recorded above.
