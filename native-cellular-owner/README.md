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

The earlier fixed RED/GREEN test receives real private-inbox Binder events and
calls the production reducer directly. Its later list/death events follow
kernel-loss withdrawal. That evidence and its original source pins remain in
[RED/GREEN provenance](../docs/evidence/native-cellular-data-owner-provenance.json).

Three additional actual ARM tests use public owner methods throughout. Ordinary
`poll()` consumes the matching setup callback before any TUN exists, then joins
a freshly created controlled link. Separate runs withdraw on actual sole-FD
link loss, empty-list indication and endpoint death. List/death start from a
fresh positive association and withdraw while the controller independently
confirms the same kernel link and addresses remain. The list peer stays live;
the death peer is actually waited with exit 0 and EOF before that confirmation.
Accepted CID 9 and the complete provider report survive every withdrawal.

All three pass against unchanged production code, with 71 additional direct
child waits/EOF (21/25/25); this is additional coverage, not a new RED or repair.
Only test code is appended, retaining the earlier fixed test as an exact prefix.
Unknown-serial ordinary `poll()` dispatch and the other negative controls remain
open. See the [report](../docs/native-cellular-data-owner.md) and
[new provenance](../docs/evidence/native-cellular-data-public-poll-provenance.json).

From the repository root, compile checks only:

```sh
cargo fmt --manifest-path native-cellular-owner/Cargo.toml -- --check
cargo clippy --locked --offline --manifest-path native-cellular-owner/Cargo.toml --all-targets -- -D warnings
cargo test --locked --offline --manifest-path native-cellular-owner/Cargo.toml --lib --no-run
cargo test --locked --offline --manifest-path native-cellular-owner/Cargo.toml --lib --no-run --target aarch64-unknown-linux-gnu
cargo build --locked --offline --manifest-path native-cellular-owner/Cargo.toml --release --target aarch64-unknown-linux-gnu
```

The four actual tests are ignored by default and need their isolated ARM
Binder/TUN controllers and complete owned custody; compiling or skipping them
is not a behavioral pass.
`fixture.rs` and the kernel tests compile only under `cfg(test)`, with the
bindings' authored control feature enabled by the development dependency.
No host behavioral test result is claimed.

Process-global Binder pools, local `stop`/drop and process waits do not qualify
owned-worker/pool joins, responsive installed Stop, remote cleanup, durable
permissions, journal/persistence or recovery. Target modem/slot/SIM/APN,
provider-to-link authority, ACK_EXP, installed services and matched Google/Pixel
kernel/policy/hardware remain open. No physical phone was accessed.
