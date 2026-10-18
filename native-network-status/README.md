# Native local network status

Readonly coarse observations from the deployment-inherited network namespace. Main
retains its namespace identity; each finite owned collector verifies that identity
and uses the existing Rust `KernelObserver`. Fresh collections do not establish
continuous interface lifetimes or provider associations.

`pokelinux-network-status --serve SOCKET` starts the root UID/GID 0 service. The
production unit selects `/run/pokelinux-network/snapshot.sock`. Connect receives one
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
ARM64 release binaries compiled and linked. That build-only record does not
include release execution; the later installed ARM result below is separate.

A separate [Rust reader and Settings card](../docs/native-network-status.md) now
pass host real-service integration and offscreen loss/expiry/recovery. They do not
inherit this ARM service cohort.

TUN creation/disappearance integration, ARM typed Rust-client integration,
Settings presentation, Internet reachability, modem behavior, and physical-device
validation remain unqualified by these service runs.


## Installed ARM service

The normal ARM64 release now passes a distinct installed-service sequence in a
generic353-package guest. The unchanged positive consumer first fails because
the unit is missing. Host production Make then stages exactly the daemon and
[unit](packaging/pokelinux-network-status.service), with no site config. Preparation
installs those bytes, checks native loader relocations and the unit, and enables
it while stopped. It performs no manual service start.

A separate fresh boot starts the enabled service through PID1 before any install,
reload, enable or start command. An independent Python consumer verifies the
original package versions, only-loopback kernel namespace, exact installed
executable/root socket peer and a fresh available0/0 snapshot with stream EOF.
The same Main/invocation remains selected around that observation. PID1 Stop then
records normal exit0, dead Main pidfd, empty cgroup and removed socket. Each VM
closes its actual same-parent QEMU wait and PIPE EOF under the original cutoffs.
The exact consumer is retained as
[`network-status-probe.py`](../tools/native-installed-services/network-status-probe.py).
It runs inside the selected disposable guest with `/results/selection.json`, the
authored `/run/pokelinux-network-inputs` payload and explicit original deadlines.
See [hash-only installed provenance](../docs/evidence/native-installed-network-status-provenance.json).

The service runs as root with Type=exec, Restart=no, KillMode=control-group and a
five-second Stop limit. Its runtime directory is root0:0/0755; the public snapshot
socket is root0:0/0666. The existing install-native-services Make target stages
the0755 executable and0644 unit; enablement is an explicit image-preparation step.
This result uses a Python socket consumer. It supplies no ARM typed-client or
Settings UI evidence, interface-mutation test, Wi-Fi/Internet/modem connectivity,
or physical-phone qualification.


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
