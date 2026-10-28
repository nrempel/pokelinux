# Rust connectivity control

This component separates short-lived fixed-profile commands from background
connection ownership. `owner` accepts only trusted in-process principal/grant
identities, keeps one mutation slot, fences departed or expired requester replies
and makes uncertain results terminal for an owner epoch. It does not authenticate
applications or connect a Settings socket.

`fixed_address::ExistingIpv4Profile` also manages the fixed address on an existing
link whose lifecycle is exclusively retained by a trusted caller. It binds the
constructing thread, namespace descriptor and continuous link observation;
conflicts or possibly dispatched failures permanently fence that attachment.
The explicit `profile-up` / `profile-down` worker modes now pass actual ARM
virtual-radio address setup, two packet round trips, removal and quiet after
separately qualified Rust association. See [the API and packet report](../docs/native-wireless-address-packets.md).
This does not change the persistent Service's virtual-link backend or repair its
separate ARM ReceiveCommand failure; Settings Wi-Fi integration remains open.

`kernel::VirtualLink` is a Linux development backend for a fresh private virtual
pair. It retains both namespaces, subscribes before creation, creates the pair
with its own rtnetlink socket and observes its witnessed local lifetime and
administrative-up state. `set_connected` adds/removes only the fixed local
192.0.2.1/30 profile and requires actual readback after its kernel ACK.
`connected` is local configuration, not Internet reachability; `fact()` is cached.
Exclusive administration is a trusted supervisor
precondition. It cannot be established by checking names, interface indices or
loopback-only inventory. A possibly dispatched failure fences this backend epoch
without mutation retry or rollback.

Nonblocking flags do not bound synchronous kernel rtnetlink dispatch.
`service::Service` now isolates the backend in one persistent cooperative
`native-process-owner` worker. Trusted single-threaded startup retains both
namespaces, launches the fixed worker and irreversibly removes Main's
capabilities; the worker retains only NET_ADMIN.
`pokelinux-connectivity-service` is the worker entry. The separate
`pokelinux-connectivity-main` owns the protected root endpoint; neither is
installed by this checkpoint, and applications cannot access that endpoint.

Main snapshots expire both observation paths from completed kernel readback
under CLOCK_BOOTTIME. Idle renewal requires another actual readback, never a
heartbeat. Original action expiry permanently fences the epoch; late retirement
receipts cannot renew observations. Stop fences admission/publication before
signaling and keeps custody until actual wait and stdout EOF. Actual wait also
revokes cached readiness before a held stdout writer closes. Neither Stop nor
BOOTTIME checks promise syscall cancellation or reclaimability of arbitrary
blocked kernel calls. No implicit retry, replacement, Disconnect or rollback is
performed; local descriptor/process release is distinct from link/vendor cleanup.

Worker errors now retain a fixed stage, the selected call cutoff when known, an
error-only BOOTTIME sample and the original `io::Error`. An unchanged real cap0
CLI consumer passes after missing-context RED; the worker still exits naturally
with code1 and source EPERM. Only `ReduceCapabilities` is executed by that control;
all seven contexts compile. See [error-context evidence](../docs/native-connectivity-worker-error-context.md).
No payload, guard, deadline, retry or retirement policy changes.

The separate `main_service` and nonblocking `client` now pass the unchanged
normal-Main packet consumer against the final source after an actual typed-refusal
RED: UID11050 private access denial, traffic after requester close, fresh
observations and explicit Disconnect/quiet. A separate real Unix accept-error
RED precedes the stop guard; the frozen regression passes with no reply after
Stop. Both active library tests, eight owner tests, formatting and strict Clippy
pass. See the [private-ingress report](../docs/native-connectivity-private-ingress.md).
This adds no Settings or actor-permission result.

```sh
cargo fmt --manifest-path native-connectivity-control/Cargo.toml -- --check
cargo clippy --locked --offline --manifest-path native-connectivity-control/Cargo.toml --all-targets -- -D warnings
cargo test --locked --offline --manifest-path native-connectivity-control/Cargo.toml --lib --test owner
```

The library's cached-readiness regression uses a real child, actual wait and a
retained stdout writer, with an authored cached fact. It does not start the
privileged Service or create a virtual link. Its ignored self-exec helper is
selected only by that parent test. The [worker report](../docs/native-connectivity-worker.md)
separately qualifies actual startup, capability split, packets, idle freshness,
stopped-worker expiry/resume and queued-command Stop.

The ignored `actual_rust_owned_virtual_link_creation` test requires a trusted
isolated fixture with two fresh retained namespaces, exclusive administration,
`CAP_NET_ADMIN` for the creating Rust child, and an explicitly retained peer
namespace path in `POKELINUX_CONNECTIVITY_PEER_NS`. The fixture also sets
`POKELINUX_CONNECTIVITY_ISOLATED=1`. Ordinary Cargo tests skip that integration;
compiling or skipping it establishes no kernel result. The fixture must keep both
namespaces until all owner/keeper children are actually waited and their output
pipes close. Never run it against a phone or shared host namespace.

The [design and evidence report](../docs/native-connectivity-control.md) keeps
policy, actual kernel/packet, installed service, Settings and hardware gates
separate. Existing NetworkManager and local-network-count services remain
independent read-only paths. No Wi-Fi, DNS, Internet, Bluetooth or physical-device
claim follows from this crate.

The [actual host packet case](../docs/native-connectivity-packets.md) now passes
two nonce roundtrips, fixture-principal withdrawal and explicit Disconnect with
independent address removal and peer quiet. The [public fixture source](../tools/native-connectivity-control/)
provides its controller/peer and setup contract. This remains separate from OS
permissions, a real Settings window and an installed production service.

The subsequent [registered app actor tracer](../docs/native-connectivity-actor.md)
uses this unchanged Main/worker through the private Rust client and real durable
grants. Actual app exit preserves packets; a fresh generation explicitly
Disconnects. That actor cohort does not execute the normal Settings engine or
window; the [later normal window case](../docs/native-settings-connectivity-window.md)
passes separately in an x86 VM. Full installed runtime, ARM window/packets and
hardware remain open. This crate receives no app-controlled identity, grant or
backend targeting over its private wire.

The [disabled ARM development bundle](packaging/README.md) now passes actual
Make/installer staging of the two normal providers, preserving an existing
Disabled configuration. Eight real filesystem installer tests pass. This is
host deployment evidence, separate from service activation and the failed ARM
normal-window/packet integration.
