# Native cellular service

Three safe-Rust executables compose one conservative Data attempt and private
operator diagnostics. The first generic ARM production Main/worker/status tracer
passes one setup/full report/controlled kernel join/real link loss/native wait/
journal refusal path with34 guest waits/EOF. Final host checks pass24 service and
nine shared tests. See the [report](../docs/native-cellular-service.md) and
[provenance](../docs/evidence/native-cellular-service-provenance.json) for the
separate component REDs, actual trace and remaining gates.

- `cellular-service --runtime ABS --journal ABS --worker ABS --config ABS --until-ns NS`
  owns one main-thread cooperative worker, bounded frame admission, a synced
  persistent attempt fence and same-account Status/Stop socket.
- `cellular-worker --pokelinux-process-worker --config ABS --until-ns NS --config-sha256 HEX`
  is launched by that owner only. Shared retained-pidfd parent bootstrap and
  same protected-content validation precede Binder startup. It performs one
  ordinary configured DataV3 setup; potential blocking stays in this process.
- `cellular-status --runtime ABS --until-ns NS [--stop]` sends one operator
  request. Its one absolute future cutoff is at most60s; stdout must be a
  nonblocking pipe. Stop waits exact server pidfd exit, which is not a parent
  wait or an exit-code/remote-cleanup receipt.

Existing same-service-effective-UID0700 runtime/state/config parents and a
regular single-link0600 configuration are provisioning premises. Runtime socket
paths are not unlinked/reused automatically. The configured device, Data name,
expected callback UID and all10 setup arguments do not grant APN/SIM/app
permission or establish slot mapping. Config is bounded32768 bytes; its immediate
parent/leaf final-component NOFOLLOW and metadata checks do not contain every
ancestor. Config duplicate keys are not separately refused. The byte SHA fences
Main/worker content identity; it does not authenticate a provider or inode.

Status keeps local phase apart from source revision/time, immutable historical
provider disposition/report and optional current association. NotSubmitted is
last-observed, not proof that nothing reached the provider; the admission journal
already fences an uncertain attempt. Complete Pending pipe delivery is required
before possible setup, but parent admission is a separate observation.
Accepted report does not establish an APN or provider-owned link. Main withdraws
associations aged1s without claiming link absence; local revocation retains
history. Same-journal refusal78 is permanent here: no reset/retry/deactivate or
vendor-cleanup contract is supplied.

Typed JSON preserves all18 result fields in the finite owner subset: Noinit QoS,
empty QoS sessions, bounded vectors/text/descriptor bytes and explicit nullable
fields. Worker lines and diagnostic packets/output are capped at4096 encoded
bytes including LF/escapes; unsupported shapes and excess are refused. Direct
typed worker decoding rejects duplicate/unknown/missing fields. Only
Kernel(ConcurrentChange) permits observation polling to continue for the same
retained attempt, without another attach/setup or renewed deadline.

Stop revokes before signal and retains the child to actual wait under one2s
cutoff; occupied expiry aborts rather than detaching. Worker fatal exits bypass
potentially blocking Binder destructors. Trusted executable/procfs, deployed
service-manager fallback/current authorization and resource policy remain
external. The tracer is guest-owned direct Main execution, not installed
systemd policy, Binder-pool joining, APN/carrier readiness or phone proof.

From the repository root (do not interpret compilation as runtime evidence):

```sh
cargo fmt --manifest-path native-cellular-service/Cargo.toml -- --check
cargo clippy --locked --offline --manifest-path native-cellular-service/Cargo.toml --all-targets -- -D warnings
cargo test --locked --offline --manifest-path native-cellular-service/Cargo.toml --lib
cargo build --locked --offline --manifest-path native-cellular-service/Cargo.toml --release --target aarch64-unknown-linux-gnu
```

The selected production dependency does not enable the authored fixture-control
AIDL. The independent controller/NDK peer and privileged namespace/TUN setup are
test infrastructure, never an assume-ready production switch. No physical phone
or original/vendor code is executed by these host component commands.
