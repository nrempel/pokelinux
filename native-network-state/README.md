# Native kernel network state

`pokelinux-network-state` is a read-only Rust `NETLINK_ROUTE` producer for the
constructing thread's retained network namespace. It reports kernel link/address
observations; it grants no Data/Binder, APN, route, carrier, internet or installed
provider authority. The implementation forbids unsafe Rust.

`KernelObserver::subscribe_current_namespace(until_boottime_ns)` retains the
namespace and constructs/subscribes its own nonblocking `CLOEXEC` socket together.
`refresh(until_boottime_ns)` returns complete matching link/address dumps,
namespace identity, notification revision and a boot-time timestamp. A link's
`observed_lifetime` is present only when its creation was continuously witnessed;
pre-existing links have no such witness. IPv4 local and peer addresses remain
distinct. The caller cannot attach an arbitrary socket to a different namespace.

Snapshots fence publication before an absolute `CLOCK_BOOTTIME` deadline and
refuse interleaved changes; they cannot guarantee that a link remains present
when used later. `ConcurrentChange` is the only retryable error variant. Every
other error, including `Deadline`, makes that observer terminal. Source limits
are64 links,16 addresses per link,64KiB per datagram,256 messages/datagrams and
1MiB received bytes per refresh. Runtime negative controls, rename/reuse,
concurrency, overflow, suspend and load qualification remain open.

One fixed ignored-by-default kernel test actually returned101 for the explicit
missing-observer baseline, then0 after the library repair, under a generic
AArch64 Linux7.1.6 VM kernel. It observes a witnessed point-to-point TUN link with
distinct IPv4 local/peer and IPv6 `nodad` addresses, then withdraws the lifetime
after the independent owner closes its sole TUN FD. The32 observer child
waits/EOFs and a separate15-wait TUN prerequisite have distinct custody evidence.
The [report](../docs/native-network-observer.md) and
[provenance](../docs/evidence/native-network-observer-provenance.json) describe
credentials, mapped ELF closure, retained failures and integration gates.

From the repository root, these commands perform compile checks only:

```sh
cargo fmt --manifest-path native-network-state/Cargo.toml -- --check
cargo clippy --locked --offline --manifest-path native-network-state/Cargo.toml --all-targets -- -D warnings
cargo test --locked --offline --manifest-path native-network-state/Cargo.toml --lib --no-run
cargo test --locked --offline --manifest-path native-network-state/Cargo.toml --lib --no-run --target aarch64-unknown-linux-gnu
cargo build --locked --offline --manifest-path native-network-state/Cargo.toml --release --target aarch64-unknown-linux-gnu
```

No host behavioral test result is claimed. The actual kernel test requires its
isolated ARM VM controller, independent TUN/IP controls and owned process/module/
namespace cleanup; ordinary `cargo test` skips it. Its exact test name is
`kernel_tests::actual_kernel_link_lifetime`. Do not infer a VM or phone pass from
compiling or skipping it. Installed service/Stop/persistence, permissions,
provider association, modem/SIM/carrier and matched Google-kernel policy remain
unqualified. No physical phone was accessed.
