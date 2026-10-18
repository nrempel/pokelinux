# Native local network status

Readonly coarse observations from the deployment-inherited network namespace. Main
retains its namespace identity; each finite owned collector verifies that identity
and uses the existing Rust `KernelObserver`. Fresh collections do not establish
continuous interface lifetimes or provider associations.

`pokelinux-network-status --serve SOCKET` starts the root UID/GID 0 service. The
intended deployment endpoint is `/run/pokelinux-network/snapshot.sock`; service
packaging and installation are not part of this milestone. Connect receives one
JSON line and EOF, with no request or mutation protocol. The six fields are
`schema`, `revision`, `until_ns`, `available`, `local_interfaces`, and
`address_records`. `until_ns` is an absolute CLOCK_BOOTTIME decimal string.

Counts exclude interfaces carrying the kernel `IFF_LOOPBACK` flag. Address-record
counts include observed tentative, down, and link-local records; they do not mean
usable connections. Healthy empty `0/0` is available. Unavailable snapshots have
null counts and `until_ns` equal to `"0"`. Interface names, addresses, MACs, and
SSIDs are not published. Counts establish no route, Internet, Wi-Fi, transport,
signal, modem, or physical connectivity result.

The original collection deadline is one second and the view lease is two seconds
from original collection start. Main retains at most one collector through actual
child wait and PIPE EOF. Invalid, missing, withdrawn, or late completion clears
publication; getter expiry does not retire a reader. Expiry and Stop fence
publication before queued completion is considered. Private `cfg(test)` collector
substitution is confined to the fixture-driver test ELF. Normal release binaries
have no fixture CLI modes.

The hash-only [validation record](validation/local-kernel-reader.json) separates
actual compiled publication-model RED, the same three model and four service
groups GREEN, the formatter bridge, a preserved test-only Clippy failure, and the
final strict checks. The normal debug service published healthy empty counts from
an independently verified isolated namespace containing only loopback. Authored
`3/7` helper counts qualify process lifecycle controls only. Normal x86-64 and
ARM64 release binaries compiled and linked; neither release ELF was executed.

TUN creation/disappearance integration, installed-service/PID1 behavior, client
server-identity enforcement, Settings presentation, Internet reachability, modem
behavior, and physical-device validation remain unqualified.


## Reproducing the tests

Run commands from the repository root with the pinned dependencies available
locally. Default Cargo testing runs the three model tests and ordinary lib/bin/doc
targets. `fixture-driver` is an argument-driven process helper, so `test = false`
excludes its no-argument entry from default testing. Explicitly selecting the
helper still compiles it; its consumer is the permanent four-case Python driver.
See [Cargo target selection](https://doc.rust-lang.org/cargo/commands/cargo-test.html#target-selection).

```sh
export CARGO_TARGET_DIR=/tmp/pokelinux-network-status-target
cargo test --locked --offline --manifest-path native-network-status/Cargo.toml
cargo fmt --manifest-path native-network-status/Cargo.toml -- --check
cargo clippy --locked --offline --manifest-path native-network-status/Cargo.toml --all-targets -- -D warnings
cargo clippy --locked --offline --manifest-path native-network-status/Cargo.toml --test fixture-driver -- -D warnings
cargo build --locked --offline --manifest-path native-network-status/Cargo.toml --bin pokelinux-network-status
cargo test --locked --offline --manifest-path native-network-status/Cargo.toml --test fixture-driver --no-run
```

Cargo reports the exact helper executable path in the final command. In a caller
created isolated network namespace containing only loopback, with real/effective
UID and GID 0 and no effective capabilities, pass that path to:

```sh
python3 native-network-status/tests/run_service_cases.py \
  --normal /tmp/pokelinux-network-status-target/debug/pokelinux-network-status \
  --fixture /exact/helper/executable/reported/by/Cargo \
  --output /tmp/network-status-cases
```

The output directory must be fresh. The driver independently checks the baseline,
uses `tools/native-installed-services/owned.py`, and runs the unchanged cases in
`tests/service_cases.py`: actual healthy-empty kernel publication, wait and EOF
before controlled publication, Stop with a held collector, and rejection of a
queued successful completion after its original deadline. Its original global
work/close budgets are 30/36 seconds; each original case retains its six-second
cap. A case failure or incomplete actual wait/EOF makes the driver fail. Test
execution does not mutate network state or create a TUN interface.
