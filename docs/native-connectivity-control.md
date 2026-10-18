# Native connectivity control

The next connectivity tracer is a Rust-owned fixed IPv4 connection over one
exclusively administered virtual link. Settings Connect should establish real
local packet traffic, closing Settings should preserve that connection, and an
explicit authorized Disconnect from a fresh Settings instance should stop it.
This is a development backend, not Wi-Fi, Internet, Bluetooth or phone support.
The existing NetworkManager adapter remains separate read-only reference code;
NetworkManager is not selected as the final production connectivity owner.

## Current packet checkpoint

The [subsequent packet slice](native-connectivity-packets.md) now implements Rust
fixed IPv4 add/remove and passes two real nonce roundtrips, fixture-principal
withdrawal, explicit Disconnect and independent readback/quiet. The initial
foundation below retains its own source/evidence cohort. The later
[persistent worker](native-connectivity-worker.md) adds isolated host supervision,
idle actual readback, stopped-worker expiry/resume and queued-command Stop.
The separate [root-private Main/client](native-connectivity-private-ingress.md)
now passes the same packet lifecycle through actual credentialed Unix sockets,
with a separate real accept-error stopping regression. Real permissions/Settings,
installed deployment, ARM and phone qualification remain open.

## Earlier foundation checkpoint

Eight compiled Rust owner groups and one actual virtual-pair creation test now
pass on Harbour's native x86_64 Linux6.8.0-136-generic kernel. The same compiled
consumer first failed against the refusing implementations: one negative owner group
passed, seven positive groups failed, and the creation assertion failed with
`Unsupported`. Only production implementations changed; a separately checked
rustfmt bridge preserves the tests' tokens and string literals apart from
whitespace and commas. Locked offline format and strict all-target Clippy checks
also pass. These are Debug test ELFs, not a deployed service or Release runtime.

Rust subscribes before creating the pair, retains its witnessed local lifetime,
sets the local link administratively UP and verifies that no IPv4 address was
installed. At this checkpoint, `set_connected` explicitly returned `Unsupported`. The owner
accepts trusted in-process authorization decisions; no application socket or
OS permission boundary is qualified by its deterministic tests.

A separate substrate prerequisite uses the original `ip` tool and two processes
with zero capabilities to exchange a UDP nonce across sockets in distinct
retained namespaces. Its privileged provisioner is actually reaped before the
packet peers start. This establishes fixture viability, not Rust packet control.
No Settings window or screenshot was produced.

[Hash-only provenance](evidence/native-connectivity-control-provenance.json)
records both evidence classes and the preserved RED/format/GREEN epochs. The
current GREEN has20 disjoint actual direct waits with20 stdout PIPE EOFs; three
regular Cargo/format stderr closures remain separate. The raw prerequisite has
13 outer, three supervisor and ten provisioner direct waits, counted separately.
All selected epochs close their owned containers on the same daemon, preserve
inputs and finish with no adopted waits. This claims no transitive-child or
vendor cleanup. Independent review confirms the current source/lock/test bindings
and closure records.

## Ownership and request lifetime

A command carries a trusted service-issued application identity, permission epoch
and original absolute `CLOCK_BOOTTIME` deadline. App requests contain no device
paths, interface indices, arbitrary addresses or backend configuration. Only one
fixed-profile mutation may be outstanding. Before dispatch, app departure,
revocation or expiry cancels definitely unsent work.

Dispatch transfers completion responsibility to the background connection owner;
it is not evidence that a syscall was sent or succeeded. After dispatch, app
closure fences that requester's reply while the service continues observing the
operation. A successful connection survives Settings closing. Revocation removes
future control authority, without implicitly disconnecting an already approved
system connection. Results require both concrete backend completion and fresh
requester authority to produce a reply. Global readback may still publish after
a requester has departed or expired.

Definitely unsent failure leaves the latest actual observation unchanged. A sent
uncertain result fences further mutations in the current backend epoch. Readback
may report current configuration, but cannot retroactively acknowledge an old
request or authorize retries, rollback or compensating Disconnect. This first
slice promises no connection persistence across service death or namespace
retirement.

## Exclusive virtual backend

The trusted supervisor retains two fresh private network namespaces throughout
owner shutdown. An initial loopback-only inventory is a sanity check, not proof
of exclusive administration. Rust subscribes to kernel link/address notifications
before creating its virtual pair; both endpoint namespace identities remain
retained. A separate mutation socket uses fixed payload limits and nonblocking socket flags.
Those flags do not bound synchronous kernel dispatch: [netlink dispatch](https://raw.githubusercontent.com/torvalds/linux/v6.6/net/netlink/af_netlink.c)
can enter [RTNL handling](https://raw.githubusercontent.com/torvalds/linux/v6.6/net/core/rtnetlink.c)
and wait on a kernel lock. BOOTTIME checks fence accepted completion; they do not
prove syscall duration or responsive Stop. A production service must isolate this
component under owned worker/supervisor custody, with Main snapshots and Stop
remaining responsive. The [later worker checkpoint](native-connectivity-worker.md)
implements that separation and passes stopped-worker controls; arbitrary
uninterruptible kernel reclamation remains unqualified.

Peer provisioning is serialized with command admission closed. Its short-lived
privileged provisioner must close control sockets and actually exit and be reaped
before an independent packet peer starts without capabilities or administrative
descriptors. During operation, only the Rust owner administers link/address
lifecycle. No link deletion, move, rename, replacement or namespace destruction
may race a pending mutation. Outer cleanup happens after the owner stops and is
actually reaped.

Unexpected disappearance, observer failure or conflicting configuration retires
the backend epoch. Do not compensate against an old interface index or adopt a
replacement by name. Read-only lifetime observations detect invalidation; they
do not make a later rtnetlink write atomic against interface-index reuse. Linux
IPv4 [address mutation](https://raw.githubusercontent.com/torvalds/linux/v6.6/net/ipv4/devinet.c)
resolves the supplied index under RTNL; the [address UAPI](https://raw.githubusercontent.com/torvalds/linux/master/include/uapi/linux/if_addr.h)
has no expected userspace lifetime token. Exclusive lifecycle administration is
the targeting argument for this backend. Physical and hotplug backends need
their own justified serialization contract.

## Evidence and remaining integration

The component checkpoint above does not complete connectivity control. Setup,
namespace capability checks and compilation do not count as behavioral RED or
packet success. Keep owner policy tests, actual isolated kernel/packet tests, installed
service tests, Settings UI integration and hardware evidence distinct. The
end-to-end Settings tracer, deployment, restart/recovery, suspend, Wi-Fi
association, IP acquisition, DNS, Internet reachability, Bluetooth, tethering,
USB and all physical-device gates remain open.
