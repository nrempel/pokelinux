# Rust connectivity control

This component separates short-lived fixed-profile commands from background
connection ownership. `owner` accepts only trusted in-process principal/grant
identities, keeps one mutation slot, fences departed or expired requester replies
and makes uncertain results terminal for an owner epoch. It does not authenticate
applications or connect a Settings socket.

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

Nonblocking flags do not bound synchronous kernel rtnetlink dispatch. Run this
component outside a responsive service Main; owned worker supervision and Stop
integration remain open. Absolute BOOTTIME checks reject late completion, without
claiming syscall cancellation. No implicit mutation retry or destructive rollback
is performed; descriptor/process release is distinct from link/vendor cleanup.

```sh
cargo fmt --manifest-path native-connectivity-control/Cargo.toml -- --check
cargo clippy --locked --offline --manifest-path native-connectivity-control/Cargo.toml --all-targets -- -D warnings
cargo test --locked --offline --manifest-path native-connectivity-control/Cargo.toml --test owner
```

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
