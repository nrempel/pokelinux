# Native fixed IPv4 after open wireless association

The normal Rust ARM service now completes fixed-address Up, two independent
nonce ACKs, explicit Down and quiet after confirmed open wireless association.
A reusable existing-link attachment backs the explicit normal `profile-up` /
`profile-down` operations. Its selected fixture retains the radio lifecycle, confirms the existing normal Rust open association,
and keeps the AP in a separate namespace. The address API does not associate a
radio, create a link, infer ownership from an ifindex, or provide Internet access.

## Actual baseline RED

The frozen desired consumer requests:

```text
pokelinux-connectivity-service profile-up NS_DEV NS_INO IFINDEX IFNAME ORIGINAL_UNTIL_NS
```

The unchanged normal ARM service actually exits 1 with the exact 30-byte output
`Error: Kind(PermissionDenied)\n`. This is the observed refusal; it is not relabeled
as an unsupported or invalid-arguments response. Actual wait and stdout EOF close
about 41 ms after arming, under the original one-second cutoff. Independent kernel
samples show no station IPv4 address or selected subnet route. The separately
namespaced capability-free peer receives zero packets; the station remains
associated until one separately requested normal Rust wireless Disconnect.

There is exactly one profile-up, no profile-down, no positive UDP client and no
retry. The one desired address/packet assertion fails after qualified cleanup.
All 139 direct waits have actual stdout EOF: 15 host, 3 boot, 105 guest, 15 address-case
and 1 poweroff. Thirty-three successful adopted terminal waits precede actual
ECHILD; their origins and PIPE EOF are not inferred. QEMU exits 0; the expected RED
container and attached CLI exit 1, the exact container is removed on the same
Docker daemon, and selected inputs remain unchanged.

A preceding execution fails during fixture setup: capability-free guest UID 0
cannot traverse the selected UID 1000/mode 0700 results root to create peer gates.
It submits no profile operation, positive UDP request or Rust Disconnect and is
not behavior RED. The corrected fixture keeps the results root unchanged, puts
owned gates in a guest RAM subtree with UID/GID 0 and mode 0700, and checks actual
after-exec UID 0/cap0 create/fsync/close/unlink before AP address provisioner
and packet-peer setup. Gate artifacts
are copied to protected results only after all case handles retire. Earlier
unexecuted preparations, including a superseded admission, remain separate.

## Shared Rust API and ownership

`ExistingIpv4Profile::attach_existing(target, expected_configured, until)` retains
a real namespace descriptor, constructing Rust ThreadId and continuous kernel
observer. The selected namespace, interface name/index, UP state and optional
observed lifetime must match exactly. A pre-existing link has lifetime `None`;
that value is an exact baseline and grants no ownership. The trusted caller must
retain exclusive link/radio lifecycle custody, including continuity before the
observer subscribed. Association is independently established by the radio owner.

`set_configured` uses the existing netlink mutator to add or remove only
`192.0.2.1/30`, followed by exact independent observer readback. `refresh` is
read-only. Prior configured state is exact; attaching never adopts an earlier
uncertain operation. Changed namespace/thread/identity/lifecycle, observer loss
or conflicting readback retires the object. A possibly dispatched failure makes
all further methods terminal; there is no retry, replacement, rollback or
refreshed action cutoff. Dropping the object only closes descriptors.

The shared address codec/mutator preserves `VirtualLink`'s existing veth-only
creation and pair/witness checks. Existing-link observations remain separate from
its witnessed veth Fact. The normal CLI requires an explicit selected namespace,
ifindex/name and original cutoff, accepts already exact minimal NET_ADMIN rights,
and reports completion only after kernel readback and final deadline checks.
The long-lived owner is expected to retain the same attachment object; these
one-shot fixture operations do not implement persistent wireless ownership.

Eight focused new unit groups cover exact prior state/identity/lifetime,
malformed and expired requests, namespace/thread retention, terminal methods,
observer loss and the shared fixed-address codec. The closed optimized build
passes 18 active host tests (10 library and 8 owner), formatting, strict Clippy,
and normal host/ARM Release compilation with unchanged dependencies. Three
existing ignored kernel cases are not claimed as passes. The exported post-rustfmt
source map, separately preserved raw input map and adopted worktree bytes bind
the actual compiled source. Those component results do not establish kernel
packet behavior.

## Desired native packet lifecycle

The same pure consumer requires normal profile-up completion and actual worker
exit/stdout EOF before either packet client starts. Independent kernel samples
must show the selected station address and connected subnet route without a
default route. Two separately executed capability-free clients each send one
fresh nonce to the capability-free peer in the retained AP namespace and receive
its exact ACK. The peer independently records the same two nonces.

