# Native cellular owner observations

One fixed actual ARM Binder/kernel test fails 101 for missing consume/join after
healthy raw-callback/kernel prerequisites, then passes 0 on the first repair.
The 42 direct child waits/EOF and Root's full input/container closure are qualified.

`pokelinux-cellular-owner` retains one remote Data endpoint, a fresh callback pair
and one `KernelObserver` for a single attempt. It tracks in-memory provider
submission history separately from a current owner-local kernel association.
It provides no APN acquisition, route/carrier/internet or installed permission
authority. The crate forbids unsafe Rust.

`mark_may_have_submitted` changes `NotSubmitted` to `OutcomeUnknown` once, before
the caller's possible submission. Callback attachment and wire submission use
the retained endpoint externally; this is not an owned transport-worker API.
Production reduction validates expected UID, ordinary callback type, serial and
bounded shape before recording the first matching provider report. Both radio
error and data-call cause must be `NONE` for `ProviderAccepted`; otherwise the
report is historically rejected. `poll()`
drains events and reads a fresh complete snapshot from the retained observer.
Cached `last_association` is an observation, not current authorization.

The source join uses witnessed namespace/index/lifetime, a unique matching name,
UP/nonzero MTU and exactly one eligible matching local IP/prefix record with
allowed scope/flags. Raw duplicate variants remain unqualified. The initial subset accepts
nonempty matching IP families and infinite Android address lifetimes only.
Any valid list conservatively retires the association; death and kernel loss
also retire it without rewriting accepted provider history. Provider-reported
DNS/P-CSCF/gateway/name/address values do not establish link ownership or APN
readiness. Structural policy/error negative controls remain unqualified.

The fixed actual-kernel test receives real private-inbox Binder events and calls
the production reducer directly. It does not execute ordinary `poll` queued
Setup/List/Died dispatch. The repaired run checks ordinary polling for a fresh
join and real kernel-loss withdrawal; its later list/death events occur after
withdrawal, so do not independently prove withdrawal from a positive state.
See the [report](../docs/native-cellular-data-owner.md) and
[provenance](../docs/evidence/native-cellular-data-owner-provenance.json).

From the repository root, compile checks only:

```sh
cargo fmt --manifest-path native-cellular-owner/Cargo.toml -- --check
cargo clippy --locked --offline --manifest-path native-cellular-owner/Cargo.toml --all-targets -- -D warnings
cargo test --locked --offline --manifest-path native-cellular-owner/Cargo.toml --lib --no-run
cargo test --locked --offline --manifest-path native-cellular-owner/Cargo.toml --lib --no-run --target aarch64-unknown-linux-gnu
cargo build --locked --offline --manifest-path native-cellular-owner/Cargo.toml --release --target aarch64-unknown-linux-gnu
```

The actual test is ignored by default and needs its isolated ARM Binder/TUN
controller and complete owned custody; compiling or skipping it is not a pass.
`fixture.rs` and the fixed kernel test compile only under `cfg(test)`, with the
bindings' authored control feature enabled by the development dependency.
No host behavioral test result is claimed.

Process-global Binder pools, local `stop`/drop and process waits do not qualify
owned-worker/pool joins, responsive installed Stop, remote cleanup, durable
permissions, journal/persistence or recovery. Target modem/slot/SIM/APN,
provider-to-link authority, ACK_EXP, installed services and matched Google/Pixel
kernel/policy/hardware remain open. No physical phone was accessed.
