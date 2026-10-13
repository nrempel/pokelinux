# Current session, owned display context and normal app

This isolated lane composes the shared native observer/actor, Rust Wayland
security-context supervisor and Messages writer/readonly split with original
Hyprland and the unchanged normal Qt app. It uses seeded local history and no
production radio backend. See [scope and results](../../docs/native-session-display-composition.md)
and [chronological lane evidence](evidence.json).

Prepare an existing mutable Rust target directory on RAM scratch. Finished
selected sources and executables are separately frozen; never reuse a retired
immutable target as mutable cache.

```sh
python3 tools/native-session-display-app/build.py \
  --label new-owned-display-build --cache-path /dev/shm/your-existing-rust-target
python3 tools/native-session-display-app/run.py \
  --label new-held-policy-epoch --build new-owned-display-build \
  --test-case composite-held-policy-stop-green
python3 tools/native-session-display-app/run.py \
  --label new-held-sdk-epoch --build new-owned-display-build \
  --test-case composite-held-sdk-loss-green
```

The VM driver uses Harbour's already prepared original compositor and normal app
selections and one existing production guest backing through shared readonly
input/exclusive output locks. It does not create a new VM image. Each guest runs
four vCPUs inside a six-CPU/eight-GiB networkless container. Build uses eight CPUs,
12GiB, pinned offline dependencies and a readonly Cargo cache.

Direct-realm and poll-only outside-actor resource reds require their historical
selected binaries; a newer ownership build cannot substitute for either original
red. Preflight tracing has a separate driver and evidence selection. All normal
frames require independent visual inspection. Passing these tools is a bounded
integration checkpoint, with service-epoch display metadata and the production
and hardware gates named in the report.
