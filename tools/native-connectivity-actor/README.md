# Registered app to native connectivity actor fixture

This source exercises an actual launched UID11050 Rust process under the Settings
profile, the existing lifecycle actor and durable grant worker, the normal
root-private connectivity Main and independent real UDP packets. It is a process
fixture, not the normal Settings Qt window or its production client engine.

The Rust consumer, app and Python controller/peer are frozen across the qualified
refusing-actor RED and subsequent implementation test. No binary, raw runtime
receipt, nonce, packet capture, credential or proprietary input is published.
Publishing this source is not an additional execution of a public entrypoint.

Use fresh exact copies of the current lifecycle manifest and its path dependency
crates. Copy the app manifest/lock/source into a sibling `settings-app-fixture/`
next to `native-lifecycle/`, retaining the manifest's measured relative paths.
Do not invoke Cargo directly in this nested tool directory. Copy
`service_connectivity_actor_tests.rs` into the staged lifecycle `src/` and append
these exact lines to its copied `src/service_restore_tests.rs`:

```rust
#[path = "service_connectivity_actor_tests.rs"]
mod connectivity_actor_tests;
```

Build with the selected offline registry and locked toolchain. Select the actual
Cargo compiler-artifact ELFs for normal connectivity Main/worker, lifecycle
launcher/daemon, Settings app fixture, and lifecycle library tests. The actual
library filter is:

```text
service::restore_tests::connectivity_actor_tests::settings_actual_connect_packet_lifecycle
```

It is an explicitly ignored integration case. Its controlled `session=None` and
authenticated admin lock/background settings are authored facts; no genuine
Source, authentication or graphical shell is exercised. The production actor loop
runs in the library-test process with existing cfg(test) helper selection and
app-wait observation; no installed session-owner selection is qualified.

Run only in a fresh, exclusively administered disposable network namespace,
without host networking. The initial fixture capabilities are NET_ADMIN,
SYS_ADMIN, SETPCAP, SETUID, SETGID and KILL. The actor retains only the latter four;
Main, peer and controller drop every capability, while the owned network worker
retains only NET_ADMIN. The app has zero effective/permitted/inheritable/ambient
capabilities and NoNewPrivs, while its bounding set retains four capabilities.
This distinction is checked, not hidden by a claim that every set is zero.

Mount controller.py, peer.py and the existing owned.py/closure.py at `/runner/`.
Use protected executable ancestry and separate writable fixture paths under
`/opt`, `/run` and `/var/lib`; the actual app must not be able to write selected
ELFs. Set `POKELINUX_CONNECTIVITY_ISOLATED=1`. Supply absolute selected ELF paths
and original aggregate deadlines:

```sh
python3 -B /runner/controller.py \
  --main-elf "$MAIN_ELF" --worker-elf "$WORKER_ELF" \
  --driver-elf "$LIBTEST_ELF" --settings-app-elf "$APP_ELF" \
  --launcher-elf "$LAUNCHER_ELF" --storage-elf "$DAEMON_ELF" \
  --output "$FRESH_OUTPUT" \
  --work-until-ns "$ORIGINAL_WORK_UNTIL_NS" \
  --close-until-ns "$ORIGINAL_CLOSE_UNTIL_NS"
```

The six-second functional case starts before Main. Startup/actions retain their
original one-second deadlines. The original 800ms quiet deadline includes gate
delivery; armed-to-final-EAGAIN timing is a separate measurement. The packet peer
requires an explicit exact count: zero on the refusing branch, two on success.
The first runtime's old two-packet quiet prerequisite caused a preserved setup
failure before its desired assertion and does not count as RED.

Actual Apps Child::try_wait observations qualify app exit. App stdout/stderr are
null, so no app PIPE EOF is claimed. Actor join, normal Main's own native-worker
wait/EOF, independent worker pidfd and Scope wait/EOF receipts are separate.
The positive desired assertion runs only after backend cleanup. Process reaping
is not evidence of vendor cleanup, service restart or hardware support.
