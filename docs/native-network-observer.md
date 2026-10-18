# Native kernel network observer

`native-network-state` now observes links and addresses through a Rust
`NETLINK_ROUTE` socket. One fixed test first failed with `Unsupported`, after
independent healthy TUN controls, then passed after the observer was implemented.
Both executions ran under an actual generic AArch64 Linux7.1.6 kernel in a VM.
This qualifies a read-only kernel observation boundary. It grants no Data/Binder,
APN, route, carrier, internet-reachability or installed-provider authority.

The [package README](../native-network-state/README.md) describes the API and
compile checks. The [public provenance](evidence/native-network-observer-provenance.json)
records source hashes and summarized evidence; private audit files and guest logs
are retained separately.

## Owned status-service consumer

The new [Rust status service](../native-network-status/README.md) reuses this
observer unchanged. Its actual normal host service publishes healthy-empty0/0
from an independently checked isolated namespace, while three controlled-worker
cases qualify publication only after wait/EOF, held-reader Stop and late-success
refusal. Fresh observer instances do not establish continuous interface lifetimes
or persistent provider associations. Host/ARM release compilation is separate
from this document's earlier actual ARM kernel/TUN execution; the new service
has no installed, TUN, UI, Wi-Fi, Internet or phone qualification.
See the [service proof](evidence/native-network-status-provenance.json) and
[default-test/permanent-driver proof](../native-network-status/validation/test-entry.json).

## What the producer owns

`KernelObserver::subscribe_current_namespace(until_boottime_ns)` retains the
constructing thread's network namespace and creates its own nonblocking,
close-on-exec socket together. It checks the namespace identity around setup and
subscribes before initial synchronization. Callers cannot supply an unrelated
socket/namespace pair. The observer acquires no TUN descriptor, mutates no link
and starts no worker.

Each successful `refresh` includes namespace device/inode identity, a notification
revision, distinct completed `RTM_GETLINK` and `RTM_GETADDR` dump sequences, and a
`CLOCK_BOOTTIME` timestamp before the caller's absolute deadline. The producer
requires matching multipart dump completion and drains notifications to an actual
`EAGAIN`. Interleaved state notifications invalidate the candidate. The two dumps
are separate observations; a returned snapshot cannot prevent subsequent change.

A lifetime is a local epoch assigned only after a continuously witnessed creation.
Links already present at initial synchronization have no witnessed lifetime.
Neither a name nor an ifindex alone identifies a persistent device or provider.
Address records preserve the local/peer distinction, prefix, scope and flags;
link records retain flags, MTU and operational state.

The source bounds a refresh to64 links,16 addresses per link,64KiB per datagram,
256 messages,256 datagrams and1MiB received bytes. `ConcurrentChange` is the only
retryable error variant; it does not publish a candidate. Every other error,
including `Deadline`, poisons that observer. Deadline checks fence publication;
they do not prove a hard return bound during scheduler stalls or suspend. These
error, bound and concurrency paths have source review, not executed negative
control qualification in this milestone.

