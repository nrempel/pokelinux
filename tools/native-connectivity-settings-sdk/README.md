# Normal Settings Rust client to native connectivity

This fixture links the normal Settings crate's safe Rust `connectivity::Engine`
into an actual launched UID11050 Settings process. It exercises the production
lifecycle actor loop, durable NetworkControl grant, root-private connectivity
Main and native worker, and independent real UDP packets in disposable network
namespaces. It does not exercise the Settings C ABI, Qt adapter, normal window,
authentication, genuine session Source, installed session-owner executable or a
physical phone. Controlled session/admin facts remain authored fixture inputs.

Stage exact current lifecycle sources and their path dependencies, plus
`native-settings-ui/client-core`, at their repository-relative paths. Copy this
manifest, lock and SDK app into a sibling `settings-sdk-app-fixture/`. Do not run
Cargo directly in this nested tools directory. Copy the consumer into the staged
lifecycle `src/` and append to its copied `src/service_restore_tests.rs`:

```rust
#[path = "service_connectivity_sdk_actor_tests.rs"]
mod connectivity_sdk_actor_tests;
```

Use the selected offline registry and locked Cargo. Select actual compiler JSON
artifacts for the SDK app and lifecycle library test, and matching normal Main,
worker, launcher and daemon ELFs. The exact ignored library-test filter is:

```text
service::restore_tests::connectivity_sdk_actor_tests::settings_sdk_connect_packet_lifecycle
```

Mount `controller.py`, the unchanged [shared packet peer](../native-connectivity-actor/peer.py),
and existing owned.py/closure.py helpers under `/runner/`. The controller takes the
same six absolute ELF options and output/original aggregate deadlines as the
[registered app fixture](../native-connectivity-actor/README.md). Preserve its
capability split, protected executable ancestry and separate writable fixture
paths; use no host network. Set `POKELINUX_CONNECTIVITY_ISOLATED=1` only in that
fresh disposable environment.

The SDK receives the original absolute BOOTTIME action deadline. Acceptance is
not an acknowledgement: the consumer requires fresh matching observed state,
without an uncertain outcome, before that same original cutoff. It checks local
refusal without a grant, actual granted Connect, an independent nonce roundtrip,
SDK Stop and actual join, actual app wait, a new app process and two fresh status
observations, another independent packet, explicit Disconnect, absent address
and default route, exact ENETUNREACH101, and the full quiet period.

The six-second case, one-second startup/action cutoffs and 800ms quiet phase are
fixed. Quiet timing includes gate delivery; the armed-to-final-EAGAIN interval is
a separate measurement. Apps stdout/stderr are null: actual Apps waits, SDK joins
and task-count-one observations are separate from the owned command PIPE EOFs.
Actor join and Main's native-worker wait/output EOF remain separate too. The
final desired assertion runs after backend cleanup. Reaping is not vendor cleanup.

The refusing SDK baseline reached a genuine behavioral RED after healthy setup
and cleanup. Its formatted consumer bytes are frozen for the implementation
check. Hash-only results and remaining gates belong in the corresponding public
evidence report; private logs, runtime nonces, binary artifacts and inputs are
not published here.
