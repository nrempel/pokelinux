# Native one-shot wireless discovery

A Rust worker for one Linux `nl80211` scan in a newly created, exclusively
administered radio epoch. It has no association, credentials, IP configuration,
automatic retry or background thread. It is not enabled in the phone deployment.

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
