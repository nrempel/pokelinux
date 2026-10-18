# Read-only local network status client

Rust owns the socket, response validation, deadlines and copied state. The four
C operations in `include/pokelinux_network_status_client.h` create a client, tick
it, read its view and free it. There is no worker thread, network mutation,
provider selector or request packet. Qt only presents the copied scalar fields.

The fixed endpoint is `/run/pokelinux-network/snapshot.sock`. A successful
connection must have an actual root:root peer with a positive PID. The client
reads at most512 bytes through nonblocking operations and requires exactly one
newline-terminated JSON object followed by EOF. All six service fields are
required; unknown/duplicate fields, invalid count relationships and noncanonical
or invalid deadlines withdraw both counts. An unavailable wire response has null
counts and deadline `"0"`; the C view uses `-1` counts and zero revision.

Healthy empty `0/0` is available. Counts exclude kernel loopback interfaces and
include observed address records that may be tentative, down or link-local.
They establish no Internet, Wi-Fi, route, signal or modem connection.

Polling starts no more often than every250ms. Each request retains its original
500ms `CLOCK_BOOTTIME` deadline; partial bytes never extend it. Accepted service
leases can extend no more than two seconds into the future. Every getter checks
expiry, including when no timer tick has run. A malformed/failed/expired read
clears the previous counts; a new read-only connection can recover.

## Tests

In an isolated environment with real/effective UID and GID0, run:

```sh
cargo test --locked --offline --manifest-path native-app/network-status-client-core/Cargo.toml
cargo fmt --manifest-path native-app/network-status-client-core/Cargo.toml -- --check
cargo clippy --locked --offline --manifest-path native-app/network-status-client-core/Cargo.toml --all-targets -- -D warnings
```

All seven default tests, fmt and strict all-target Clippy pass. Separate normal
host/ARM archives compile and expose exactly the four C operations. That original
ARM archive record is build-only. A later native ARM Qt6.12 run executes this real
reader through the UID11050 normal-QML component with the installed PID1 service:
0/0 → Stop → getter expiry without Qt events → dashes → fresh invocation →0/0.
The same component/adapter recovers; both PID1 Stops report normal exit0 and empty
cgroups. The normal Settings ELF is built/installed, without running its main or
showing a window. See the [service/client report](../../docs/native-network-status.md).

The ordinary tests use real Unix sockets for healthy-empty publication, zero
request bytes, malformed and partial replies, withheld EOF, original deadline
expiry without timer delivery, service loss/reconnection, strict field/count
validation and C ABI pointer/lifetime checks. Their controlled timestamps are
component evidence, separate from physical suspend/wake.

`tests/client-observe.c` is the frozen positive C ABI consumer used against the
normal Release daemon. It requires UID/GID11050 with zero effective capabilities
in the same independently verified loopback-only namespace. Its case fixture
provides `/out`, compiles/links this exact source, and starts the normal daemon
with the fixed endpoint. The consumer records its observation, frees its Rust
client, and waits for `/out/daemon-stopped.json`; the fixture writes that marker
only after actual daemon Main0 and PIPE EOF, then records the consumer's actual
exit. The nonconnecting stub failed its healthy-empty assertion; the real Rust
reader passes the same assertion. A setup/compile/cleanup failure is not RED.
