# Root-private connectivity ingress fixture

The authored driver, controller, manifest and actually generated 4809-byte fixture
lock are byte-identical across the private Main/client RED and both packet GREEN
cohorts. Development comments retain their original preparation wording. No ELF,
runtime receipt, nonce, captured packet or private envelope is included. Publishing
this source is not a separate execution of a new public entrypoint.

Preserve the measured sibling build layout: copy Cargo.toml, Cargo.lock and
driver.rs into fresh `private-ingress-fixture/` beside selected exact copies of
`native-connectivity-control/`, `native-network-state/`, `native-process-owner/`
and `native-kernel-identity/`. Do not invoke Cargo directly in this nested tool
directory or silently regenerate the lock. Use the selected cached offline
registry/toolchain. Build normal Main/worker from the control manifest and compile
the harness=false driver, selecting actual compiler-artifact ELFs:

```sh
cargo test --no-run --locked --offline \
  --manifest-path "$FIXTURE_STAGE/private-ingress-fixture/Cargo.toml" \
  --test driver \
  --message-format=json
```

Use a trusted fresh, exclusively administered namespace with root and initial
NET_ADMIN/SYS_ADMIN/SETPCAP/SETUID/SETGID. Set
`POKELINUX_CONNECTIVITY_ISOLATED=1`. Mount `controller.py` at
`/runner/controller.py`, existing public `peer.py` at `/runner/peer.py`, `owned.py`
at `/runner/owned.py` and `closure.py` at `/runner/closure.py`. Retain the existing
Scope and exact-container custody. Invoke the controller with explicit selected
absolute paths and the original outer deadlines; output must be fresh:

```sh
python3 -B /runner/controller.py \
  --main-elf "$MAIN_ELF" \
  --worker-elf "$WORKER_ELF" \
  --driver-elf "$DRIVER_ELF" \
  --output "$FRESH_OUTPUT" \
  --work-until-ns "$ORIGINAL_WORK_UNTIL_NS" \
  --close-until-ns "$ORIGINAL_CLOSE_UNTIL_NS"
```

The one six-second case starts before Main; startup/actions retain their original
one-second deadlines. The quiet phase uses one original 800 ms cutoff including
gate delivery; the armed-to-final-EAGAIN span is recorded separately. The peer's
final drain can complete after that cutoff due to scheduling/poll delay. No
cutoff is renewed.

The unchanged fixture passes the current guarded-Main packet case. Ordinary
accept-error QA is a separate component execution. No ordinary app, Settings,
installed-service, ARM or phone permission claim follows.