The newer [connectivity-control review](native-connectivity-control.md#exclusive-virtual-backend)
also makes synchronous kernel dispatch explicit: `DONTWAIT` does not bound time
inside rtnetlink handling or its locks. Keep this observer under owned worker
custody before claiming responsive service Main snapshots or Stop. This is a
lifecycle constraint, not a new observer runtime result.

## Actual RED, then GREEN

The ignored-by-default `kernel_tests::actual_kernel_link_lifetime` test was
explicitly selected once per qualifying VM. The test source, Cargo manifest/lock
and five custody-envelope files were unchanged. Only `src/lib.rs` changed between
the qualifying baseline and final repair.

| Evidence | Actual result | Direct child waits with EOF |
| --- | --- | --- |
| Observer baseline | Test exit101: explicit `Unsupported` after healthy independent controls |16; the test exits101, the other15 exit0 |
| Observer final repair | Test exit0: observation and withdrawal pass |16, all exit0 |
| Separate TUN prerequisite | Sole-FD lifetime and independent kernel controls pass |15, all exit0; separate from the32 above |

The repaired Rust process ran as UID/GID1000, with no supplementary groups and
zero effective capabilities. Actual `/proc` executable, credentials, namespace
and four mapped ELF files were checked, including loader/libc/libgcc closure;
their bytes remained unchanged after the process wait. This is ARM kernel
execution, separate from ARM user-mode emulation or a host behavioral test.

A separate controller loaded the original `tun.ko`, entered an isolated retained
network namespace and held the sole nonpersistent TUN FD with `CLOEXEC`. The Rust
test inherited no TUN FD. Independent `ip` controls established an UP point-to-point
link with MTU1400, IPv4 local192.0.2.2 distinct from peer192.0.2.1/32, and
IPv6 2001:db8:1::2/64 with explicit `nodad`. DAD was not exercised.

The native snapshot identified `pklcap0`, ifindex2, witnessed lifetime1 and
revision5, with completed dump sequences3/4. After the controller actually closed
its sole TUN FD, independent kernel dumps showed the interface and documentation
addresses absent. The same observer then withdrew that lifetime at revision9
with completed dump sequences5/6, within the original boot-time budget.

Both observer cases restore the original namespace, close controller-owned TUN
and namespace FDs, unload the module, reap every direct child with EOF, power off,
and close the exact owned container on the same daemon. Selected inputs remain
unchanged. The Rust observer leaves scope before its final receipt and its
process is waited; its individual FDs were not independently tested for `EBADF`.
The standalone TUN prerequisite has its own closure evidence and is not added to
the observer's32 waits.

## The multicast repair and retained failures

The first TUN attempt failed on the fixture parser's missing `peer` JSON key;
iproute2 reports this IPv4 pair as `local` plus `address`. It is a failed
prerequisite. An earlier observer repair reached healthy controls but then
returned `InvalidData`; it is a failed GREEN attempt, separate from the qualified
missing-capability RED. All attempts remain preserved.

The final source obtains kernel sender PID0 and multicast group identity from
`recvmsg` metadata. It routes subscribed multicast by that group and message
family, while matching unicast dump replies to their request sequence and port.
It does not classify notification origin from header sequence/port alone. The
successful run's bounded test-only metadata trace contains21 messages, including
one IPv4 multicast with group16, kernel sender PID0 and nonzero originating
request sequence/header port. No raw packet payload is published.

The [Linux Netlink introduction](https://docs.kernel.org/userspace-api/netlink/intro.html)
explains dump/notification framing. Subsystem details matter: pinned Linuxv6.12
[`devinet.c`, lines1814–1839](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/net/ipv4/devinet.c#L1814-L1839)
retains request sequence/port when forming IPv4 address multicast, and
[`af_netlink.c`, lines1831–1837](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/net/netlink/af_netlink.c#L1831-L1837)
places sender port/group information in the receive address. These are source
references. The actual executed kernel was `7.1.6-1-aarch64-ARCH`; no Linux6.12,
Google/Pixel kernel or physical-device execution is claimed by those links.

## Remaining qualification

Strict formatting, Clippy with warnings denied, host and ARM64 libtest compilation,
and ARM64 release compilation pass. The compile stages ran no host behavioral
tests. Ordinary Cargo test selection skips the actual-kernel test; passing a
compile or a skipped test cannot substitute for the two VM executions above.

Actual malformed/error injection, ifindex reuse, rename, concurrent churn,
overflow, suspend/deadline and load tests remain open. Production integration
also needs a matched Google kernel and policy, the actual modem Data endpoint,
version/hash and slot/subscription mapping, carrier/APN/SIM admission, explicit
provider-to-kernel association, and durable permissions, service/lifecycle,
responsive Stop and persistence policy. The read-only
[NetworkManager boundary](native-connectivity.md) is separate. No modem, vendor
backend, normal installed app, carrier traffic, authentication or physical phone
was exercised by this observer proof.
