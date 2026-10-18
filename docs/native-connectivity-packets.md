# Rust fixed IPv4 packet lifecycle

Rust now configures and removes a fixed local IPv4 profile on its owned virtual
link, with actual packets crossing into a second retained namespace. Two random
UDP nonce/ACK roundtrips pass: one after Connect, one after withdrawal of the
literal fixture app principal. A fresh fixture principal explicitly Disconnects;
independent address readback becomes empty, a fresh socket returns
`ENETUNREACH`, and the packet peer observes zero datagrams through its final real
`EAGAIN`.

This is a native x86_64 host component/packet test on Linux6.8.0-136-generic,
using Cargo Debug test ELFs. The authorization values are trusted fixture inputs.
No Settings window, OS permission boundary, installed service, ARM kernel,
matched Pixel kernel, Wi-Fi, Internet or phone behavior is qualified.

## What changed

`VirtualLink::set_connected` now checks the witnessed endpoint and its last
observed state, sends one fixed `RTM_NEWADDR` or `RTM_DELADDR`, correlates the kernel
ACK and requires actual address readback under the same original BOOTTIME
deadline. There are no application-supplied interface indices, addresses or
routes. An already matching explicit choice returns fresh readback without a
mutating syscall. No default route is installed.

A possibly dispatched failure makes this backend epoch terminal. No mutation
retry, compensating delete or automatic rollback follows. An original deadline
that expires before entering the mutating syscall remains definitely unsent.
Read-only collection may repeat on concurrent kernel notifications. Failed or
conflicting pre-send observation also conservatively retires this backend epoch.

`connected` describes the exact local IPv4 profile. Actual packet reachability
is measured separately. `fact()` is cached; there is no idle background lifetime
invalidation or production freshness service yet. The
[exclusive virtual-lifecycle contract](native-connectivity-control.md#exclusive-virtual-backend)
remains necessary: lifetime checks do not make a write atomic against index reuse.
Nonblocking netlink flags do not bound synchronous kernel handling, so this
component must be isolated under owned worker supervision before claiming a
responsive service Main or Stop.

## Actual RED and unchanged positive consumer

The compiled desired-behavior test first fails with
`CONNECT_REQUIRES_ACTUAL_FIXED_IPV4_SUCCESS: Unsupported`, after real pair
creation, successful peer address/UP provisioning, actual provisioner wait/EOF,
capability-free packet-peer readiness and supervisor capability removal. The
Rust test exits101 with one failed test; no UDP roundtrip is reached. This is a
behavioral RED, not a missing executable or setup failure.

The original outer aggregate remains false. Its report checker selected the
packet peer by receipt ordinal when validating Rust's executable argument list.
Separate Root and independent reviews bind the exact Rust role correctly and
qualify actual component RED plus closure; they do not relabel that failed
aggregate. The successor outer checker matches the exact role. No runtime peer,
controller, oracle, action deadline or functional cutoff changes with that fix.

After the Rust IPv4 repair, the same consumer passes. A checked rustfmt bridge
preserves predicates and string literals: whitespace/commas change, and three
single-expression match arms gain only enclosing braces. The original controller
and independent peer remain byte-exact. Locked offline format and strict
all-target Clippy checks pass, followed by all eight existing owner groups with
zero capabilities. The build scope fully closes before the same PID execs the
packet controller; no privileged builder remains beside the owner.

## Measured result

The Rust process retains only `NET_ADMIN` in its effective, permitted and
bounding sets. The packet peer and supervisor have zero capabilities. The peer's
actual descriptor inventory contains only stdio and its UDP socket. Both
namespace handles outlive all child waits and output closure.

The same retained namespace/index/witnessed lifetime persists through
false→true→false profile state. Independent capability-free Python GETADDR
readback sees192.0.2.1/30, then no local IPv4 address; both checks find no default
route. The peer's two distinct37-byte received payloads exactly match Rust's
nonces and their41-byte ACKs. The second exchange follows fixture-principal
withdrawal with no extra Connect. Disconnect uses a different literal principal;
this does not establish an actual app-close or permission-revocation boundary.

The quiet phase has its original800ms cutoff, including gate delivery. In this
run, actual peer arming through the final `EAGAIN` spans801.237361ms with zero
received datagrams. The fresh loss probe returns errno101. The complete
functional case finishes in842.419364ms before its original six-second cutoff;
this is one recorded host case, not a latency guarantee. Connect and Disconnect
retain their original at-most-one-second action deadlines.

The current GREEN closes21 disjoint actual direct waits with21 stdout PIPE EOFs:
13 outer, four build, three controller and one provisioner. Three regular
Cargo/format stderr descriptor closures remain separate. Original cutoffs,
retained namespaces, ECHILD, exact container removal, daemon identity and all
selected input hashes pass; no adopted waits occur. No transitive-child or vendor
cleanup is inferred. Independent review confirms source, artifact, packet,
readback, clock and closure bindings.

[Public fixture source](../tools/native-connectivity-control/) preserves the
measured controller/peer bytes. [Hash-only provenance](evidence/native-connectivity-packets-provenance.json)
records the separate failed aggregate, qualified component RED, formatter bridge
and current GREEN. Raw outputs, nonces and compiled test binaries remain private.

## Remaining work

Implement the owned service worker and qualify responsive Main snapshots/Stop,
blocked-dispatch recovery, continuous readback/cache freshness and actual backend
deadline/uncertainty/conflict/loss controls. Then connect real application authentication and permissions, the
Settings Rust client/UI, installed deployment, restart/recovery and explicit
connection-persistence policy. ARM, matched Pixel kernel/backends, suspend/lock,
Wi-Fi/IP acquisition/DNS/Internet, Bluetooth, tethering, USB and every physical
phone gate remain open. Zero phone models are qualified.

The next service design uses one persistent `native-process-owner::Supervisor`
worker per epoch, preserving the original observer between commands. Trusted
startup launches that fixed worker, then Main drops all capabilities before
admitting clients; no privileged broker remains. Worker loss makes the epoch
unavailable/uncertain. Recovery requires an explicit trusted service restart into
fresh namespaces after prior custody closes; no same-namespace worker replacement
or connection-persistence promise is selected. Main must expire actual observation
leases independently and must not turn a heartbeat into fresh kernel evidence.
This architecture decision is not implemented service qualification.
