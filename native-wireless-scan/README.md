# Native one-shot wireless operations

Rust Linux `nl80211` operations for a newly created, exclusively administered
radio epoch: discovery, explicit open association and explicit Disconnect.
There are no credentials, IP configuration, automatic retry or background
thread. These operations are not enabled in the phone deployment.

The caller selects a positive ifindex, integer frequency in MHz and the original
absolute CLOCK_BOOTTIME deadline, at most five seconds away:

```text
pokelinux-wireless-scan IFINDEX FREQUENCY_MHZ ORIGINAL_UNTIL_NS
```

The worker retains the current thread's network namespace, resolves the family
and scan multicast group, checks the actual station/wiphy/wdev identity, and
subscribes before the sole trigger. ACK0 and a matching completion event precede
a complete, generation-consistent BSS dump. Observed loss, truncation,
interruption, duplicate selected attributes, stale or contradictory scan events,
changed identity, missing timestamps and deadline expiry refuse completion.
SSID bytes are reported as lowercase hex. The original deadline governs
nonblocking receive and an absolute BOOTTIME timerfd, without renewal.

A definitely unsupported preflight returns code2 and explicit unavailable JSON.
Failures after trigger admission are uncertain. Nothing aborts or repeats the
scan. Socket closure or process reaping establishes no kernel scan cancellation
or vendor cleanup. The caller must retain the worker through actual wait and
stdout EOF under the same cutoff, and supervise a blocked kernel call by owning
the whole worker process. This primitive does not provide that supervisor.

Names, ifindex, namespace inventories, interface checks and FLUSH do not prove
exclusive ownership. nl80211 supplies no generic per-scan cookie. The qualifying
fixture creates its own fresh hwsim radios in a disposable guest; this contract
is not established for a shared or recovered physical radio.

```sh
cargo test --locked --manifest-path native-wireless-scan/Cargo.toml
cargo clippy --locked --manifest-path native-wireless-scan/Cargo.toml --all-targets -- -D warnings
```

The normal Release CLI and host component tests are separate evidence. See the
[wireless tracer report](../docs/native-wireless-scan.md) for actual ARM tests,
original-tool comparison, remaining Pixel backend work and hardware gates.
Own unsafe Rust is forbidden. The private netlink socket is uniquely owned and
never clones or exposes the dependency's raw-descriptor wrapper.

## Explicit open association

The reusable `associate_open_once(AssociationRequest)` selects an exact positive
ifindex, nonzero unicast six-byte BSSID, one to 32 SSID bytes and an integer
frequency, with an original absolute BOOTTIME cutoff at most five seconds away.
The normal CLI accepts only this explicit spelling:

```text
pokelinux-wireless-scan associate-open IFINDEX FREQUENCY_MHZ BSSID_HEX SSID_HEX ORIGINAL_UNTIL_NS
pokelinux-wireless-scan disconnect IFINDEX ORIGINAL_UNTIL_NS
```

Hex is lowercase and bounded; no credentials or aliases are accepted. The scan
invocation above remains the normal discovery operation.

CONNECT restricts BSS selection by exact MAC/frequency and requests open-system
authentication. It subscribes to MLME before the sole mutation and requires its
selected success event with matching peer, no timeout, and actual associated-BSS
readback for exact BSSID/SSID/frequency. Authentication/association progress and
ACK0 do not complete the operation. A protected BSS, changed namespace/identity,
loss, contradictory event or incomplete dump refuses completion.

The independent `disconnect_once(DisconnectRequest)` is a separately requested
action after confirmed association. It requires the local reason 3 Disconnect
event and actual complete readback with no associated BSS. No socket-owner disconnection is requested on worker exit.

The same fresh exclusive-radio precondition applies. MLME has no generic
per-request cookie here, so subscription and identity checks do not replace
trusted exclusive administration. Any possibly dispatched failure remains
uncertain. Callers must fence that epoch without reconnect, resubmission,
automatic Disconnect, rollback or a refreshed action cutoff, and retain each
worker through actual wait and stdout EOF. These primitives provide no generic
kernel-syscall cancellation or supervisor.

The [separate association consumer](../tools/native-wireless-association/README.md)
uses independent station/AP kernel output and later explicit Disconnect
readback. Association does not configure an address or prove packets/Internet.