A later explicit normal profile-down must complete and exit, with independent
station address/subnet-route absence. A fresh loss client and peer's actual final
EAGAIN observation establish quiet under the original 800 ms window. Independent
station/AP observations must still confirm association before and after removal
and quiet. Only then may the fixture request the separate normal Rust wireless
Disconnect. A caller-retained Scope prevents L2 Disconnect and radio removal
while any case worker or peer lacks actual wait/stdout EOF. Success, timeout and
process exit do not authorize an uncertain effect retry.

The pure consumer is byte-identical across RED and GREEN selections. GREEN's
runtime helper includes a read-only publication wait because ACK2 precedes peer
JSON publication by construction. It waits for exactly the same two records under
the original six-second cutoff, sends no extra packet and renews no deadline.
This was a static pre-execution fixture correction, not an observed RED; the whole
runtime is not claimed byte-identical. The selected normal ARM service hashes
`64c28416ec9747b8648019dfd0066213c96a0c31bdcad7b38175e046fb2e6c28`.
The first native GREEN selection completes normal profile-up and profile-down
with exit 0, two capability-free nonce ACKs, independent address absence and
quiet, and L2 observations before/after removal. Its 24 address-case waits/EOFs
close, followed by the separate normal Rust Disconnect. The complete saved pure
consumer review reports passed, confirmed outcome and qualified cleanup. However,
the final guest result JSON is truncated at 262,144 bytes; boot JSON decoding
fails after QEMU exits 0, container/attach exit 2 and outer exit 1. That whole
cohort remains FAILED and is not relabeled GREEN.

The existing fixture requests asynchronous poweroff before writing the final
large report. That ordering defect permits suspected shutdown interruption;
the failed epoch did not sample its inherited RLIMIT_FSIZE, so a unique cause is
not established. The failed cohort remains intact. A later fixture-only repair
samples its actual inherited limit and effective PID1 unit before radio setup. It blocks SIGTERM
before the existing poweroff request, preserves the handler, restores the prior
mask in that child, and atomically publishes reports with file fsync, rename and
parent-directory fsync under the original cutoff. It then restores the exact
previous signal mask. Pending TERM may prevent natural guest exit 0; that exit
is not claimed. No timer, extra worker, limit override or effect retry is added.

## Actual native GREEN

The repaired selection passes the same frozen pure consumer with the same normal
752,432-byte ARM Release service. Profile-up completes and its actual wait/EOF
close before either packet client starts. Independent selected address/subnet
readback and two exact peer nonce ACKs pass across the retained separate namespaces.
Profile-down completes, independent station address/subnet absence passes, and
the original 800 ms quiet request has zero peer datagrams plus an actual final
EAGAIN. The measured peer-arm-to-final-EAGAIN interval is 799,963,664 ns; delivery
and final sampling are recorded separately, with no renewed deadline. Exact L2
readback still agrees before and after removal and quiet. The later separate
normal Rust wireless Disconnect and independent disconnection readback pass.

All 150 direct waits have stdout EOF: 15 host, three boot, 107 guest, 24 address-case
and one poweroff. Thirty-three successful adopted terminal waits precede actual
ECHILD, without inferred origins or PIPE EOF. The case Scope retires before L2
Disconnect and radio removal. Actual AP/peer/keeper closure, control-endpoint
absence, hwsim removal and both retained namespace-descriptor closures qualify.
QEMU, container and attached CLI exit 0; the exact container is removed on the
same daemon, selected inputs remain unchanged, and cleanup errors are empty.

The complete 284,571-byte final guest record survives durable publication. This
epoch actually observes inherited and effective FSIZE limits as infinity, its
own same-PID exec bootstrap as pixel-sim MainPID/parent PID1, and the unchanged
SIGTERM default disposition. The pending-TERM snapshot before publication is
false; it does not establish the unique cause of the older interruption or a
natural guest exit 0. The earlier failed whole cohort remains FAILED. The new
results directory uses underlying `/tmp`; both that filesystem and retained B
must independently have at least 2 GiB free, with all original execution cutoffs
and VM resources preserved.

The [fixture snapshots](../tools/native-wireless-association/address-packets/README.md)
and [hash-only provenance](evidence/native-wireless-address-packets-provenance.json)
keep setup failure, real refusal RED, source-bound build and native outcomes separate. Images, executables and raw private runtime
records stay outside Git. No physical interface or phone is used. This does not
repair the separate ARM persistent-worker ReceiveCommand 50 ms failure or integrate
Settings, user consent, WPA, DHCP, DNS, default routing or Internet access.
